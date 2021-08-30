/*******************************************************************************
 *
 * kit/kernel/process/mod.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Process management functions.

use core::{i32, u32, usize};
use core::cell::*;
use core::fmt;
use core::slice;
use core::mem;

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;

use displaydoc::Display;

use crate::error;

use crate::paging::{self, Pageset, PagesetExt, RcPageset, PageType};
use crate::paging::generic::Pageset as GenericPageset;
use crate::memory::{self, RegionUser};
use crate::scheduler;
use crate::syscall;
use crate::util::copy_memory;
use crate::sync::WaitQueue;

pub mod x86_64;
pub use self::x86_64 as target;

pub type Id = u32;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub enum State {
    Loading,
    Running,
    Sleeping,
    Dead,
}

impl State {
    pub fn short_description(&self) -> &'static str {
        match *self {
            State::Loading  => "Load",
            State::Running  => "Run",
            State::Sleeping => "Slp",
            State::Dead     => "Dead",
        }
    }
}

struct GlobalState {
    kernel_process: RcProcess,
    current_process: RcProcess,
    process_tree: BTreeMap<Id, RcProcess>,
    next_id: Id,
}

static mut GLOBAL_STATE: Option<*const RefCell<GlobalState>> = None;
static mut INITIALIZED: bool = false;

pub fn initialized() -> bool {
    unsafe { INITIALIZED }
}

pub unsafe fn initialize() {
    if INITIALIZED {
        panic!("process module already initialized");
    }

    // The kernel process does not have its own memory space, so when switching
    // between kernel processes 
    let kernel_process = Rc::new(RefCell::new(Process {
        id:          0, // only process that can have ID 0
        pgid:        0, // all kernel subprocesses share this value too
        name:        Rc::new("kernel".into()),
        state:       State::Running,
        hw_state:    Box::into_raw(box target::HwState::new()),
        mem:         None,
        exit_status: 0,
        exit_wait:   WaitQueue::new(),
    }));

    let current_process = kernel_process.clone();

    let mut process_tree = BTreeMap::new();

    process_tree.insert(kernel_process.borrow().id, kernel_process.clone());

    GLOBAL_STATE = Some(Box::into_raw(box RefCell::new(GlobalState {
        kernel_process,
        current_process,
        process_tree,
        next_id: 1,
    })));

    scheduler::initialize();
    syscall::initialize();

    INITIALIZED = true;
}

fn global_state<'a>() -> &'a RefCell<GlobalState> {
    unsafe {
        GLOBAL_STATE.as_ref().and_then(|ptr| ptr.as_ref())
            .expect("Process module not initialized!")
    }
}

/// Get the kernel process (ID=0).
pub fn kernel() -> RcProcess {
    global_state().borrow().kernel_process.clone()
}

/// Get the current process.
pub fn current() -> RcProcess {
    global_state().borrow().current_process.clone()
}

/// Get a process by ID.
pub fn by_id(id: Id) -> Option<RcProcess> {
    global_state().borrow().process_tree.get(&id).map(|r| r.clone())
}

/// Get the processes sharing a PGID.
pub fn by_pgid(pgid: Id) -> Vec<RcProcess> {
    global_state().borrow().process_tree.values()
        .filter(|proc| proc.borrow().pgid == pgid)
        .map(|proc| proc.clone())
        .collect()
}

/// Get all processes.
pub fn all() -> Vec<RcProcess> {
    global_state().borrow().process_tree.values().cloned().collect()
}

/// Change the current process (immediately).
///
/// # Panics
///
/// Panics if the process to switch to is not in the `Running` state.
pub fn switch_to(process: RcProcess) {
    assert!(process.borrow().is_running());

    let old_process = current();

    let old_hw_state = old_process.borrow().hw_state;
    let new_hw_state = process.borrow().hw_state;

    // Don't switch pageset for processes that don't have a memory space.
    //
    // This allows kernel processes to have lighter context switching - the
    // kernel pageset is always accessible anyway.
    if let Some(pageset) = process.borrow().pageset() {
        // Safety: we got the pageset from another process, so it shouldn't be
        // anything weird
        unsafe {
            paging::set_current_pageset(Some(pageset));
        }
    }

    global_state().borrow_mut().current_process = process;

    // Do the magic!
    unsafe {
        process_hw_switch(old_hw_state, new_hw_state);
    }
}

extern {
    fn process_hw_switch(old: *mut target::HwState, new: *mut target::HwState);
    fn process_hw_enter_user();
    fn process_hw_enter_kernel();
}

fn new_user_hw_state() -> Box<target::HwState> {
    let mut hw_state = target::HwState::new();

    unsafe {
        // Use the user setup routine to jump to user code on switch
        hw_state.kernel_mut()
            .set_instruction_pointer(process_hw_enter_user as usize);

        // Set the stack pointer for the user code.
        hw_state.user_mut()
            .set_stack_pointer(target::STACK_BASE_ADDR);
    }

    Box::new(hw_state)
}

pub type RcProcess = Rc<RefCell<Process>>;

#[derive(Debug)]
pub struct Process {
    id:          Id,

    /// Process group: subprocesses spawned from the same process will have the
    /// same PGID. Exit() will cause all processes with that PGID to exit.
    pgid:        Id,
    name:        Rc<String>,
    state:       State,
    hw_state:    *mut target::HwState,

    /// Information about the memory space of the process.
    ///
    /// Can be shared between processes.
    mem:         Option<RcProcessMem>,

    exit_status: i32,

    /// Wait queue for exit event.
    exit_wait:   WaitQueue,
}

impl Process {
    fn next_id() -> Id {
        let mut global_state = global_state().borrow_mut();

        let next_id = global_state.next_id;
        assert!(next_id != u32::MAX, "out of process IDs");

        global_state.next_id += 1;
        next_id
    }

    pub fn create<S>(name: S) -> RcProcess where S: Into<String> {
        let id = Process::next_id();

        let mut process_mem = ProcessMem {
            id:          id,
            pageset:     Pageset::alloc(),
            heap_base:   target::HEAP_BASE_ADDR,
            heap_length: 0,
        };

        // FIXME? This assumes a downward growing stack, like x86
        process_mem.map_allocate(
            target::STACK_BASE_ADDR - target::STACK_SIZE,
            target::STACK_SIZE,
            PageType::default().writable()).unwrap();

        let process = Process {
            id:          id,
            pgid:        id,
            name:        Rc::new(name.into()),
            state:       State::Loading,
            hw_state:    Box::into_raw(new_user_hw_state()),
            mem:         Some(Rc::new(RefCell::new(process_mem))),
            exit_status: 0,
            exit_wait:   WaitQueue::new(),
        };

        let rc_process = Rc::new(RefCell::new(process));

        global_state().borrow_mut().process_tree.insert(id, rc_process.clone());

        rc_process
    }

    /// Creates a process sharing the same memory space and pgid.
    pub fn create_subprocess(&self) -> RcProcess {
        let id = Process::next_id();

        assert!(!self.is_dead());

        let process = Process {
            id: id,
            pgid: self.pgid,
            name: self.name.clone(),
            state: State::Loading,
            hw_state: Box::into_raw(new_user_hw_state()),
            mem: self.mem.clone(),
            exit_status: 0,
            exit_wait: WaitQueue::new(),
        };

        let rc_process = Rc::new(RefCell::new(process));

        global_state().borrow_mut().process_tree.insert(id, rc_process.clone());

        rc_process
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn pgid(&self) -> Id {
        self.pgid
    }

    pub fn name(&self) -> &str {
        &*self.name
    }

    pub fn set_name<S>(&mut self, name: S) where S: Into<String> {
        self.name = Rc::new(name.into());
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn mem(&self) -> Option<RcProcessMem> {
        self.mem.as_ref().map(|rc| rc.clone())
    }

    pub fn pageset(&self) -> Option<RcPageset> {
        self.mem.as_ref().map(|rc| rc.borrow().pageset.clone())
    }

    pub fn is_running(&self) -> bool {
        self.state == State::Running
    }

    pub fn is_sleeping(&self) -> bool {
        self.state == State::Sleeping
    }

    /// True if the process is `Running` or `Sleeping`.
    pub fn is_alive(&self) -> bool {
        self.is_running() || self.is_sleeping()
    }

    pub fn is_loading(&self) -> bool {
        self.state == State::Loading
    }

    pub fn is_dead(&self) -> bool {
        self.state == State::Dead
    }

    /// Returns the exit status of the process if it has exited.
    pub fn exit_status(&self) -> Option<i32> {
        if self.state == State::Dead {
            Some(self.exit_status)
        } else {
            None
        }
    }

    /// Read the process's hardware state, which usually includes registers and
    /// other architecture-dependent properties.
    ///
    /// # Unsafety
    ///
    /// This method is incapable of guaranteeing the typical safety guarantees
    /// of Rust, since various low-level architecture-specific routines may be
    /// modifying it at the same time.
    pub unsafe fn hw_state(&self) -> &target::HwState {
        assert!(!self.hw_state.is_null());
        self.hw_state.as_ref().unwrap()
    }

    /// Modify the process's hardware state, which usually includes registers
    /// and other architecture-dependent properties.
    ///
    /// Modifying a process's state while it is running could cause data
    /// corruption and unwanted behavior, so be careful.
    ///
    /// # Unsafety
    ///
    /// This method is incapable of guaranteeing the typical safety guarantees
    /// of Rust, since various low-level architecture-specific routines may be
    /// modifying it at the same time.
    pub unsafe fn hw_state_mut(&mut self) -> &mut target::HwState {
        assert!(!self.hw_state.is_null());
        self.hw_state.as_mut().unwrap()
    }

    pub fn load<T: Image>(&mut self, image: &T) -> Result<(), Error> {
        image.load_into(self)
    }

    /// Setup the process to start executing an arbitrary kernel function.
    ///
    /// The function will receive exactly one argument, which can be passed
    /// here.
    ///
    /// If creating kernel threads to run Rust code, see
    /// [process::spawn_kthread].
    ///
    /// # Unsafety
    ///
    /// An arbitrary C function may be executed by scheduling this process after
    /// calling this, which may have unsafe behavior.
    pub unsafe fn load_kernel_fn(
        &mut self,
        ptr: unsafe extern "C" fn (usize) -> i32,
        arg: usize
    ) {
        assert_eq!(self.state, State::Loading);

        // Safety: hwstate exclusive ownership due to Loading state
        //
        // Contract as defined by the unsafety of this function, for
        // set_instruction_pointer and set_argument
        let kstate = self.hw_state_mut().kernel_mut();

        kstate.set_instruction_pointer(process_hw_enter_kernel as usize);
        kstate.push_stack(arg);
        kstate.push_stack(ptr);
    }

    pub fn set_args(&mut self, args: &[&[u8]]) -> Result<(), Error> {
        assert_eq!(self.state, State::Loading);

        if let Some(ref mem) = self.mem {
            let params = mem.borrow_mut().setup_args(args)?;

            // Safety: hwstate exclusive ownership due to Loading state
            unsafe {
                self.hw_state_mut().user_mut().set_args(params);
            }

            Ok(())
        } else {
            panic!("Can't set_args because process doesn't have its own \
                memory space");
        }
    }

    pub fn set_entry_point(&mut self, vaddr: usize) {
        assert_eq!(self.state, State::Loading);

        // Safety: hwstate exclusive ownership due to Loading state
        unsafe {
            self.hw_state_mut().user_mut().set_instruction_pointer(vaddr);
        }
    }

    /// Set the process's state to `Running` if it was previously `Loading`.
    /// This prevents further modifications intended for initialization from
    /// occurring.
    ///
    /// Call this before handing the process off to the scheduler.
    ///
    /// # Panics
    ///
    /// Panics if the current state is *not* `Loading`, because that can be
    /// indicate the presence of a bug. If you expect that the process might not
    /// be `Loading`, you should check its state first.
    pub fn run(&mut self) {
        assert_eq!(self.state, State::Loading,
                "Tried to run a non-Loading process");

        self.state = State::Running;
    }

    /// Set the process's state to `Sleeping`.
    ///
    /// # Panics
    ///
    /// Panics if the current state is neither `Running` nor `Sleeping`.
    pub fn sleep(&mut self) {
        if !self.is_alive() {
            panic!("Tried to put a {:?} process to sleep", self.state);
        }

        self.state = State::Sleeping;
    }

    /// Set the process's state to `Running` if it was `Sleeping`.
    ///
    /// Make sure to call this before pushing a process onto the scheduler.
    ///
    /// # Panics
    ///
    /// Panics if the current state is neither `Running` nor `Sleeping`.
    pub fn awaken(&mut self) {
        if !self.is_alive() {
            panic!("Tried to wake a {:?} process", self.state);
        }

        self.state = State::Running;
    }

    /// Set the process's state to `Dead` and set its exit status to the given
    /// value.
    pub fn exit(&mut self, exit_status: i32) {
        self.state = State::Dead;
        self.exit_status = exit_status;
    }
}

impl PartialEq for Process {
    fn eq(&self, other: &Process) -> bool {
        self.id == other.id
    }
}

impl Eq for Process { }

impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            drop(Box::from_raw(self.hw_state))
        }
    }
}

pub type RcProcessMem = Rc<RefCell<ProcessMem>>;

#[derive(Debug)]
pub struct ProcessMem {
    id:          Id,
    pageset:     RcPageset,
    heap_base:   usize,
    heap_length: usize,
}

impl ProcessMem {

    pub fn heap_base(&self) -> usize {
        self.heap_base
    }

    pub fn heap_length(&self) -> usize {
        self.heap_length
    }

    pub fn heap_end(&self) -> usize {
        let page_size = <Pageset as GenericPageset>::page_size();

        let mut heap_end = self.heap_base + self.heap_length;

        if heap_end % page_size != 0 {
            heap_end += page_size - (heap_end % page_size);
        }

        heap_end
    }

    pub fn pageset(&self) -> RcPageset {
        self.pageset.clone()
    }

    /// Allocates at least enough pages at `vaddr` to contain `size`.
    pub fn map_allocate(&mut self,
                        vaddr: usize,
                        size: usize,
                        page_type: PageType)
                        -> Result<(), Error> {
        let page_size = <Pageset as GenericPageset>::page_size();

        let pages =
            if size % page_size != 0 {
                size / page_size + 1
            } else {
                size / page_size
            };

        let mut mapped = 0;

        let mut pageset = self.pageset.borrow_mut();

        while mapped < pages {
            let (paddr_start, acq_pages) =
                memory::acquire_region(RegionUser::Process(self.id),
                                            pages - mapped)
                     .ok_or(Error::OutOfMemory(mapped))?;

            let paddr_end = paddr_start + acq_pages * page_size;

            pageset.map_pages_with_type(
                    vaddr,
                    (paddr_start..paddr_end).step_by(page_size),
                    page_type.user())
                 .map_err(|e| Error::from(e))?;

            mapped += acq_pages;
        }

        Ok(())
    }

    pub fn unmap_deallocate(&mut self, vaddr: usize, size: usize)
                            -> Result<(), Error> {
        let page_size = <Pageset as GenericPageset>::page_size();

        let pages =
            if size % page_size != 0 {
                size / page_size + 1
            } else {
                size / page_size
            };

        let mut pageset = self.pageset.borrow_mut();

        // Release contiguous physical regions.
        let mut paddr_range = None;

        pageset.modify_pages(vaddr, pages, |page| {
            if let Some((paddr, _)) = page {
                if let Some((paddr_start, paddr_end)) = paddr_range.take() {
                    if paddr_end == paddr - page_size {
                        paddr_range = Some((paddr_start, paddr));
                    } else {
                        memory::release_region(RegionUser::Process(self.id),
                                               paddr_start);

                        paddr_range = Some((paddr, paddr));
                    }
                } else {
                    paddr_range = Some((paddr, paddr));
                }
            }

            None
        })?;

        if let Some((paddr_start, _paddr_end)) = paddr_range {
            memory::release_region(RegionUser::Process(self.id), paddr_start);
        }

        Ok(())
    }

    /// Adjusts the process's heap by the requested amount.
    pub fn adjust_heap(&mut self, amount: isize) -> Result<(), Error> {
        if amount < 0 && self.heap_length < -amount as usize {
            return Err(Error::Overflow);
        }

        if amount > 0 && usize::MAX - self.heap_length < amount as usize {
            return Err(Error::Overflow);
        }

        let new_heap_length = self.heap_length.wrapping_add(amount as usize);

        let page_size = <Pageset as GenericPageset>::page_size();

        fn divup(dividend: usize, divisor: usize) -> usize {
            if dividend % divisor == 0 { dividend / divisor }
            else                       { dividend / divisor + 1 }
        }

        let old_heap_pages = divup(self.heap_length, page_size);
        let new_heap_pages = divup(new_heap_length,  page_size);

        if new_heap_pages > old_heap_pages {
            let base = self.heap_base + old_heap_pages * page_size;
            let size = (new_heap_pages - old_heap_pages) * page_size;

            self.map_allocate(base, size, PageType::default().writable())?;
        } else if new_heap_pages < old_heap_pages {
            let base = self.heap_base + new_heap_pages * page_size;
            let size = (old_heap_pages - new_heap_pages) * page_size;

            self.unmap_deallocate(base, size)?;
        }

        self.heap_length = new_heap_length;

        Ok(())
    }

    /// Sets up args in the memory space, and returns the parameters that should
    /// be passed to the entry point.
    ///
    /// See [HwState::set_args]
    pub fn setup_args(&mut self, args: &[&[u8]])
        -> Result<Option<(i32, usize)>, Error> {

        // Special case: if there are no args, just set HwState.
        if args.is_empty() {
            return Ok(None);
        }

        // Args length must fit within an i32.
        assert!(args.len() <= i32::MAX as usize);

        // Size of all args + size of null bytes at end of each arg + size of
        // pointer table
        let args_size =
            args.iter().map(|a| a.len()).sum::<usize>() + args.len() +
            mem::size_of::<usize>() * args.len();

        let page_size = <Pageset as GenericPageset>::page_size();

        let vaddr = target::ARGS_TOP_ADDR - args_size;
        let vaddr = vaddr - vaddr % page_size;

        self.map_allocate(vaddr, args_size, PageType::default())?;

        unsafe {
            // Swap in process pageset.
            // Careful: must reset to old pageset after!
            let old_pageset = paging::current_pageset();
            paging::set_current_pageset(Some(self.pageset()));

            // Set pointer table and copy args.
            let ptr_table: &mut [usize] =
                slice::from_raw_parts_mut(vaddr as *mut usize, args.len());

            let mut next_ptr =
                vaddr + ptr_table.len() * mem::size_of::<usize>();

            for (index, arg) in args.iter().enumerate() {
                let arg_dest: &mut [u8] =
                    slice::from_raw_parts_mut(next_ptr as *mut u8,
                                              arg.len() + 1);

                ptr_table[index] = next_ptr;

                copy_memory(arg, arg_dest);

                arg_dest[arg.len()] = 0;

                next_ptr += arg.len() + 1;
            }

            // Reset to old pageset.
            paging::set_current_pageset(old_pageset);
        }

        // Return parameters that should be put in HwState
        Ok(Some((args.len() as i32, vaddr)))
    }
}

#[derive(PartialEq, Eq, Debug, Display)]
pub enum Error {
    /// An error occurred while trying to modify pages: {0}
    PagingError(paging::Error),
    /// Ran out of free physical regions to allocate pages with ({0} alloc'd)
    OutOfMemory(usize),
    /// An integer overflow occurred (parameter too big/small?)
    Overflow,
    /// Unknown process id {0}
    UnknownPid(Id),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::PagingError(ref paging_error) => Some(paging_error),
            _ => None
        }
    }
}

impl From<paging::Error> for Error {
    fn from(paging_error: paging::Error) -> Error {
        Error::PagingError(paging_error)
    }
}

pub trait Image {
    fn load_into(&self, process: &mut Process) -> Result<(), Error>;
}

/// Exit the current process.
pub fn exit(status: i32) -> ! {
    {
        let rc_process = current();

        let mut process = rc_process.borrow_mut();

        assert!(process.id != 0, "attempted to exit({}) kernel!", status);

        process.exit(status);

        // Notify wait queue
        process.exit_wait.awaken_all();
    }

    scheduler::r#yield();

    panic!("returned to process {} after exit", current().borrow().id);
}

/// Sleep until the given process id wakes up.
pub fn wait(id: Id) -> Result<(), Error> {
    let queue = by_id(id).ok_or(Error::UnknownPid(id))?
        .borrow().exit_wait.clone();

    wait!(by_id(id).map(|p| p.borrow().is_dead()).unwrap_or(true), [queue]);

    Ok(())
}

/// Set our state to sleep and then yield to the scheduler.
pub fn sleep() {
    current().borrow_mut().sleep();
    scheduler::r#yield();
}

type BoxedFun = Box<dyn FnOnce() + Send + 'static>;
type ThinBoxedFun = Box<BoxedFun>;

/// Spawn a kernel thread using Rust code.
pub fn spawn_kthread<S, F>(name: S, fun: F) -> Id
    where S: Into<String>, F: FnOnce() + Send + 'static {

    let subproc = kernel().borrow().create_subprocess();

    let id;

    {
        let mut subproc = subproc.borrow_mut();

        id = subproc.id();

        subproc.set_name(name);

        unsafe {
            let boxed_fun = Box::into_raw(Box::new(Box::new(fun) as BoxedFun));

            subproc.load_kernel_fn(kthread_entry, boxed_fun as usize);
        }

        subproc.run();
    }

    scheduler::push(subproc);

    id
}

unsafe extern "C" fn kthread_entry(boxed_fun: usize) -> i32 {
    // Get the boxed function
    let fun: ThinBoxedFun = Box::from_raw(boxed_fun as *mut BoxedFun);

    // Call it
    fun();

    // Return code zero
    0
}

/// Dumps a list of processes to the console, for debugging.
pub fn debug_print_processes() {
    use crate::terminal::console;

    let processes = all();

    let _ = writeln!(console(), "ID    PGID  STATE NAME");

    for rc_process in processes {
        let process = rc_process.borrow();

        let _ = writeln!(console(), "{:<5} {:<5} {:<5} {}",
            process.id(),
            process.pgid(),
            process.state().short_description(),
            process.name());
    }
}

/// C interface. See `kit/kernel/include/process.h`.
pub mod ffi {
    use crate::c_ffi::*;

    #[no_mangle]
    pub extern fn process_current_id() -> uint32_t {
        super::current().borrow().id
    }

    #[no_mangle]
    pub unsafe extern fn process_exit(status: c_int) -> ! {
        super::exit(status);
    }

    #[no_mangle]
    pub unsafe extern fn process_signal(pid: uint32_t, signal: c_int) -> c_int {
        if let Some(process) = super::by_id(pid) {
            // Case 1: this is the current process, so just exit
            if super::current().borrow().id == process.borrow().id {
                super::exit(signal);
                // the function will not return!
            }

            // Case 2: we're telling another process to exit.
            process.borrow_mut().exit(signal);

            // Let waiting processes know
            process.borrow().exit_wait.awaken_all();

            1
        } else {
            // Case 3: we can't find the process.
            0
        }
    }

    #[no_mangle]
    pub unsafe extern fn process_wait_exit_status(pid: uint32_t,
                                                  status: *mut c_int)
                                                  -> c_int {
        if super::wait(pid).is_ok() {
            if let Some(rc_process) = super::by_id(pid) {
                let exit_status = rc_process.borrow().exit_status()
                    .expect("wait() returned but process is not dead");

                *status = exit_status;
                0
            } else {
                -1
            }
        } else {
            -1
        }
    }

    //void *process_adjust_heap(int64_t amount);
    #[no_mangle]
    pub unsafe extern fn process_adjust_heap(amount: int64_t) -> *mut c_void {
        let current_process = super::current();

        let rc_mem = current_process.borrow_mut().mem()
            .expect("Current process has no memory associated with it");

        let mut mem = rc_mem.borrow_mut();

        mem.adjust_heap(amount as isize).unwrap();

        let heap_end = mem.heap_end() as *mut c_void;

        heap_end
    }
}
