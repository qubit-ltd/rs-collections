// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Consistent storage updated inside one poisoning transaction.

use std::collections::BTreeMap;

use hashbrown::HashTable;

use crate::internal::{
    EntryArena,
    Record,
    Sequence,
    SlotId,
};

/// Owns the arena and both indexes for an ordered index map.
///
/// # Type Parameters
///
/// * `K` - Primary key type.
/// * `O` - Ordered secondary key type.
/// * `V` - Stored value type.
#[derive(Clone, Debug)]
pub(crate) struct InternalState<K, O, V> {
    /// Single owning storage for primary records.
    pub(crate) arena: EntryArena<Record<K, O, V>>,
    /// Hash index containing only arena identifiers.
    pub(crate) primary: HashTable<SlotId>,
    /// Ordered index from secondary key and sequence to arena identifier.
    pub(crate) ordered: BTreeMap<(O, Sequence), SlotId>,
    /// Next stable sequence assigned by insertion or attachment.
    pub(crate) next_sequence: u64,
    /// Number of records participating in the ordered index.
    pub(crate) attached_len: usize,
}

impl<K, O, V> InternalState<K, O, V> {
    /// Creates empty internal storage.
    ///
    /// # Returns
    ///
    /// Empty arena and indexes.
    #[must_use]
    #[inline]
    pub(crate) const fn new() -> Self {
        Self {
            arena: EntryArena::new(),
            primary: HashTable::new(),
            ordered: BTreeMap::new(),
            next_sequence: 0,
            attached_len: 0,
        }
    }

    /// Creates empty internal storage with primary capacity.
    ///
    /// # Parameters
    ///
    /// * `capacity` - Number of primary records to reserve.
    ///
    /// # Returns
    ///
    /// Empty storage with reserved arena and hash-table capacity.
    #[must_use]
    #[inline]
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: EntryArena::with_capacity(capacity),
            primary: HashTable::with_capacity(capacity),
            ordered: BTreeMap::new(),
            next_sequence: 0,
            attached_len: 0,
        }
    }

    /// Allocates the next stable attachment sequence.
    ///
    /// # Returns
    ///
    /// A sequence not previously assigned since construction or clear.
    ///
    /// # Panics
    ///
    /// Panics when every supported sequence has been allocated.
    #[must_use]
    #[inline]
    pub(crate) fn allocate_sequence(&mut self) -> Sequence {
        let sequence = Sequence(self.next_sequence);
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .expect("ordered index attachment sequences exhausted");
        sequence
    }
}
