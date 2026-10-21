// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Attachment state of a primary record's ordered index.

/// Describes whether a primary record participates in ordered operations.
///
/// # Examples
///
/// ```
/// use qubit_collections::map::ordered_index_map::{IndexState, OrderedIndexMap};
///
/// let mut map = OrderedIndexMap::new();
/// map.insert("job", 10, "ready");
/// drop(map.detach("job").expect("the job was just inserted"));
/// let entry = map.get_entry("job").expect("detaching retains the record");
/// assert_eq!(IndexState::Detached, entry.state());
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[must_use = "the state determines whether ordered operations can observe a record"]
pub enum IndexState {
    /// The record participates in the ordered secondary index.
    Attached,
    /// The record remains in the primary map but not in the ordered index.
    Detached,
}
