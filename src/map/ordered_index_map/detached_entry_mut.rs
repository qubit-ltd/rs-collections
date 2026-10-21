// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Exclusive value view returned after detaching a record.

/// Shared key and order plus exclusive value access for a detached record.
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
/// map.try_insert("job", 10, "ready")?;
/// let mut detached = map.detach("job").expect("job is attached");
/// assert_eq!("job", *detached.key());
/// assert_eq!(10, *detached.order());
/// *detached.value_mut() = "running";
/// assert_eq!("running", *detached.value());
/// # Ok::<(), qubit_collections::map::ordered_index_map::TryInsertError<&str, i32, &str>>(())
/// ```
#[derive(Debug)]
#[must_use = "the detached entry view provides exclusive access to a retained value"]
pub struct DetachedEntryMut<'a, K, O, V> {
    /// Primary key.
    pub(crate) key: &'a K,
    /// Retained ordered key.
    pub(crate) order: &'a O,
    /// Stored value.
    pub(crate) value: &'a mut V,
}

impl<'a, K, O, V> DetachedEntryMut<'a, K, O, V> {
    /// Returns the primary key.
    ///
    /// # Returns
    ///
    /// The key retained by the detached primary record.
    #[must_use]
    #[inline]
    pub const fn key(&self) -> &K {
        self.key
    }

    /// Returns the retained ordered key.
    ///
    /// # Returns
    ///
    /// The order that can be reused by a later attachment.
    #[must_use]
    #[inline]
    pub const fn order(&self) -> &O {
        self.order
    }

    /// Returns the stored value.
    ///
    /// # Returns
    ///
    /// A shared reference to the retained value.
    #[must_use]
    #[inline]
    pub const fn value(&self) -> &V {
        self.value
    }

    /// Returns exclusive access to the stored value.
    ///
    /// # Returns
    ///
    /// A mutable reference that cannot change the retained key or order.
    #[must_use]
    #[inline]
    pub fn value_mut(&mut self) -> &mut V {
        self.value
    }

    /// Consumes the view and returns exclusive access to the stored value.
    ///
    /// # Returns
    ///
    /// The mutable value reference with the view's original lifetime.
    #[must_use]
    #[inline]
    pub fn into_value_mut(self) -> &'a mut V {
        self.value
    }
}
