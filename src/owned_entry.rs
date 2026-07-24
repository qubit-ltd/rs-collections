// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Owned record returned by replacement and removal operations.

use crate::IndexState;

/// Owned key, order, value, and prior attachment state for one record.
///
/// # Type Parameters
///
/// * `K` - Primary key type.
/// * `O` - Ordered secondary key type.
/// * `V` - Stored value type.
#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use = "the owned entry contains a removed or replaced record"]
pub struct OwnedEntry<K, O, V> {
    /// Primary key.
    pub(crate) key: K,
    /// Retained ordered key.
    pub(crate) order: O,
    /// Stored value.
    pub(crate) value: V,
    /// Attachment state before removal.
    pub(crate) state: IndexState,
}

impl<K, O, V> OwnedEntry<K, O, V> {
    /// Returns the primary key.
    ///
    /// # Returns
    ///
    /// A shared reference to the owned key.
    #[must_use]
    #[inline(always)]
    pub const fn key(&self) -> &K {
        &self.key
    }

    /// Returns the retained ordered key.
    ///
    /// # Returns
    ///
    /// A shared reference to the owned order.
    #[must_use]
    #[inline(always)]
    pub const fn order(&self) -> &O {
        &self.order
    }

    /// Returns the stored value.
    ///
    /// # Returns
    ///
    /// A shared reference to the owned value.
    #[must_use]
    #[inline(always)]
    pub const fn value(&self) -> &V {
        &self.value
    }

    /// Returns exclusive access to the stored value.
    ///
    /// # Returns
    ///
    /// A mutable reference to the owned value.
    #[must_use]
    #[inline(always)]
    pub fn value_mut(&mut self) -> &mut V {
        &mut self.value
    }

    /// Returns the attachment state before removal.
    ///
    /// # Returns
    ///
    /// The record's state immediately before it left the map.
    #[must_use = "the attachment state describes the record before removal"]
    #[inline(always)]
    pub const fn state(&self) -> IndexState {
        self.state
    }

    /// Consumes the entry and returns its primary key.
    ///
    /// The order and value are dropped.
    ///
    /// # Returns
    ///
    /// The owned primary key.
    #[must_use]
    #[inline]
    pub fn into_key(self) -> K {
        self.key
    }

    /// Consumes the entry and returns its order.
    ///
    /// The key and value are dropped.
    ///
    /// # Returns
    ///
    /// The owned ordered key.
    #[must_use]
    #[inline]
    pub fn into_order(self) -> O {
        self.order
    }

    /// Consumes the entry and returns its value.
    ///
    /// The key and order are dropped.
    ///
    /// # Returns
    ///
    /// The owned stored value.
    #[must_use]
    #[inline]
    pub fn into_value(self) -> V {
        self.value
    }

    /// Splits the entry into every owned component.
    ///
    /// # Returns
    ///
    /// The primary key, order, value, and prior attachment state.
    #[must_use = "the returned components contain the complete removed record"]
    #[inline]
    pub fn into_parts(self) -> (K, O, V, IndexState) {
        (self.key, self.order, self.value, self.state)
    }
}
