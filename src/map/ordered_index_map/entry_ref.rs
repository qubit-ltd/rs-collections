// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared view of one primary record.

use super::IndexState;

/// Shared key, order, value, and attachment state for one record.
///
/// Copying or cloning this view copies only its shared references and state; it
/// does not clone or allocate the referenced record data. Each reference keeps
/// the lifetime of the borrowed map data.
///
/// # Type Parameters
///
/// * `K` - Primary key type.
/// * `O` - Ordered secondary key type.
/// * `V` - Stored value type.
///
/// # Examples
///
/// ```
/// use qubit_collections::map::OrderedIndexMap;
///
/// let mut map = OrderedIndexMap::new();
/// map.insert("job", 10, "ready");
/// let entry = map.get_entry("job").expect("the entry was just inserted");
/// assert_eq!("job", *entry.key());
/// assert_eq!(10, *entry.order());
/// assert_eq!("ready", *entry.value());
/// assert!(entry.is_attached());
/// ```
#[derive(Debug)]
#[must_use = "the entry view provides access to a stored record"]
pub struct EntryRef<'a, K, O, V> {
    /// Key used to find this record in the primary map.
    pub(crate) key: &'a K,
    /// Order key retained by the record, including while it is detached.
    pub(crate) order: &'a O,
    /// Value associated with this record in the primary map.
    pub(crate) value: &'a V,
    /// Whether the ordered secondary index currently includes this record.
    pub(crate) state: IndexState,
}

impl<K, O, V> Copy for EntryRef<'_, K, O, V> {}

impl<K, O, V> Clone for EntryRef<'_, K, O, V> {
    /// Copies the shared references and attachment state without cloning
    /// records.
    ///
    /// # Returns
    ///
    /// A view of the same record with the original reference lifetimes.
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, K, O, V> EntryRef<'a, K, O, V> {
    /// Returns the primary key.
    ///
    /// # Returns
    ///
    /// The key retained by the primary record.
    #[must_use]
    #[inline]
    pub const fn key(&self) -> &'a K {
        self.key
    }

    /// Returns the retained ordered key.
    ///
    /// # Returns
    ///
    /// The secondary key, including while the record is detached.
    #[must_use]
    #[inline]
    pub const fn order(&self) -> &'a O {
        self.order
    }

    /// Returns the stored value.
    ///
    /// # Returns
    ///
    /// A shared reference to the record's value.
    #[must_use]
    #[inline]
    pub const fn value(&self) -> &'a V {
        self.value
    }

    /// Returns the ordered-index attachment state.
    ///
    /// # Returns
    ///
    /// Whether ordered operations can currently observe this record.
    #[must_use = "the attachment state describes ordered visibility"]
    #[inline]
    pub const fn state(&self) -> IndexState {
        self.state
    }

    /// Reports whether this record is attached to the ordered index.
    ///
    /// # Returns
    ///
    /// `true` when ordered operations can observe this record.
    #[must_use]
    #[inline]
    pub const fn is_attached(&self) -> bool {
        matches!(self.state, IndexState::Attached)
    }

    /// Reports whether this record is detached from the ordered index.
    ///
    /// # Returns
    ///
    /// `true` when only primary-key operations can observe this record.
    #[must_use]
    #[inline]
    pub const fn is_detached(&self) -> bool {
        matches!(self.state, IndexState::Detached)
    }
}
