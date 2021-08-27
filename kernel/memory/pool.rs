/*******************************************************************************
 *
 * kit/kernel/memory/pool.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use core::fmt;

use alloc::vec::Vec;
use alloc::collections::{BTreeMap, VecDeque};

use crate::paging::{GenericPageset, Pageset};

use super::VirtualAddress;

#[derive(Debug)]
pub enum Error {
    NoFreeSpace,
    AddressNotAllocated(usize),
    PageAlreadyExists(usize),
    PageNotEmpty(usize),
    PageNotFound(usize),
}

pub struct Pool {
    object_size: usize,
    objects_allocated: usize,
    objects_capacity: usize,
    pages: LittleMap<VirtualAddress, Page>,
    // normally FIFO to the front, but new pages go to the back
    free_pages: VecDeque<VirtualAddress>,
}

const LITTLE_MAX: usize = 8;

/// A map that starts as a plain vec in order to make initial insertions
/// allocation-free.
enum LittleMap<K, V> {
    Little(Vec<(K, V)>),
    Big(BTreeMap<K, V>)
}

impl<K: Ord, V> LittleMap<K, V> {
    fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        match *self {
            LittleMap::Little(ref mut vec) => {
                vec.iter_mut().find(|tup| &tup.0 == key)
                    .map(|&mut (_, ref mut v)| v)
            },
            LittleMap::Big(ref mut map) => {
                map.get_mut(key)
            }
        }
    }

    fn insert_if_not_present<F>(&mut self, key: K, value: F) -> bool
        where F: FnOnce() -> V {

        match *self {
            LittleMap::Little(ref mut vec) => {
                if vec.iter().any(|tup| tup.0 == key) {
                    return false;
                }

                if vec.len() == LITTLE_MAX {
                    let mut new_map = vec.drain(..).collect::<BTreeMap<K, V>>();

                    new_map.insert(key, value());

                    *self = LittleMap::Big(new_map);
                    true
                } else {
                    vec.push((key, value()));
                    true
                }
            },
            LittleMap::Big(ref mut map) => {
                use alloc::collections::btree_map::Entry;

                if let Entry::Vacant(e) = map.entry(key) {
                    e.insert(value());
                    true
                } else {
                    false
                }
            },
        }
    }

    fn remove(&mut self, key: &K) -> bool {
        match *self {
            LittleMap::Little(ref mut vec) => {
                if let Some(index) = vec.iter().position(|(k, _)| k == key) {
                    vec.remove(index);
                    true
                } else {
                    false
                }
            },
            LittleMap::Big(ref mut map) => {
                map.remove(key).is_some()
            }
        }
    }
}

impl fmt::Debug for Pool {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Pool")
            .field("object_size", &self.object_size)
            .field("objects_allocated", &self.objects_allocated)
            .field("objects_capacity", &self.objects_capacity)
            .finish_non_exhaustive()
    }
}

impl Pool {
    /// Create a new pool for objects of the given size.
    ///
    /// Object size must be less than page size.
    ///
    /// Size should account for any required alignment.
    pub fn new(object_size: usize) -> Pool {
        assert!(object_size < Pageset::page_size(),
            "Pool is only for objects that are smaller than the page size");

        Pool {
            object_size,
            objects_allocated: 0,
            objects_capacity: 0,
            pages: LittleMap::Little(Vec::with_capacity(2)),
            free_pages: VecDeque::with_capacity(2),
        }
    }

    pub fn object_size(&self) -> usize {
        self.object_size
    }

    pub fn objects_allocated(&self) -> usize {
        self.objects_allocated
    }

    pub fn objects_capacity(&self) -> usize {
        self.objects_capacity
    }

    pub fn objects_free(&self) -> usize {
        self.objects_capacity - self.objects_allocated
    }

    pub fn objects_per_page(&self) -> usize {
        Pageset::page_size() / self.object_size
    }

    pub fn pages_free(&self) -> usize {
        self.free_pages.len()
    }

    /// Get an address for an object
    pub fn allocate(&mut self) -> Result<VirtualAddress, Error> {
        // Try to get a free page
        if let Some(page_addr) = self.free_pages.pop_front() {
            // Find that page
            let page = self.pages.get_mut(&page_addr).unwrap_or_else(|| {
                panic!("Unable to find known free page 0x{:016x} in page index",
                    page_addr)
            });

            // Get from that page
            let vaddr = page.take_free(page_addr, self.object_size)
                .unwrap_or_else(|| {
                    panic!("Supposedly free page 0x{:016x} \
                        was not actually free", page_addr)
                });

            // Increment the counter
            self.objects_allocated += 1;

            // Put it back on the list if it's not full
            if !page.is_full() {
                self.free_pages.push_front(page_addr);
            }

            Ok(vaddr)
        } else {
            Err(Error::NoFreeSpace)
        }
    }

    pub fn deallocate(&mut self, object_addr: VirtualAddress) -> Result<Deallocated, Error> {
        let page_size = Pageset::page_size();
        let page_addr = (object_addr / page_size) * page_size;

        if let Some(page) = self.pages.get_mut(&page_addr) {
            let was_full = page.is_full();

            if page.mark_free(object_addr, self.object_size) {
                // If it was full we need to add it to the free list
                if was_full {
                    self.free_pages.push_front(object_addr);
                }

                // Decrement the counter
                self.objects_allocated -= 1;

                Ok(Deallocated {
                    // If the page is now empty, let the caller know they can
                    // release it
                    unused_page: if page.is_empty(self.object_size) {
                        Some(page_addr)
                    } else { 
                        None
                    }
                })
            } else {
                Err(Error::AddressNotAllocated(object_addr))
            }
        } else {
            Err(Error::AddressNotAllocated(object_addr))
        }
    }

    pub fn add_page(&mut self, page_addr: VirtualAddress) -> Result<(), Error> {
        let size = self.object_size;

        if self.pages.insert_if_not_present(page_addr, || Page::new(size)) {
            // Add it to the free list, to the back so it's used last
            self.free_pages.push_back(page_addr);

            // Add to the counter
            self.objects_capacity += self.objects_per_page();

            Ok(())
        } else {
            // Don't overwrite
            Err(Error::PageAlreadyExists(page_addr))
        }
    }

    pub fn remove_page(&mut self, page_addr: VirtualAddress) -> Result<(), Error> {
        // Verify that the page exists and is empty
        if let Some(page) = self.pages.get_mut(&page_addr) {
            if !page.is_empty(self.object_size) {
                return Err(Error::PageNotEmpty(page_addr));
            }
        } else {
            return Err(Error::PageNotFound(page_addr));
        }

        self.pages.remove(&page_addr);

        // Remove it from the free page list
        if let Some(index) = self.free_pages.iter().position(|p| *p == page_addr) {
            self.free_pages.remove(index);
        } else {
            panic!("Expected page 0x{:016x} to be in free_pages", page_addr);
        }

        // Subtract from the counter
        self.objects_capacity -= self.objects_per_page();

        Ok(())
    }
}

#[must_use]
pub struct Deallocated {
    unused_page: Option<VirtualAddress>,
}

struct Page {
    used_bitmap: Vec<u8>,
}

impl Page {
    fn new(object_size: usize) -> Page {
        let bits_size = Pageset::page_size()/object_size;
        let bytes_size = bits_size / 8 + if bits_size % 8 == 0 { 1 } else { 0 };

        let mut used_bitmap = vec![0; bytes_size];

        // Set any excess bits to 1 (used)
        if bits_size % 8 != 0 {
            let mask = !(1 << (bits_size % 8) - 1);
            used_bitmap[bytes_size - 1] |= mask;
        }

        Page { used_bitmap }
    }

    fn is_empty(&self, object_size: usize) -> bool {
        let bits_size = Pageset::page_size()/object_size;
        self.used_bitmap.iter().enumerate().all(|(byte_num, byte)| {
            // Handle last byte
            if byte_num == bits_size / 8 {
                let mask = 1 << (bits_size % 8) - 1;
                *byte & mask == 0
            } else {
                *byte == 0
            }
        })
    }

    fn is_full(&self) -> bool {
        self.used_bitmap.iter().all(|byte| *byte == 0xFF)
    }

    fn take_free(
        &mut self,
        page_addr: VirtualAddress,
        object_size: usize
    ) -> Option<VirtualAddress> {
        // Find the first clear bit, then calculate an address
        for (byte_num, byte) in self.used_bitmap.iter_mut().enumerate() {
            for bit_num in 0..8 {
                let mask = 1 << bit_num;
                if *byte & mask == 0 {
                    *byte |= mask;
                    let object_num = byte_num * 8 + bit_num;
                    return Some(page_addr + object_size * object_num);
                }
            }
        }
        None
    }

    fn mark_free(
        &mut self,
        object_addr: VirtualAddress,
        object_size: usize
    ) -> bool {
        // Find the bit corresponding to the object, then clear it, return true
        // if it was set
        let object_num = (object_addr % Pageset::page_size())/object_size;
        let byte_num = object_num / 8;
        let mask = 1 << (object_num % 8);
        let was_set = self.used_bitmap[byte_num] & mask != 0;
        self.used_bitmap[byte_num] &= !mask;
        was_set
    }
}
