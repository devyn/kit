/*******************************************************************************
 *
 * kit/kernel/memory/large_heap.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use crate::sync::{Spinlock, LockFreeList};
use crate::sync::lock_free_list::Node;
use crate::paging::PAGE_SIZE;
use crate::util::{align_up, align_down};

use alloc::sync::Arc;
use alloc::vec::Vec;

use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::*;
use core::cmp;

use super::{VirtualAddress, PageCount, FreeRegion};
use super::KSTACK_SIZE;
use super::{kernel_acquire_and_map, kernel_unmap_and_release};
use super::AcquiredMappedRegion;
use super::pool::Pool;
use super::region_math;

pub const LARGE_HEAP_START: usize = 0xffff_ffff_9000_0000;
pub const LARGE_HEAP_LENGTH: usize = 0x20000; // pages
pub const STACKS_START: usize = 0xffff_ffff_f000_0000;

const BOOTSTRAP_HEAP_PAGES: usize = 24;
const BOOTSTRAP_HEAP_POOLS: usize = 16;

const_assert!(BOOTSTRAP_HEAP_POOLS < BOOTSTRAP_HEAP_PAGES);

const MIN_UNUSED_ALLOCATED: usize = 16; // acquire if under
const MAX_UNUSED_ALLOCATED: usize = 64; // release if over
const UNUSED_ALLOCATED_ACQUIRE_SIZE: usize = 16;

/// Ensure there are regions at least this size (in pages). This should be
/// determined based on the size of allocator and paging data structures
/// required to allocate more memory.
const REQUIRED_MIN_REGION_LENGTH: usize = 2;

type RcPool = Arc<Pool>;

#[derive(Debug)]
pub struct HeapState {
    // Total dimensions of the virtual address space for the heap
    start: VirtualAddress,
    end: VirtualAddress,
    length: PageCount,

    /// For allocating smaller than page size objects - one pool for each object
    /// size
    ///
    /// Using a sorted Vec that's always at least a page big in order to avoid
    /// lock issues
    pools: Spinlock<Vec<(usize, RcPool)>>,

    // For allocating stacks
    stacks_start: *mut u8,
    stacks_end: Spinlock<*mut u8>,

    /// Tracked memory regions
    regions: HeapRegionState,
}

#[derive(Debug)]
struct HeapRegionState {
    /// Physical memory allocated to the heap
    alloc_physical: LockFreeList<AcquiredMappedRegion>,

    /// Free virtual space in the heap area
    free_virtual: LockFreeList<FreeRegion<VirtualAddress>>,

    /// Pre-allocated pages for the page allocator
    ///
    /// Helps avoid issues where page allocation requires pages to put
    /// page-table structures on
    unused_allocated: LockFreeList<FreeRegion<VirtualAddress>>,

    /// Lock for maintaining (restock, shrink) unused_allocated
    maintain_unused_allocated_lock: Spinlock<()>,
}

const fn preferred_region_size(object_size: usize) -> PageCount {
    assert!(object_size > 0);

    // This heuristic can be tweaked as necessary. We want to avoid having too
    // much tracking overhead for not enough allocatable objects.
    if object_size >= PAGE_SIZE/8 {
        4
    } else {
        1
    }
}

pub unsafe fn initialize() -> HeapState {
    let alloc_physical = LockFreeList::new();

    let free_virtual = LockFreeList::new();

    // We reserve some space in order to have some pages pre-mapped, as well as
    // some healthy initial pools for small object sizes, since these are
    // required almost immediately.
    let bootstrap_start = LARGE_HEAP_START +
        (LARGE_HEAP_LENGTH - BOOTSTRAP_HEAP_PAGES) * PAGE_SIZE;

    kernel_acquire_and_map(
        bootstrap_start as *mut u8,
        BOOTSTRAP_HEAP_PAGES,
        |acq| alloc_physical.push(Node::new(acq))
    ).unwrap();

    free_virtual.push(Node::new(FreeRegion {
        start: LARGE_HEAP_START,
        length: AtomicUsize::new(LARGE_HEAP_LENGTH - BOOTSTRAP_HEAP_PAGES),
    }));

    // At least a page large, in order to avoid triggering the small object
    // allocator
    let min_size_of_pools = PAGE_SIZE /
        core::mem::size_of::<(usize, RcPool)>() + 1;

    let mut pools = Vec::with_capacity(min_size_of_pools);

    // It's a big problem if we don't at least have some common sizes in here
    // first. Let's initialize the bootstrap pools in multiples of 8
    for index in 0..BOOTSTRAP_HEAP_POOLS {
        let size = 8 + index * 8;
        let pool = Pool::new(size, 1);
        pool.insert_region(bootstrap_start + index * PAGE_SIZE).unwrap();
        trace!("{:?}", pool);
        pools.push((size, Arc::new(pool)));
    }

    // Put the rest of the pages as unused_allocated
    let initial_unused_allocated = FreeRegion {
        start: bootstrap_start + BOOTSTRAP_HEAP_POOLS * PAGE_SIZE,
        length: AtomicUsize::new(BOOTSTRAP_HEAP_PAGES - BOOTSTRAP_HEAP_POOLS),
    };

    let unused_allocated = LockFreeList::new();

    unused_allocated.push(Node::new(initial_unused_allocated));

    HeapState {
        start: LARGE_HEAP_START,
        end: LARGE_HEAP_START + LARGE_HEAP_LENGTH * PAGE_SIZE,
        length: LARGE_HEAP_LENGTH,
        pools: Spinlock::new(pools),
        stacks_start: STACKS_START as *mut u8,
        stacks_end: Spinlock::new(STACKS_START as *mut u8),
        regions: HeapRegionState {
            alloc_physical,
            free_virtual,
            unused_allocated,
            maintain_unused_allocated_lock: Spinlock::new(()),
        },
    }
}

pub fn debug_print_allocator_stats(state: &HeapState) {
    use crate::terminal::console;

    let _ = writeln!(console(), "Large heap: {:016x} - {:016x}",
        state.start, state.end);

    {
        let pools = state.pools.lock();

        let _ = writeln!(console(), "OSIZE RSIZE FREE     USED     CAPACITY");

        for (size, pool) in pools.iter() {
            let _ = writeln!(console(), "{:<5} {:<5} {:<8} {:<8} {:<8}",
                size,
                pool.region_pages(),
                pool.objects_free(),
                pool.objects_used(),
                pool.objects_capacity());
        }
    }

    {
        let stacks_end = state.stacks_end.lock();

        let _ = writeln!(console(), "Stacks: {:p} - {:p}",
            state.stacks_start, *stacks_end);
    }

    // Physical regions?
}

const MIN_ALIGN: usize = 8;

/// Returns ptr::null on failure
pub unsafe fn allocate(state: &HeapState, size: usize, align: usize)
    -> *mut u8 {

    // Note: either avoid holding onto locks where allocate() might end up being
    // called again, or have a strategy for what happens if the lock is being
    // held.

    // There is a minimum alignment.
    let align = if align < MIN_ALIGN { MIN_ALIGN } else { align };

    // Align the requested size up to the alignment.
    let size_aligned = align_up(size, align);

    // We have two strategies: one for whole pages, one for small objects.
    if size_aligned >= PAGE_SIZE {
        let pages = size_aligned / PAGE_SIZE +
            if size_aligned % PAGE_SIZE != 0 { 1 } else { 0 };

        allocate_pages(state, pages, align)
    } else {
        assert_eq!(PAGE_SIZE % align, 0);
        allocate_small_object(state, size_aligned)
    }
        .map(|p| p as *mut u8)
        .unwrap_or(core::ptr::null::<u8>() as *mut u8)
}

pub unsafe fn deallocate(
    state: &HeapState,
    ptr: *mut u8,
    size: usize,
    align: usize
) {
    // There is a minimum alignment.
    let align = if align < MIN_ALIGN { MIN_ALIGN } else { align };

    // Align the requested size up to the alignment.
    let size_aligned = align_up(size, align);

    if size_aligned >= PAGE_SIZE {
        let pages = size_aligned / PAGE_SIZE +
            if size_aligned % PAGE_SIZE != 0 { 1 } else { 0 };

        deallocate_pages(state, ptr as usize, pages);
    } else {
        deallocate_small_object(state, ptr as usize, size_aligned);
    }
}

/// Allocate pages, preferring already unused mapped pages first.
fn allocate_pages(state: &HeapState, pages: usize, align: usize)
    -> Result<VirtualAddress, ()> {

    if pages == 0 { return Err(()); }

    let regions = &state.regions;

    // First, try to allocate from unused_allocated. If we can get what we need
    // from here, we don't need to map the pages - they're already mapped to
    // usable physical memory
    let from_unused_allocated = allocate_pages_from(
        &regions.unused_allocated, pages, align);

    if let Some(alloc_start) = from_unused_allocated {
        trace!("Got from unused_allocated: {:016x} x {}", alloc_start, pages);
        // Check to see if we need to restock unused_allocated
        restock_unused_allocated(&regions);
        return Ok(alloc_start);
    }

    // Otherwise, allocate virtual from the free_virtual list and acquire and
    // map physical pages.
    acquire_allocate_pages(regions, pages, align)
}

/// Always acquire fresh physical pages and allocate them.
fn acquire_allocate_pages(regions: &HeapRegionState, pages: usize, align: usize)
    -> Result<VirtualAddress, ()> {

    // Allocate virtual from the free_virtual list and acquire and map physical
    // pages.
    let alloc_start = allocate_pages_from(
        &regions.free_virtual, pages, align).ok_or(())?;

    // Map the pages
    let map_res = kernel_acquire_and_map(
        alloc_start as *mut u8,
        pages,
        |acq| {
            regions.alloc_physical.push(Node::new(acq));
        }
    );

    if !map_res.is_ok() {
        return Err(());
    }

    // We successfully allocated!
    Ok(alloc_start)
}

fn allocate_pages_from(
    vaddr_free: &LockFreeList<FreeRegion<VirtualAddress>>,
    pages: usize,
    align: usize
) -> Option<VirtualAddress> {
    // Find a virtual region large enough that will work for the alignment
    trace!("vaddr_free={0:p} {0:?}", vaddr_free);
    trace!("pages={:?}", pages);

    'retry: loop {
        let r = vaddr_free.iter().flat_map(|r| {
            let r_length = r.length.load(Relaxed);

            // Find the end of the region
            let r_end = r.start + r_length * PAGE_SIZE;

            // Figure out where our allocation would need to be placed
            //
            // Prefer to put it near the end so that we can just update the
            // length and avoid taking anything out of the list
            let alloc_start = align_down(r_end - pages * PAGE_SIZE, align);
            let alloc_end = alloc_start + pages * PAGE_SIZE;

            trace!("considering  {:016x} < {:016x}, {:016x} > {:016x}", r.start,
                alloc_start, alloc_end, r_end);

            // If the allocation would fall out of the region, we can't use it
            if r.start > alloc_start { return None; }
            if r_end < alloc_end { return None; }

            // Figure out what the regions before and after would be
            let new_length = (alloc_start - r.start) / PAGE_SIZE;
            let region_after = (r_end, (r_end - alloc_end) / PAGE_SIZE);

            // We could allocate
            Some((r, alloc_start, r_length, new_length, region_after))
        }).nth(0);

        let (region, alloc_start, r_length, new_length, region_after) = match r {
            Some(r) => r,
            None => return None
        };

        // Try to compare_exchange the region length to the new length
        if let Err(_) = region.length.compare_exchange(
            r_length,
            new_length,
            Relaxed,
            Relaxed
        ) {
            // Updated while we did the calculation. Try again.
            continue 'retry;
        }

        trace!("region={:?}", region);
        trace!("alloc_start=0x{:016x}", alloc_start);
        trace!("r_length={}", r_length);
        trace!("new_length={}", new_length);
        trace!("region_after=({:016x}, {})", region_after.0, region_after.1);

        // Insert a node if the after region has length
        let (r_after_start, r_after_length) = region_after;

        if r_after_length > 0 {
            vaddr_free.push(Node::new(FreeRegion {
                start: r_after_start,
                length: AtomicUsize::new(r_after_length),
            }));
        }

        // If new_length = 0, try to remove it from the list
        if new_length == 0 {
            vaddr_free.remove(&region);
        }

        break Some(alloc_start);
    }
}

fn restock_unused_allocated(regions: &HeapRegionState) {
    // Prevent recursion
    if let Some(lock) = regions.maintain_unused_allocated_lock.try_lock() {
        // Get an estimate of how many pages are unused-allocated, as well as
        // what the biggest region size is.
        let (total_unused_allocated, max_region_length) =
            regions.unused_allocated.iter()
                .fold((0, 0),
                    |(total_unused_allocated, max_region_length), region| {
                        let length = region.length.load(Relaxed);
                        (
                            total_unused_allocated + length,
                            cmp::max(max_region_length, length)
                        )
                    });

        if total_unused_allocated < MIN_UNUSED_ALLOCATED ||
            max_region_length < REQUIRED_MIN_REGION_LENGTH {

            // Try to allocate more
            let allocated = acquire_allocate_pages(regions,
                UNUSED_ALLOCATED_ACQUIRE_SIZE,
                PAGE_SIZE);

            if let Ok(start) = allocated {
                let new_region = FreeRegion {
                    start,
                    length: AtomicUsize::new(UNUSED_ALLOCATED_ACQUIRE_SIZE)
                };
                trace!("restock_unused_allocated: {:016x?}", new_region);
                regions.unused_allocated.push(Node::new(new_region));
            } else {
                warn!("Unable to restock_unused_allocated. Out of memory?");
            }
        }

        drop(lock);
    }
}

fn deallocate_pages(state: &HeapState, vaddr: VirtualAddress, pages: usize) {
    // TODO: release pages to unused_allocated first so we don't fiddle with the
    // pageset so often

    if pages == 0 {
        return;
    }

    let vaddr_end = vaddr + pages * PAGE_SIZE;

    let vaddr_range = vaddr..vaddr_end;

    let mut pages_to_deallocate = pages;

    let mut tries = 0;

    // Try a finite number of times to find the matching pages
    while pages_to_deallocate > 0 && tries < 1000 {
        let matching_regions = state.regions.alloc_physical.drain_filter(|acq| {
            let acq_vaddr_end = acq.vaddr as usize + acq.pages * PAGE_SIZE;

            region_math::overlaps(
                &((acq.vaddr as usize)..acq_vaddr_end),
                &vaddr_range
            )
        });

        let mut matched_regions = 0;

        // For each matching region we can grab from the list, rip the matching
        // part out of it, put the non-matching parts back, and then unmap and
        // release that
        for acq in matching_regions {
            matched_regions += 1;

            trace!("matching_regions[_] = {:016x?}", acq);

            let acq_vaddr_end = acq.vaddr as usize + acq.pages * PAGE_SIZE;

            let acq_range = (acq.vaddr as usize)..acq_vaddr_end;

            let cut =
                region_math::cut(acq_range, vaddr_range.clone()).unwrap();

            trace!("cut = {:016x?}", cut);

            // Create a new acq that's going to be only the region we want to
            // actually release
            let matching_acq = AcquiredMappedRegion {
                // we use max because we want the beginning of the acq if my
                // vaddr is less than it
                vaddr: cut.excluded.start as *mut u8,
                paddr: acq.paddr + (cut.excluded.start - acq.vaddr as usize),
                // the matching pages within the acq region
                pages: (cut.excluded.end - cut.excluded.start)/PAGE_SIZE,
            };

            trace!("matching_acq={:016x?}", matching_acq);

            if let Some(ref before) = cut.before {
                let acq_before = AcquiredMappedRegion {
                    vaddr: before.start as *mut u8,
                    paddr: acq.paddr,
                    pages: (before.end - before.start)/PAGE_SIZE,
                };
                trace!("acq_before={:016x?}", acq_before);
                state.regions.alloc_physical.push(Node::new(acq_before));
            }

            if let Some(ref after) = cut.after {
                let acq_after = AcquiredMappedRegion {
                    vaddr: after.start as *mut u8,
                    paddr: acq.paddr + (after.start - acq.vaddr as usize),
                    pages: (after.end - after.start)/PAGE_SIZE,
                };
                trace!("acq_after={:016x?}", acq_after);
                state.regions.alloc_physical.push(Node::new(acq_after));
            }

            kernel_unmap_and_release(matching_acq);
            pages_to_deallocate -= matching_acq.pages;
        }

        if matched_regions == 0 {
            tries += 1;
        }
    }

    if pages_to_deallocate > 0 {
        panic!(
            "deallocate_pages({:016x}, {}) had {} leftover pages to \
            deallocate that it couldn't find.",
            vaddr, pages, pages_to_deallocate
        );
    }

    super::release_to_free_region_list(
        &state.regions.free_virtual,
        vaddr,
        pages,
        "virtual");
}

// Map on sorted vec
fn pool_get(state: &HeapState, size: usize) -> Option<RcPool> {
    let pools = state.pools.lock();

    if let Ok(index) = pools.binary_search_by_key(&size, |tup| tup.0) {
        Some(pools[index].1.clone())
    } else {
        None
    }
}

fn pool_get_or_create(state: &HeapState, size: usize) -> RcPool {
    pool_get(state, size).unwrap_or_else(|| {
        // Allocate a new pool
        let new_pool = Arc::new(
            Pool::new(size, preferred_region_size(size)));

        // We should be free from allocation here - EXCEPT if pools Vec needs to
        // be grown, but that will be up to the page allocator, so the pools
        // lock won't be touched.
        let mut pools = state.pools.lock();
        match pools.binary_search_by_key(&size, |tup| tup.0) {
            // It exists now?
            Ok(found) =>
                pools[found].1.clone(),
            // Put it where the binary search said to
            Err(insert_to) => {
                pools.insert(insert_to, (size, new_pool.clone()));
                new_pool
            }
        }
    })
}

fn add_region_to_pool(state: &HeapState, pool: &Pool) -> Result<(), ()> {
    let region = allocate_pages(state, pool.region_pages(), PAGE_SIZE)?;

    assert!(region != 0);

    unsafe { pool.insert_region(region) }.unwrap();

    Ok(())
}

fn allocate_small_object(state: &HeapState, size_aligned: usize)
    -> Result<VirtualAddress, ()> {

    // Find the pool appropriate to the object, or create it
    let pool = pool_get_or_create(state, size_aligned);

    {
        // Try to just get an address from the pool
        if let Ok(vaddr) = pool.allocate() {
            // It's dangerous if all of the pools are too full, so we need to
            // make sure they always have some free space.
            //
            // If the pool is now almost full, add another region
            if pool.objects_free() < pool.region_object_capacity()/2 {
                // Use the pool maintenance guard to avoid getting stuck in a
                // loop doing this.
                pool.try_maintain(|| {
                    let _ = add_region_to_pool(state, &pool);
                });
            }
            return Ok(vaddr);
        }
    }

    // The pool is probably empty. Allocate a region first and try again
    add_region_to_pool(state, &pool)?;
    Ok(pool.allocate().expect("Added region but pool still empty"))
}

fn deallocate_small_object(
    state: &HeapState,
    vaddr: usize,
    size_aligned: usize
) {

    // Find the pool appropriate to the object
    if let Some(pool) = pool_get(state, size_aligned) {
        if let Err(err) = pool.deallocate(vaddr) {
            warn!("BUG: Pool {} deallocation error: {}", size_aligned, err);
        }
    } else {
        warn!("BUG: Trying to deallocate unknown pointer 0x{:16x} x {}",
            vaddr, size_aligned);
    }
}

pub fn allocate_kernel_stack(heap_state: &HeapState) -> *mut u8 {
    assert_eq!(KSTACK_SIZE % PAGE_SIZE, 0,
        "Kernel stack size must be a multiple of the page size");

    let pages = KSTACK_SIZE / PAGE_SIZE;

    // Increment, skip one page for next time, stacks have guard page on either
    // side
    let new_stack = unsafe {
        let mut stacks_end = heap_state.stacks_end.lock();
        *stacks_end = stacks_end.offset(
            ((pages + 1) * PAGE_SIZE) as isize);
        *stacks_end
    };

    // Allocate pages to the stack area
    kernel_acquire_and_map(new_stack, pages, 
        |acq| {
            heap_state.regions.alloc_physical.push(Node::new(acq));
        }
    ).unwrap_or_else(|_| panic!("Out of memory"));

    // Actually return the stack pointer (going down)
    unsafe {
        new_stack.offset(KSTACK_SIZE as isize)
    }
}
