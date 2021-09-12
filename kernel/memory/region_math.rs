/*******************************************************************************
 *
 * kit/kernel/memory/region_math.rs
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
        // Remove any empty regions
        self.regions.retain(|start, end| start < end);

        // Ensure that all overlapping regions are joined
        while !self.regions.is_empty() {
            let mut iter = self.regions.iter().peekable();
            let mut unify_target: Option<((T, T), (T, T))> = None;

            while let Some(current) = iter.next() {
                if let Some(next) = iter.peek() {
                    let joined = join(
                        current.0..current.1,
                        next.0..next.1);

                    if joined.is_some() {
                        // Can unify
                        unify_target = Some((
                            (current.0.clone(), current.1.clone()),
                            (next.0.clone(), next.1.clone())
                        ));
                        break;
                    }
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
            let here = start.clone()..end.clone();

            if let Some(joined) = join(region.clone(), here.clone()) {
                if joined.start == here.start {
                    // Can be extended
                    *end = joined.end;
                    inserted = true;
                    break;
                } else {
                    // Can be unified
                    replace = Some((here.start, (joined.start, joined.end)));
                    break;
                }
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
            let here = start.clone()..end.clone();
            if let Some(cut) = cut(here, region.clone()) {
                let Cut { before, after, .. } = cut;

                // If there's some left over before, we can mutate here to match
                // the before region
                if let Some(before) = before {
                    *end = before.end;
                } else {
                    // Otherwise, this will set it to zero length and it will be
                    // deleted by normalize
                    *end = start.clone();
                }

                // If there is after, we won't find any matching regions beyond
                // this, so we can terminate and let split_new insert it
                if let Some(after) = after {
                    split_new = Some((after.start, after.end));
                    break;
                }
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

/// If the regions intersect or touch
pub fn intersects<T: Ord + Clone>(a: &Range<T>, b: &Range<T>) -> bool {
    (a.start < b.end && a.end >= b.start) ||
        (b.start < a.end && b.end >= a.start)
}

/// If the regions overlap (share space)
pub fn overlaps<T: Ord + Clone>(a: &Range<T>, b: &Range<T>) -> bool {
    (a.start < b.end && a.end > b.start) ||
        (b.start < a.end && b.end > a.start)
}

/// Join a region into another region, if they touch each other
pub fn join<T: Ord + Clone>(a: Range<T>, b: Range<T>)
    -> Option<Range<T>> {

    if intersects(&a, &b) {
        Some(cmp::min(a.start.clone(), b.start.clone()) ..
             cmp::max(a.end.clone(), b.end.clone()))
    } else {
        None
    }
}

#[test]
fn join_non_overlapping_a_lt_b() {
    let a = 0..1000;
    let b = 2000..3000;
    assert_eq!(join(a, b), None);
}

#[test]
fn join_non_overlapping_b_lt_a() {
    let a = 2000..3000;
    let b = 0..1000;
    assert_eq!(join(a, b), None);
}

#[test]
fn join_overlapping_left() {
    let a = 0..2000;
    let b = 1000..3000;
    assert_eq!(join(a, b), Some(0..3000));
}

#[test]
fn join_overlapping_right() {
    let a = 1000..3000;
    let b = 0..2000;
    assert_eq!(join(a, b), Some(0..3000));
}

#[test]
fn join_touching() {
    let a = 0..1500;
    let b = 1500..3000;
    assert_eq!(join(a, b), Some(0..3000));
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cut<T> {
    pub before: Option<Range<T>>,
    pub excluded: Range<T>,
    pub after: Option<Range<T>>,
}

/// Cut a region out of another region, returning the portions of the original
/// `region` before and after as well as the region matching `exclude`.
pub fn cut<T: Ord + Clone>(region: Range<T>, exclude: Range<T>)
    -> Option<Cut<T>> {

    if overlaps(&region, &exclude) {
        let excluded =
            cmp::max(&region.start, &exclude.start).clone() ..
            cmp::min(&region.end, &exclude.end).clone();

        let before = (excluded.start > region.start)
            .then(|| region.start.clone() .. excluded.start.clone());

        let after = (region.end > excluded.end)
            .then(|| excluded.end.clone() .. region.end.clone());

        Some(Cut {
            before,
            excluded,
            after
        })
    } else {
        // non-overlapping
        None
    }
}

#[test]
fn cut_encompassing() {
    let a = 1500..3000;
    let b = 0..6000;
    assert_eq!(cut(a, b), Some(Cut {
        before: None,
        excluded: 1500..3000,
        after: None
    }));
}

#[test]
fn cut_exact_match() {
    let a = 1500..3000;
    let b = 1500..3000;
    assert_eq!(cut(a, b), Some(Cut {
        before: None,
        excluded: 1500..3000,
        after: None
    }));
}

#[test]
fn cut_beginning() {
    let a = 0..6000;
    let b = 0..1500;
    assert_eq!(cut(a, b), Some(Cut {
        before: None,
        excluded: 0..1500,
        after: Some(1500..6000),
    }));
}

#[test]
fn cut_end() {
    let a = 0..6000;
    let b = 4500..6000;
    assert_eq!(cut(a, b), Some(Cut {
        before: Some(0..4500),
        excluded: 4500..6000,
        after: None
    }));
}

#[test]
fn cut_inner() {
    let a = 0..6000;
    let b = 3000..4500;
    assert_eq!(cut(a, b), Some(Cut {
        before: Some(0..3000),
        excluded: 3000..4500,
        after: Some(4500..6000),
    }));
}

#[test]
fn cut_touching() {
    let a = 0..1500;
    let b = 1500..3000;
    assert_eq!(cut(a, b), None);
}

#[test]
fn cut_disjunct() {
    let a = 0..1500;
    let b = 3000..4500;
    assert_eq!(cut(a, b), None);
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
