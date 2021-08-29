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

use crate::sync::Spinlock;
use crate::paging::PAGE_SIZE;

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::collections::BTreeSet;

use core::sync::atomic::{AtomicBool, AtomicUsize};
use core::sync::atomic::Ordering::*;


use super::{FreeRegion, PhysicalAddress, VirtualAddress, PageCount};
use super::KSTACK_SIZE;
use super::{kernel_acquire_and_map, align_addr};
use super::pool::Pool;

pub const LARGE_HEAP_START: usize = 0xffff_ffff_9000_0000;
pub const LARGE_HEAP_LENGTH: usize = 0x20000; // pages
pub const STACKS_START: usize = 0xffff_ffff_f000_0000;
pub const SPARE_PAGES: usize = 8;

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
    regions: Spinlock<HeapRegionState>,
    // Spare pages (already mapped) to be used in emergencies
    spare_pages: [AtomicUsize; SPARE_PAGES],
    spare_pages_dirty: AtomicBool,
}

#[derive(Debug)]
struct HeapRegionState {
    alloc_physical: Vec<(PhysicalAddress, PageCount)>,
    free_virtual: BTreeSet<FreeRegion<VirtualAddress>>,
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
    let mut alloc_physical = vec![];

    kernel_acquire_and_map(
        LARGE_HEAP_START as *mut u8,
        SPARE_PAGES,
        &mut alloc_physical,
    )
    .expect("Failed to initialize large heap spare pages");

    let spare_pages = [const { AtomicUsize::new(0) }; SPARE_PAGES];

    for index in 0..SPARE_PAGES {
        spare_pages[index].store(LARGE_HEAP_START + PAGE_SIZE * index, Relaxed);
    }

    let mut free_virtual = BTreeSet::new();

    let initial_start = LARGE_HEAP_START + PAGE_SIZE * SPARE_PAGES;

    free_virtual.insert(FreeRegion {
        start: initial_start,
        length: LARGE_HEAP_LENGTH - SPARE_PAGES,
    });

    // At least a page large, in order to avoid triggering the small object
    // allocator
    let min_size_of_pools = PAGE_SIZE /
        core::mem::size_of::<(usize, RcPool)>() + 1;

    let mut pools = Vec::with_capacity(min_size_of_pools);

    // It's a big problem if we don't at least have some common sizes in here
    // first. Let's initialize every multiple of 8 up to 128
    for size in (8..=128).step_by(8) {
        let pool = Pool::new(size, preferred_region_size(size));
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
        regions: Spinlock::new(HeapRegionState {
            alloc_physical,
            free_virtual,
        }),
        spare_pages: spare_pages,
        spare_pages_dirty: AtomicBool::new(false),
    }
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

    if let Some(mut regions) = state.regions.try_lock() {
        // Find a virtual region large enough that will work for the alignment
        let range = .. FreeRegion { length: pages - 1, start: 0 };

        debug!("regions.free_virtual={:?}", regions.free_virtual);
        debug!("pages={:?}, range={:?}", pages, range);

        let r = regions.free_virtual.range(range).flat_map(|r| {
            // Find the end of the region
            let r_end = r.start + r.length * page_size;

            // Figure out where our allocation would need to be placed
            let alloc_start = align_addr(r.start, align);
            let alloc_end = alloc_start + pages * page_size;

            debug!("considering  {:016x} < {:016x}, {:016x} > {:016x}", r.start,
                alloc_start, alloc_end, r_end);

            // If the allocation would fall out of the region, we can't use it
            if r_end < alloc_end { return None; }

            // Figure out what the regions before and after would be
            let region_before = FreeRegion {
                length: (alloc_start - r.start) / page_size,
                start: r.start
            };
            let region_after = FreeRegion {
                length: (r_end - alloc_end) / page_size,
                start: r_end
            };

            // We could allocate
            Some((r.clone(), alloc_start, region_before, region_after))
        }).nth(0);

        let (old_region, alloc_start, region_before, region_after) = match r {
            Some(r) => r,
            None => return Err(())
        };

        // Remove the old region
        regions.free_virtual.remove(&old_region);

        // If there's space around the allocated region, save it
        if region_before.length > 0 {
            regions.free_virtual.insert(region_before);
        }
        if region_after.length > 0 {
            regions.free_virtual.insert(region_after);
        }

        // Map the pages
        let map_res = kernel_acquire_and_map(
            alloc_start as *mut u8,
            pages,
            &mut regions.alloc_physical);

        if !map_res.is_ok() {
            return Err(());
        }

        // See if we might be able to add more spare pages
        add_spare_pages(state, &mut regions);

        // We successfully allocated!
        Ok(alloc_start)
    } else {
        // Special case: if we only need one page, and align is multiple of the
        // page size, we can use a spare page.
        if pages == 1 && PAGE_SIZE % align == 0 {
            debug!("need a spare page");

            // Find one we can take
            for index in 0..SPARE_PAGES {
                let page = state.spare_pages[index].swap(0, Relaxed);

                if page != 0 {
                    state.spare_pages_dirty.store(true, Relaxed);
                    return Ok(page);
                }
            }

            // Not found... :(
            Err(())
        } else {
            // Can't allocate pages while the region lock is held
            Err(())
        }
    }
}

