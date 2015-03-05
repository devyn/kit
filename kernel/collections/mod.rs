/*******************************************************************************
 *
 * kit/kernel/collections/mod.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

//! Aggregate data storage containers (i.e., collections).
//!
//! Some are more optimal for certain use cases than others.

pub mod treemap;

pub use self::treemap::TreeMap;
