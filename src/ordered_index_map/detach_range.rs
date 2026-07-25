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

use crate::DetachedEntryMut;
use crate::internal::{
    Sequence,
    SlotId,
};

use super::{
    OrderedIndexMap,
    SequenceBounds,
};

/// A lending cursor that detaches records within an ordered range.
///
/// Unlike [`Iterator`], the returned entry borrows from the cursor, preventing
/// another cursor operation until that entry view is released.
///
/// # Type Parameters
///
/// * `K` - Primary key type.
/// * `O` - Ordered secondary key type.
/// * `V` - Stored value type.
/// * `S` - Hash builder used by the primary index.
#[must_use = "the cursor detaches records only while it is advanced"]
pub struct DetachRange<'a, K, O, V, S = RandomState> {
    /// Exclusively borrowed owner map.
    pub(super) map: &'a mut OrderedIndexMap<K, O, V, S>,
    /// Expanded bounds over order and stable sequence.
    pub(super) bounds: SequenceBounds<O>,
}

impl<K, O, V, S> DetachRange<'_, K, O, V, S>
where
    O: Ord + Clone,
{
    /// Detaches and returns the first remaining record in the range.
    #[allow(
        clippy::should_implement_trait,
        reason = "a lending cursor cannot implement Iterator"
    )]
    ///
    /// # Returns
    ///
    /// Exclusive value access for the detached record, or `None` when no
    /// matching attached record remains.
    ///
    /// # Panics
    ///
    /// Panics if the owner map is poisoned.
    pub fn next(&mut self) -> Option<DetachedEntryMut<'_, K, O, V>> {
        self.map.assert_healthy();
        let (ordered_key, id) = self.first_candidate()?;
        self.map.detach_slot(id, ordered_key);
        Some(self.map.detached_entry_mut(id))
    }

    /// Detaches and returns the last remaining record in the range.
    ///
    /// # Returns
    ///
    /// Exclusive value access for the detached record, or `None` when no
    /// matching attached record remains.
    ///
    /// # Panics
    ///
    /// Panics if the owner map is poisoned.
    pub fn next_back(&mut self) -> Option<DetachedEntryMut<'_, K, O, V>> {
        self.map.assert_healthy();
        let (ordered_key, id) = self.last_candidate()?;
        self.map.detach_slot(id, ordered_key);
        Some(self.map.detached_entry_mut(id))
    }

    /// Reports whether no attached record remains within the range.
    ///
    /// # Returns
    ///
    /// `true` when advancing from either end would return `None`.
    ///
    /// # Panics
    ///
    /// Panics if the owner map is poisoned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.assert_healthy();
        self.map
            .state
            .ordered
            .range((self.bounds.0.as_ref(), self.bounds.1.as_ref()))
            .next()
            .is_none()
    }

    /// Finds and copies the first removable ordered-index reference.
    fn first_candidate(&self) -> Option<((O, Sequence), SlotId)> {
        self.map
            .state
            .ordered
            .range((self.bounds.0.as_ref(), self.bounds.1.as_ref()))
            .next()
            .map(|(key, id)| (key.clone(), *id))
    }

    /// Finds and copies the last removable ordered-index reference.
    fn last_candidate(&self) -> Option<((O, Sequence), SlotId)> {
        self.map
            .state
            .ordered
            .range((self.bounds.0.as_ref(), self.bounds.1.as_ref()))
            .next_back()
            .map(|(key, id)| (key.clone(), *id))
    }
}
