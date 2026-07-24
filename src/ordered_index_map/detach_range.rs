// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Lending cursor for detaching an ordered range.

use std::collections::hash_map::RandomState;
use std::hash::{
    BuildHasher,
    Hash,
};
use std::ops::Bound;

use crate::DetachedEntryMut;
use crate::internal::{
    Sequence,
    SlotId,
};

use super::{
    OrderedIndexMap,
    sequence_bounds,
};

/// A lending cursor that detaches records within an ordered range.
///
/// Unlike [`Iterator`], the returned entry borrows from the cursor, preventing
/// another cursor operation until that entry view is released.
#[must_use = "the cursor detaches records only while it is advanced"]
pub struct DetachRange<'a, K, O, V, S = RandomState> {
    /// Exclusively borrowed owner map.
    pub(super) map: &'a mut OrderedIndexMap<K, O, V, S>,
    /// Owned lower range bound.
    pub(super) start: Bound<O>,
    /// Owned upper range bound.
    pub(super) end: Bound<O>,
}

impl<K, O, V, S> DetachRange<'_, K, O, V, S>
where
    K: Eq + Hash,
    O: Ord + Clone,
    S: BuildHasher,
{
    /// Detaches and returns the first remaining record in the range.
    #[allow(
        clippy::should_implement_trait,
        reason = "a lending cursor cannot implement Iterator"
    )]
    pub fn next(&mut self) -> Option<DetachedEntryMut<'_, K, O, V>> {
        self.map.assert_healthy();
        let (ordered_key, id) = self.first_candidate()?;
        self.map.detach_slot(id, ordered_key);
        Some(self.map.detached_entry_mut(id))
    }

    /// Detaches and returns the last remaining record in the range.
    pub fn next_back(&mut self) -> Option<DetachedEntryMut<'_, K, O, V>> {
        self.map.assert_healthy();
        let (ordered_key, id) = self.last_candidate()?;
        self.map.detach_slot(id, ordered_key);
        Some(self.map.detached_entry_mut(id))
    }

    /// Reports whether no attached record remains within the range.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.assert_healthy();
        self.map
            .state
            .ordered
            .range(sequence_bounds(&self.start, &self.end))
            .next()
            .is_none()
    }

    /// Finds and copies the first removable ordered-index reference.
    fn first_candidate(&self) -> Option<((O, Sequence), SlotId)> {
        self.map
            .state
            .ordered
            .range(sequence_bounds(&self.start, &self.end))
            .next()
            .map(|(key, id)| (key.clone(), *id))
    }

    /// Finds and copies the last removable ordered-index reference.
    fn last_candidate(&self) -> Option<((O, Sequence), SlotId)> {
        self.map
            .state
            .ordered
            .range(sequence_bounds(&self.start, &self.end))
            .next_back()
            .map(|(key, id)| (key.clone(), *id))
    }
}
