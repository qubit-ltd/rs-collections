// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Exclusive value view of one primary record.

use super::IndexState;

/// Shared key and order plus exclusive value access for one record.
///
/// # Type Parameters
///
/// * `K` - Primary key type.
/// * `O` - Ordered secondary key type.
/// * `V` - Stored value type.
#[derive(Debug)]
#[must_use = "the entry view provides exclusive access to a stored value"]
pub struct EntryMut<'a, K, O, V> {
    /// Primary key.
    pub(crate) key: &'a K,
    /// Retained ordered key.
    pub(crate) order: &'a O,
    /// Stored value.
    pub(crate) value: &'a mut V,
    /// Current ordered-index attachment state.
    pub(crate) state: IndexState,
}

impl<'a, K, O, V> EntryMut<'a, K, O, V> {
    /// Returns the primary key.
    ///
    /// # Returns
    ///
    /// The key retained by the primary record.
    #[must_use]
    #[inline(always)]
    pub const fn key(&self) -> &K {
        self.key
    }

    /// Returns the retained ordered key.
    ///
    /// # Returns
    ///
    /// The secondary key, including while the record is detached.
    #[must_use]
    #[inline(always)]
    pub const fn order(&self) -> &O {
        self.order
    }

    /// Returns the stored value.
    ///
    /// # Returns
    ///
    /// A shared reference to the record's value.
    #[must_use]
    #[inline(always)]
    pub const fn value(&self) -> &V {
        self.value
    }

    /// Returns exclusive access to the stored value.
    ///
    /// # Returns
    ///
    /// A mutable reference that cannot change the indexed key or order.
    #[must_use]
    #[inline(always)]
    pub fn value_mut(&mut self) -> &mut V {
        self.value
    }

    /// Consumes the view and returns exclusive access to the stored value.
    ///
    /// # Returns
    ///
    /// The mutable value reference with the view's original lifetime.
    #[must_use]
    #[inline(always)]
    pub fn into_value_mut(self) -> &'a mut V {
        self.value
    }

    /// Returns the ordered-index attachment state.
    ///
    /// # Returns
    ///
    /// Whether ordered operations can currently observe this record.
    #[must_use = "the attachment state describes ordered visibility"]
    #[inline(always)]
    pub const fn state(&self) -> IndexState {
        self.state
    }

    /// Reports whether this record is attached to the ordered index.
    ///
    /// # Returns
    ///
    /// `true` when ordered operations can observe this record.
    #[must_use]
    #[inline(always)]
    pub const fn is_attached(&self) -> bool {
        matches!(self.state, IndexState::Attached)
    }

    /// Reports whether this record is detached from the ordered index.
    ///
    /// # Returns
    ///
    /// `true` when only primary-key operations can observe this record.
    #[must_use]
    #[inline(always)]
    pub const fn is_detached(&self) -> bool {
        matches!(self.state, IndexState::Detached)
    }
}
