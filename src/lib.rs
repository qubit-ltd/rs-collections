// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! # qubit-collections
//!
//! Focused collection types with explicitly maintained secondary indexes.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod detached_entry_mut;
mod entry_mut;
mod entry_ref;
mod index_state;
mod internal;
pub mod ordered_index_map;
mod owned_entry;

pub use detached_entry_mut::DetachedEntryMut;
pub use entry_mut::EntryMut;
pub use entry_ref::EntryRef;
pub use index_state::IndexState;
pub use ordered_index_map::{
    DetachRange,
    ExtractRange,
    OrderedIndexMap,
};
pub use owned_entry::OwnedEntry;
