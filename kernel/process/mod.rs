/*******************************************************************************
 *
 * kit/kernel/process/mod.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
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
use collections::{Vec, BTreeMap, String};

use error;

use paging::{self, Pageset, PagesetExt, RcPageset, PageType};
use paging::generic::Pageset as GenericPageset;
use memory::{self, RegionUser};
use scheduler;
use syscall;
use util::copy_memory;

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

struct GlobalState {
    process_tree: BTreeMap<Id, RcProcess>,
    current_process: Option<RcProcess>,
    noproc_hw_state: *mut target::HwState,
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

    GLOBAL_STATE = Some(Box::into_raw(box RefCell::new(GlobalState {
        process_tree: BTreeMap::new(),
        current_process: None,
        noproc_hw_state: Box::into_raw(box target::HwState::new()),
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

/// Get the current process, if applicable.
///
/// During initialization, this may be None.
pub fn current() -> Option<RcProcess> {
    global_state().borrow_mut().current_process.clone()
}

/// Get a process by ID.
pub fn by_id(id: Id) -> Option<RcProcess> {
    global_state().borrow().process_tree.get(&id).map(|r| r.clone())
}

/// Change the current process (immediately).
///
/// # Unsafety
///
/// Unsafe because the entire call stack must be reentrant. Execution of another
/// process could allow a lot of things to change before this function returns.
///
/// # Panics
///
/// Panics if the process to switch to is not in the `Running` state.
pub unsafe fn switch_to(process: RcProcess) {
    assert!(process.borrow().is_running());

    let old_process = current();

    let old_hw_state = match old_process {
        Some(old_process) => old_process.borrow().hw_state,
        None              => global_state().borrow().noproc_hw_state
    };

    let new_hw_state = process.borrow().hw_state;

    paging::set_current_pageset(Some(process.borrow().pageset()));

    global_state().borrow_mut().current_process = Some(process);

    // Do the magic!
    process_hw_switch(old_hw_state, new_hw_state);
}

// Use scheduler::exit() instead
#[doc(hidden)]
pub unsafe fn switch_to_noproc() {
    let old_process = match current() {
        Some(old_process) => old_process,
        None              => return
    };

    let old_hw_state = old_process.borrow().hw_state;
    let new_hw_state = global_state().borrow().noproc_hw_state;

    paging::set_current_pageset(None);

    global_state().borrow_mut().current_process = None;

    // Do the magic!
    process_hw_switch(old_hw_state, new_hw_state);
}

extern {
    fn process_hw_switch(old: *mut target::HwState, new: *mut target::HwState);
}

pub type RcProcess = Rc<RefCell<Process>>;

pub struct Process {
    id:          Id,
    name:        String,
    state:       State,
    pageset:     RcPageset,
    hw_state:    *mut target::HwState,
    heap_base:   usize,
    heap_length: usize,
    exit_status: i32,

    /// List of processes waiting for us to exit. Not ideal, but will likely be
    /// replaced by the host model.
    pub waiting: Vec<Id>,
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

        let mut process = Process {
            id:          id,
            name:        name.into(),
            state:       State::Loading,
            pageset:     Pageset::alloc(),
            hw_state:    Box::into_raw(box target::HwState::new()),
            heap_base:   target::HEAP_BASE_ADDR,
            heap_length: 0,
            exit_status: 0,
            waiting:     vec![],
        };

        // FIXME? This assumes a downward growing stack, like x86
        process.map_allocate(
            target::STACK_BASE_ADDR - target::STACK_SIZE,
            target::STACK_SIZE,
            PageType::default().writable()).unwrap();

        let rc_process = Rc::new(RefCell::new(process));

        global_state().borrow_mut().process_tree.insert(id, rc_process.clone());

        rc_process
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn name(&self) -> &str {
        &*self.name
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn pageset(&self) -> RcPageset {
        self.pageset.clone()
    }

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

    pub fn set_args(&mut self, args: &[&[u8]]) -> Result<(), Error> {
        assert!(self.state == State::Loading);

        // Special case: if there are no args, just set HwState.
        if args.is_empty() {
            unsafe {
                self.hw_state_mut().set_args(None);
            }
            return Ok(());
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

        try!(self.map_allocate(vaddr, args_size, PageType::default()));

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

            // Put args in HwState in preparation to run the program.
            self.hw_state_mut().set_args(Some((args.len() as i32, vaddr)));
        }

        Ok(())
    }

    pub fn set_entry_point(&mut self, vaddr: usize) {
        assert!(self.state == State::Loading);

        unsafe {
            self.hw_state_mut().set_instruction_pointer(vaddr);
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
        assert!(self.state == State::Loading,
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
                try!(memory::acquire_region(RegionUser::Process(self.id),
                                            pages - mapped)
                     .ok_or(Error::OutOfMemory(mapped)));

            let paddr_end = paddr_start + acq_pages * page_size;

            try!(pageset.map_pages_with_type(
                    vaddr,
                    (paddr_start..paddr_end).step_by(page_size),
                    page_type.user())
                 .map_err(|e| Error::from(e)));

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

        try!(pageset.modify_pages(vaddr, pages, |page| {
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
        }).map_err(|e| Error::from(e)));

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

            try!(self.map_allocate(base, size, PageType::default().writable()));
        } else if new_heap_pages < old_heap_pages {
            let base = self.heap_base + new_heap_pages * page_size;
            let size = (old_heap_pages - new_heap_pages) * page_size;

            try!(self.unmap_deallocate(base, size));
        }

        self.heap_length = new_heap_length;

        Ok(())
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
            Box::from_raw(self.hw_state).deallocate()
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Error {
    PagingError(paging::Error),
    OutOfMemory(usize),
    Overflow,
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::PagingError(_) =>
                "An error occurred while trying to modify pages",

            Error::OutOfMemory(_) =>
                "Ran out of free physical regions to allocate pages with",

            Error::Overflow =>
                "An integer overflow occurred (parameter too big/small?)",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::PagingError(ref paging_error) => Some(paging_error),
            _ => None
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(f.write_str(error::Error::description(self)));

        if let Some(cause) = error::Error::cause(self) {
            try!(write!(f, ": {}", cause));
        }

        Ok(())
    }
}

impl From<paging::Error> for Error {
    fn from(paging_error: paging::Error) -> Error {
        Error::PagingError(paging_error)
    }
}

pub trait Image {
    fn load_into(&self, &mut Process) -> Result<(), Error>;
}

/// C interface. See `kit/kernel/include/process.h`.
pub mod ffi {
    use c_ffi::*;
    use scheduler;

    #[no_mangle]
    pub extern fn process_current_id() -> uint32_t {
        if let Some(process) = super::current() {
            process.borrow().id
        } else {
            0
        }
    }

    #[no_mangle]
    pub unsafe extern fn process_exit(status: c_int) -> ! {
        if let Some(process) = super::current() {
            process.borrow_mut().exit(status);

            if process.borrow().id == 1 {
                panic!("initial process ({}, {}) exited with status {}",
                       process.borrow().id,
                       process.borrow().name,
                       status);
            }

            // Let waiting processes know
            for &pid in &process.borrow().waiting {
                if let Some(process_waiting) = super::by_id(pid) {
                    let _ = scheduler::awaken(process_waiting);
                }
            }

            scheduler::tick();
            unreachable!();
        } else {
            panic!("C called process_exit() but there is no current process");
        }
    }

    #[no_mangle]
    pub unsafe extern fn process_signal(pid: uint32_t, signal: c_int) -> c_int {
        if let Some(process) = super::by_id(pid) {
            // Case 1: this is the current process, so just exit
            if let Some(current_process) = super::current() {
                if current_process.borrow().id == process.borrow().id {
                    process_exit(signal);
                    // the function will not return!
                }
            }

            // Case 2: we're telling another process to exit.
            process.borrow_mut().exit(signal);

            // Let waiting processes know
            for &pid in &process.borrow().waiting {
                if let Some(process_waiting) = super::by_id(pid) {
                    let _ = scheduler::awaken(process_waiting);
                }
            }

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
        let current_process = super::current()
            .expect("C called process_wait_exit_status() but there is
                     no current process");

        if let Some(process) = super::by_id(pid) {
            let exit_status = process.borrow().exit_status();

            if let Some(exit_status) = exit_status {
                // TODO: remove process from wait list

                *status = exit_status;
                return 0;
            } else {
                process.borrow_mut().waiting
                    .push(current_process.borrow().id());

                current_process.borrow_mut().sleep();
                scheduler::tick();

                return process_wait_exit_status(pid, status);
            }
        } else {
            return -1;
        }
    }

    //void *process_adjust_heap(int64_t amount);
    #[no_mangle]
    pub unsafe extern fn process_adjust_heap(amount: int64_t) -> *mut c_void {
        let current_process = super::current()
            .expect("C called process_adjust_heap() but there is
                     no current process");

        current_process.borrow_mut().adjust_heap(amount as isize).unwrap();

        let heap_end = current_process.borrow().heap_end() as *mut c_void;

        heap_end
    }
}