/// Try to add spare pages if the dirty flag is set
fn add_spare_pages(state: &HeapState, regions: &mut HeapRegionState) {
    if state
        .spare_pages_dirty
        .compare_exchange(true, false, Relaxed, Relaxed)
        .is_ok()
    {
        // Count how many are empty
        let wanted_pages = (0..SPARE_PAGES)
            .filter(|index| state.spare_pages[*index].load(Relaxed) == 0)
            .count();

        debug!("wanted_pages={}", wanted_pages);

        // Don't need to allocate.
        if wanted_pages < 1 {
            return;
        }

        let mut region = match regions.free_virtual.iter().rev().cloned().nth(0)
        {
            Some(r) => r,
            None => return,
        };

        regions.free_virtual.remove(&region);

        if region.length > wanted_pages {
            regions.free_virtual.insert(FreeRegion {
                start: region.start + PAGE_SIZE * wanted_pages,
                length: region.length - wanted_pages,
            });
            region.length = wanted_pages;
        }

        let map_res = kernel_acquire_and_map(
            region.start as *mut u8,
            region.length,
            &mut regions.alloc_physical,
        );

        // Calculate the addresses of the allocated pages and push them
        if map_res.is_ok() {
            let mut spare_page_index = 0;
            let mut uninserted_pages = region.length;

            for pnum in 0..region.length {
                while spare_page_index < SPARE_PAGES {
                    let page_addr = region.start + PAGE_SIZE * pnum;
                    let push_res = state.spare_pages[spare_page_index]
                        .compare_exchange(0, page_addr, Relaxed, Relaxed);

                    spare_page_index += 1;

                    if push_res.is_ok() {
                        uninserted_pages -= 1;
                        break;
                    }
                }
            }

            assert_eq!(uninserted_pages, 0);
        }

        debug!("add_spare_pages end");
    }
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

fn allocate_small_object(state: &HeapState, size_aligned: usize)
    -> Result<VirtualAddress, ()> {

    // Find the pool appropriate to the object, or create it
    let pool = pool_get(state, size_aligned);

    {
        // Try to just get an address from the pool
        if let Ok(vaddr) = pool.allocate() {
            return Ok(vaddr);
        }
    }

    // The pool is probably empty. Allocate a region first and try again
    let region = allocate_pages(state, pool.region_pages(), PAGE_SIZE)?;

    assert!(region != 0);

    unsafe { pool.insert_region(region) }.unwrap();
    Ok(pool.allocate().expect("Added region but pool still empty"))
}

pub fn allocate_kernel_stack(heap_state: &HeapState) -> *mut u8 {
    assert_eq!(KSTACK_SIZE % PAGE_SIZE, 0,
        "Kernel stack size must be a multiple of the page size");

    let pages = KSTACK_SIZE / PAGE_SIZE;

    let mut regions = heap_state.regions.lock();

    // Increment, skip one page for next time, stacks have guard page on either
    // side
    let new_stack = unsafe {
        let mut stacks_end = heap_state.stacks_end.lock();
        *stacks_end = stacks_end.offset(
            ((pages + 1) * PAGE_SIZE) as isize);
        *stacks_end
    };

    // Allocate pages to the stack area
    kernel_acquire_and_map(new_stack, pages, &mut regions.alloc_physical)
        .unwrap_or_else(|_| panic!("Out of memory"));

    // Actually return the stack pointer (going down)
    unsafe {
        new_stack.offset(KSTACK_SIZE as isize)
    }
}
