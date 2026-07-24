// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Reusable private storage addressed by stable internal slot identifiers.

use crate::internal::SlotId;

/// Stores values in reusable slots whose identifiers never leave the owner map.
///
/// # Type Parameters
///
/// * `T` - Value stored in each occupied slot.
#[derive(Clone, Debug)]
pub(crate) struct EntryArena<T> {
    /// Occupied and reusable vacant slots.
    slots: Vec<Option<T>>,
    /// Vacant slot indexes available for reuse.
    free_slots: Vec<usize>,
    /// Number of occupied slots.
    len: usize,
}

impl<T> EntryArena<T> {
    /// Creates an empty arena.
    ///
    /// # Returns
    ///
    /// An arena without allocated slots.
    #[must_use]
    #[inline]
    pub(crate) const fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_slots: Vec::new(),
            len: 0,
        }
    }

    /// Creates an empty arena with space for at least `capacity` values.
    ///
    /// # Parameters
    ///
    /// * `capacity` - Number of values to accommodate without reallocating.
    ///
    /// # Returns
    ///
    /// An empty arena with reserved slot storage.
    #[must_use]
    #[inline]
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: Vec::with_capacity(capacity),
            free_slots: Vec::new(),
            len: 0,
        }
    }

    /// Returns the number of occupied slots.
    ///
    /// # Returns
    ///
    /// The number of stored values.
    #[must_use]
    #[inline(always)]
    pub(crate) const fn len(&self) -> usize {
        self.len
    }

    /// Returns the number of values accepted without growing slot storage.
    ///
    /// # Returns
    ///
    /// The slot vector capacity.
    #[must_use]
    #[inline(always)]
    pub(crate) fn capacity(&self) -> usize {
        self.slots.capacity()
    }

    /// Reserves capacity for at least `additional` more occupied values.
    ///
    /// # Parameters
    ///
    /// * `additional` - Additional values expected beyond the current length.
    ///
    /// # Panics
    ///
    /// Panics when the requested capacity overflows or allocation fails.
    #[inline]
    pub(crate) fn reserve(&mut self, additional: usize) {
        if additional > self.free_slots.len() {
            self.slots.reserve(additional - self.free_slots.len());
        }
    }

    /// Inserts a value into a vacant or newly appended slot.
    ///
    /// # Parameters
    ///
    /// * `value` - Value owned by the arena.
    ///
    /// # Returns
    ///
    /// The internal identifier assigned to the occupied slot.
    #[must_use]
    pub(crate) fn insert(&mut self, value: T) -> SlotId {
        let slot = if let Some(slot) = self.free_slots.pop() {
            assert!(
                self.slots[slot].is_none(),
                "free arena slot must be vacant"
            );
            self.slots[slot] = Some(value);
            slot
        } else {
            let slot = self.slots.len();
            self.slots.push(Some(value));
            slot
        };
        self.len += 1;
        SlotId(slot)
    }

    /// Returns a shared value reference for an occupied slot.
    ///
    /// # Parameters
    ///
    /// * `id` - Internal slot identifier.
    ///
    /// # Returns
    ///
    /// `Some(value)` for an occupied slot, or `None` for a vacant slot.
    ///
    /// # Panics
    ///
    /// Panics when `id` is outside the private arena, which indicates an
    /// internal index invariant violation.
    #[must_use]
    #[inline(always)]
    pub(crate) fn get(&self, id: SlotId) -> Option<&T> {
        self.slots[id.0].as_ref()
    }

    /// Returns an exclusive value reference for an occupied slot.
    ///
    /// # Parameters
    ///
    /// * `id` - Internal slot identifier.
    ///
    /// # Returns
    ///
    /// `Some(value)` for an occupied slot, or `None` for a vacant slot.
    ///
    /// # Panics
    ///
    /// Panics when `id` is outside the private arena, which indicates an
    /// internal index invariant violation.
    #[must_use]
    #[inline(always)]
    pub(crate) fn get_mut(&mut self, id: SlotId) -> Option<&mut T> {
        self.slots[id.0].as_mut()
    }

    /// Removes and returns one occupied value.
    ///
    /// # Parameters
    ///
    /// * `id` - Internal slot identifier to vacate.
    ///
    /// # Returns
    ///
    /// The removed value, or `None` for a vacant slot.
    ///
    /// # Panics
    ///
    /// Panics when `id` is outside the private arena, which indicates an
    /// internal index invariant violation.
    pub(crate) fn remove(&mut self, id: SlotId) -> Option<T> {
        let value = self.slots[id.0].take()?;
        self.free_slots.push(id.0);
        self.len -= 1;
        Some(value)
    }

    /// Removes all values and releases reusable slot metadata.
    ///
    /// Allocated slot capacity is retained.
    ///
    /// # Panics
    ///
    /// Panics when dropping a stored value panics.
    #[inline]
    pub(crate) fn clear(&mut self) {
        self.slots.clear();
        self.free_slots.clear();
        self.len = 0;
    }
}
