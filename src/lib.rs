//! # tynyptr
//!
//! This crate provides a production-ready dynamic dereference table for “tiny pointers”.
//! Rather than using full machine pointers (which require Ω(log n) bits) our library represents
//! pointers as compact indices into a table. The table automatically resizes (doubling capacity)
//! when needed. All operations (allocation, lookup, free) run in constant time on average.
//!
//! For a full discussion, see [Bender et al., Tiny Pointers](https://arxiv.org/pdf/2111.12800).

pub mod dynamic_table;
