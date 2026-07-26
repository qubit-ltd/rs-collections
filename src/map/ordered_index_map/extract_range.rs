// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Owning iterator for extracting an ordered range.

use std::collections::hash_map::RandomState;
use std::hash::{
    BuildHasher,
    Hash,
};

use super::{
    OrderedIndexMap,
    OwnedEntry,
    SequenceBounds,
};

/// An iterator that removes and yields records within an ordered range.
///
/// The iterator is double-ended. Records not yet yielded remain in the map if
/// the iterator is dropped early.
///
/// # Panics
///
/// Advancing the iterator panics if the owner map is poisoned or
/// user-provided hashing, ordering, cloning, or destruction code panics. A
/// panic after removal starts poisons the map.
///
/// # Type Parameters
///
/// * `K` - Primary key type.
/// * `O` - Ordered secondary key type.
/// * `V` - Stored value type.
/// * `S` - Hash builder used by the primary index.
#[must_use = "the iterator removes records only while it is advanced"]
pub struct ExtractRange<'a, K, O, V, S = RandomState> {
    /// Exclusively borrowed owner map.
    pub(super) map: &'a mut OrderedIndexMap<K, O, V, S>,
    /// Expanded bounds over order and stable sequence.
    pub(super) bounds: SequenceBounds<O>,
}

impl<K, O, V, S> Iterator for ExtractRange<'_, K, O, V, S>
where
    K: Hash,
    O: Ord + Clone,
    S: BuildHasher,
{
    type Item = OwnedEntry<K, O, V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.map.assert_healthy();
        let (ordered_key, id) = self
            .map
            .state
            .ordered
            .range((self.bounds.0.as_ref(), self.bounds.1.as_ref()))
            .next()
            .map(|(key, id)| (key.clone(), *id))?;
        let hash = self.map.hash_builder.hash_one(
            &self
                .map
                .state
                .arena
                .get(id)
                .expect("ordered index must reference an occupied arena slot")
                .key,
        );
        Some(self.map.remove_slot(hash, id, Some(ordered_key)))
    }
}

impl<K, O, V, S> DoubleEndedIterator for ExtractRange<'_, K, O, V, S>
where
    K: Hash,
    O: Ord + Clone,
    S: BuildHasher,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.map.assert_healthy();
        let (ordered_key, id) = self
            .map
            .state
            .ordered
            .range((self.bounds.0.as_ref(), self.bounds.1.as_ref()))
            .next_back()
            .map(|(key, id)| (key.clone(), *id))?;
        let hash = self.map.hash_builder.hash_one(
            &self
                .map
                .state
                .arena
                .get(id)
                .expect("ordered index must reference an occupied arena slot")
                .key,
        );
        Some(self.map.remove_slot(hash, id, Some(ordered_key)))
    }
}
