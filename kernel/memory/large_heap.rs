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

use alloc::sync::Arc;
use alloc::vec::Vec;

use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::*;

use super::{PhysicalAddress, VirtualAddress, PageCount, FreeRegion};
use super::KSTACK_SIZE;
use super::{kernel_acquire_and_map, align_addr, align_addr_down};
use super::pool::Pool;

pub const LARGE_HEAP_START: usize = 0xffff_ffff_9000_0000;
pub const LARGE_HEAP_LENGTH: usize = 0x20000; // pages
pub const STACKS_START: usize = 0xffff_ffff_f000_0000;
pub const BOOTSTRAP_HEAP_PAGES: usize = 16;

type RcPool = Arc<Pool>;

#[derive(Debug)]
pub struct HeapState {
    start: VirtualAddress,
    end: VirtualAddress,
    length: PageCount,
    // For allocating smaller than page size objects - one pool for each object
    // size
    //
    // Using a sorted Vec that's always at least a page big in order to avoid
    // lock issues
    pools: Spinlock<Vec<(usize, RcPool)>>,
    // For allocating stacks
    stacks_start: *mut u8,
    stacks_end: Spinlock<*mut u8>,
    // Tracked memory regions
    regions: HeapRegionState,
}

#[derive(Debug)]
struct HeapRegionState {
    alloc_physical: LockFreeList<(PhysicalAddress, PageCount)>,
    free_virtual: LockFreeList<FreeRegion<VirtualAddress>>,
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

    // We reserve some space in order to have some healthy initial pools for
    // small object sizes, since these are required almost immediately.
    let bootstrap_start = LARGE_HEAP_START +
        (LARGE_HEAP_LENGTH - BOOTSTRAP_HEAP_PAGES) * PAGE_SIZE;

    kernel_acquire_and_map(
        bootstrap_start as *mut u8,
        BOOTSTRAP_HEAP_PAGES,
        |start, length| alloc_physical.push(Node::new((start, length)))
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
    // first. Let's initialize the bootstrap pages in multiples of 8
    for index in 0..BOOTSTRAP_HEAP_PAGES {
        let size = 8 + index * 8;
        let pool = Pool::new(size, 1);
        pool.insert_region(bootstrap_start + index * PAGE_SIZE).unwrap();
        debug!("{:?}", pool);
        pools.push((size, Arc::new(pool)));
    }

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
    let size_aligned = align_addr(size, align);

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

fn allocate_pages(state: &HeapState, pages: usize, align: usize)
    -> Result<VirtualAddress, ()> {

    if pages == 0 { return Err(()); }

    let page_size = PAGE_SIZE;

    let regions = &state.regions;

    // Find a virtual region large enough that will work for the alignment
    debug!("regions.free_virtual={:?}", regions.free_virtual);
    debug!("pages={:?}", pages);

    let alloc_start = 'retry: loop {
        let r = regions.free_virtual.iter().flat_map(|r| {
            let r_length = r.length.load(Relaxed);

            // Find the end of the region
            let r_end = r.start + r_length * page_size;

            // Figure out where our allocation would need to be placed
            //
            // Prefer to put it near the end so that we can just update the
            // length and avoid taking anything out of the list
            let alloc_start = align_addr_down(r_end - pages * page_size, align);
            let alloc_end = alloc_start + pages * page_size;

            debug!("considering  {:016x} < {:016x}, {:016x} > {:016x}", r.start,
                alloc_start, alloc_end, r_end);

            // If the allocation would fall out of the region, we can't use it
            if r.start > alloc_start { return None; }
            if r_end < alloc_end { return None; }

            // Figure out what the regions before and after would be
            let new_length = (alloc_start - r.start) / page_size;
            let region_after = (r_end, (r_end - alloc_end) / page_size);

            // We could allocate
            Some((r, alloc_start, r_length, new_length, region_after))
        }).nth(0);

        let (region, alloc_start, r_length, new_length, region_after) = match r {
            Some(r) => r,
            None => return Err(())
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

        debug!("region={:?}", region);
        debug!("alloc_start=0x{:016x}", alloc_start);
        debug!("r_length={}", r_length);
        debug!("new_length={}", new_length);
        debug!("region_after=({:016x}, {})", region_after.0, region_after.1);

        // Insert a node if the after region has length
        let (r_after_start, r_after_length) = region_after;

        if r_after_length > 0 {
            regions.free_virtual.push(Node::new(FreeRegion {
                start: r_after_start,
                length: AtomicUsize::new(r_after_length),
            }));
        }

        // If new_length = 0, try to remove it from the list
        if new_length == 0 {
            regions.free_virtual.remove(&region);
        }

        break alloc_start;
    };

    // Map the pages
    let map_res = kernel_acquire_and_map(
        alloc_start as *mut u8,
        pages,
        |start, pages| {
            regions.alloc_physical.push(Node::new((start, pages)));
        }
    );

    if !map_res.is_ok() {
        return Err(());
    }

    // We successfully allocated!
    Ok(alloc_start)
}

// Map on sorted vec
fn pool_get(state: &HeapState, size: usize) -> RcPool {
    let pools = state.pools.lock();

    if let Ok(index) = pools.binary_search_by_key(&size, |tup| tup.0) {
        pools[index].1.clone()
    } else {
        // Don't hold the lock while we allocate something
        drop(pools);

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
    }
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
    let pool = pool_get(state, size_aligned);

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
        |start, pages| {
            heap_state.regions.alloc_physical.push(Node::new((start, pages)));
        }
    ).unwrap_or_else(|_| panic!("Out of memory"));

    // Actually return the stack pointer (going down)
    unsafe {
        new_stack.offset(KSTACK_SIZE as isize)
    }
}
