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
use core::slice::bytes;
use core::mem;

use alloc::boxed::Box;
use alloc::rc::Rc;
use collections::{Vec, BTreeMap, String};

use error;

use paging::{self, Pageset, PagesetExt, RcPageset, PageType};
use paging::generic::Pageset as GenericPageset;
use memory;

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
        next_id: 1,
    })));

    //syscall::initialize();

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

        let process = Process {
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

                bytes::copy_memory(arg, arg_dest);

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
                try!(memory::acquire_region(pages - mapped)
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
                        let r_pages = (paddr_end - paddr_start)/page_size;

                        memory::release_region(paddr_start, r_pages);

                        paddr_range = Some((paddr, paddr));
                    }
                } else {
                    paddr_range = Some((paddr, paddr));
                }
            }

            None
        }).map_err(|e| Error::from(e)));

        if let Some((paddr_start, paddr_end)) = paddr_range {
            let r_pages = (paddr_end - paddr_start)/page_size;

            memory::release_region(paddr_start, r_pages);
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

        let old_heap_pages = self.heap_length / page_size;
        let new_heap_pages = new_heap_length  / page_size;

        if new_heap_pages > old_heap_pages {
            let base = self.heap_base + old_heap_pages * page_size;
            let size = (new_heap_pages - old_heap_pages) * page_size;

            try!(self.map_allocate(base, size, PageType::default().writable()));
        } else if new_heap_pages < old_heap_pages {
            let base = self.heap_base + new_heap_pages * page_size;
            let size = (old_heap_pages - new_heap_pages) * page_size;

            try!(self.unmap_deallocate(base, size));
        }

        Ok(())
    }
}

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
    use core::cell::*;
    use core::mem;

    use c_ffi::c_void;

    use alloc::rc::Rc;

    use super::*;

    #[repr(C)]
    pub struct ProcessCRef(*const c_void);

    impl ProcessCRef {
        pub fn new(rc_process: Rc<RefCell<Process>>) -> ProcessCRef {
            unsafe {
                mem::transmute(rc_process)
            }
        }

        pub fn to_rc(&self) -> Rc<RefCell<Process>> {
            if self.is_null() {
                panic!("Tried to call into_rc() on null ProcessCRef");
            }

            unsafe {
                let ProcessCRef(ptr) = *self;
                let rc1: Rc<RefCell<Process>> = mem::transmute(ptr);
                let rc2 = rc1.clone();

                mem::forget(rc1);
                rc2
            }
        }

        pub fn to_option(&self) -> Option<Rc<RefCell<Process>>> {
            if self.is_null() {
                None
            } else {
                Some(self.to_rc())
            }
        }

        pub fn into_rc(self) -> Rc<RefCell<Process>> {
            if self.is_null() {
                panic!("Tried to call into_rc() on null ProcessCRef");
            }

            unsafe {
                mem::transmute(self)
            }
        }

        pub fn is_null(&self) -> bool {
            let ProcessCRef(ptr) = *self;

            ptr.is_null()
        }
    }
}
