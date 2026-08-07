// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Primary record stored once in the private arena.

use super::Sequence;
use crate::map::ordered_index_map::IndexState;

/// Owns one primary key, retained order, value, and optional attachment
/// sequence.
///
/// # Type Parameters
///
/// * `K` - Primary key type.
/// * `O` - Ordered secondary key type.
/// * `V` - Stored value type.
#[derive(Clone, Debug)]
pub(crate) struct Record<K, O, V> {
    /// Primary key stored exactly once by the collection.
    pub(crate) key: K,
    /// Ordered key retained in both attachment states.
    pub(crate) order: O,
    /// User value.
    pub(crate) value: V,
    /// Stable sequence while attached, or `None` while detached.
    pub(crate) sequence: Option<Sequence>,
}

impl<K, O, V> Record<K, O, V> {
    /// Returns the public attachment state.
    ///
    /// # Returns
    ///
    /// [`IndexState::Attached`] when a sequence is present.
    #[must_use = "the attachment state describes ordered visibility"]
    #[inline(always)]
    pub(crate) const fn state(&self) -> IndexState {
        if self.sequence.is_some() {
            IndexState::Attached
        } else {
            IndexState::Detached
        }
    }
}
