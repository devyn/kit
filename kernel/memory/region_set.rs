/*******************************************************************************
 *
 * kit/kernel/memory/region_set.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use core::cmp;
use core::ops::Range;

use alloc::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct RegionSet<T: Ord> {
    regions: BTreeMap<T, T>,
}

impl<T: Ord + Clone> RegionSet<T> {
    pub fn new() -> RegionSet<T> {
        RegionSet {
            regions: BTreeMap::new(),
        }
    }

    fn normalize(&mut self) {
        self.regions.retain(|start, end| start < end);

        while !self.regions.is_empty() {
            let mut iter = self.regions.iter();
            let mut previous = iter.next().unwrap();
            let mut unify_target = None;

            for current in iter {
                if current.0 < previous.1 {
                    // Can unify
                    unify_target = Some((
                        (previous.0.clone(), previous.1.clone()),
                        (current.0.clone(), current.1.clone())));
                    break;
                } else {
                    previous = current;
                }
            }

            if let Some((a, b)) = unify_target {
                // Remove the region (b) that will be unified with
                self.regions.remove(&b.0);
                // If b stretches farther than a
                if b.1 > a.1 {
                    // Extend a to the region (b) we will unify with
                    *self.regions.get_mut(&a.0).unwrap() = b.1;
                }
            } else {
                // Nothing to unify
                break;
            }
        }
    }

    pub fn insert(&mut self, region: Range<T>) {
        let mut replace = None;
        let mut inserted = false;

        for (start, end) in self.regions.iter_mut() {
            if region.start >= *start && region.start <= *end {
                // Can be extended
                *end = region.end.clone();
                inserted = true;
            } else if region.start < *start && region.end >= *start {
                // Can be unified
                replace = Some((
                    start.clone(),
                    (region.start.clone(), cmp::max(&region.end, &end).clone()),
                ));
            }
        }

        if let Some((old_start, (new_start, new_end))) = replace {
            self.regions.remove(&old_start);
            self.regions.insert(new_start, new_end);
        } else if !inserted {
            self.regions.insert(region.start, region.end);
        }

        self.normalize();
    }

    pub fn remove(&mut self, region: Range<T>) {
        let mut split_new = None;

        for (start, end) in self.regions.iter_mut() {
            // Skip if totally non-overlapping
            if region.end < *start || region.start > *end { continue; }

            if region.start >= *start && region.end >= *end {
                // Cut off end of region
                *end = region.start.clone();
            } else if region.end < *end {
                // Cut off beginning of region
                split_new = Some((region.end.clone(), end.clone()));
                *end = region.start.clone();
                // We won't find any matching regions beyond this, so we can
                // terminate.
                break;
            } else if region.start <= *start && region.end >= *end {
                // Remove whole region
                *end = start.clone();
            }
        }

        if let Some((start, end)) = split_new {
            if end > start {
                self.regions.insert(start, end);
            }
        }

        self.normalize();
    }

    pub fn unwrap(self) -> BTreeMap<T, T> {
        self.regions
    }

    pub fn iter(&self) -> impl Iterator<Item = Range<T>> + '_ {
        self.regions
            .iter()
            .map(|(start, end)| start.clone()..end.clone())
    }
}

impl<T: Ord + Clone> From<RegionSet<T>> for BTreeMap<T, T> {
    fn from(set: RegionSet<T>) -> BTreeMap<T, T> {
        set.unwrap()
    }
}

#[cfg(test)]
macro_rules! insert_test {
    (($($before_range:expr),*$(,)?) => ($($after_range:expr),*$(,)?)) => ({
        let mut set = RegionSet::new();

        $(
            set.insert($before_range);
        )*

        let vec: Vec<Range<i32>> = set.iter().collect();

        assert_eq!(&vec, &[$($after_range),*]);
    })
}

#[test]
fn region_insert_unify_end() {
    insert_test!((
        0..1000,
        1000..2000,
        4000..5000,
    ) => (
        0..2000,
        4000..5000,
    ));
}
#[test]
fn region_insert_unify_start() {
    insert_test!((1000..2000, 0..1000) => (0..2000));
}

#[test]
fn region_insert_encompassing_single() {
    insert_test!((1000..2000, 0..3000) => (0..3000));
}

#[test]
fn region_insert_encompassing_multiple() {
    insert_test!((1000..2000, 4000..5000, 0..6000) => (0..6000));
}

#[test]
fn region_insert_between_two() {
    insert_test!((1000..2000, 4000..5000, 2000..4000) => (1000..5000));
}

#[cfg(test)]
macro_rules! remove_test {
    (($($initial_range:expr),*$(,)?) -
     ($($remove_range:expr),*$(,)?) =>
     ($($after_range:expr),*$(,)?)) => ({
        let mut map = BTreeMap::new();

        $(
            let range = $initial_range;
            map.insert(range.start, range.end);
        )*

        let mut set = RegionSet { regions: map };

        $(
            set.remove($remove_range);
        )*

        let vec: Vec<Range<i32>> = set.iter().collect();

        assert_eq!(&vec, &[$($after_range),*]);
    })
}

#[test]
fn region_remove_split() {
    remove_test!((0..5000, 8000..9000) - (1000..2000)
        => (0..1000, 2000..5000, 8000..9000));
}

#[test]
fn region_remove_exact() {
    remove_test!((2000..3000, 8000..9000) - (2000..3000)
        => (8000..9000));
}

#[test]
fn region_remove_cut_start() {
    remove_test!((1000..6000, 8000..9000) - (0..2000)
        => (2000..6000, 8000..9000));
}

#[test]
fn region_remove_cut_end() {
    remove_test!((1000..6000, 8000..9000) - (3000..7000)
        => (1000..3000, 8000..9000));
}

#[test]
fn region_remove_encompassing() {
    remove_test!((1000..6000, 8000..9000) - (0..7000) => (8000..9000));
}

#[test]
fn region_remove_after_nonmatching() {
    remove_test!((0..2000, 5000..10000) - (5000..6000)
        => (0..2000, 6000..10000));
}

#[test]
fn region_complex1() {
    let mut set = RegionSet::new();

    set.insert(005000..010000);
    set.insert(000000..002000);

    let vec: Vec<Range<usize>> = set.iter().collect();

    assert_eq!(&vec, &[
        (000000..002000),
        (005000..010000),
    ]);

    set.remove(005000..006000);

    let vec: Vec<Range<usize>> = set.iter().collect();

    assert_eq!(&vec, &[
        (000000..002000),
        (006000..010000),
    ]);

    set.remove(008000..009000);

    let vec: Vec<Range<usize>> = set.iter().collect();

    assert_eq!(&vec, &[
        (000000..002000),
        (006000..008000),
        (009000..010000),
    ]);

    set.insert(002000..006000);

    let vec: Vec<Range<usize>> = set.iter().collect();

    assert_eq!(&vec, &[
        (000000..008000),
        (009000..010000),
    ]);
}
