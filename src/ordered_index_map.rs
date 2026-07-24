// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! A hash map with an independently managed ordered secondary index.

use crate::internal::OrderedIndexEntry;
use std::borrow::Borrow;
use std::collections::{
    BTreeMap,
    HashMap,
};
use std::hash::Hash;

/// A primary-key map with an ordered secondary index.
///
/// Entries are addressable by a unique primary key `K` and ordered by a
/// possibly non-unique secondary key `O`. Equal secondary keys retain insertion
/// order. An entry may be removed from the ordered index while remaining
/// addressable in the primary map.
///
/// It is a logic error for a stored primary key's [`Hash`] or [`Eq`] behavior,
/// or an indexed order key's [`Ord`] behavior, to change while it is in the
/// map. This includes changes through interior mutability. As with the standard
/// map types, the resulting behavior is encapsulated to this instance and may
/// include panics, incorrect results, or non-termination, but not undefined
/// behavior.
///
/// This type provides no internal synchronization. Callers that share a map
/// between threads must protect it with an appropriate synchronization
/// primitive.
///
/// # Panic and Unwind Safety
///
/// Updating two independent indexes cannot provide strong unwind safety when
/// user implementations of [`Hash`], [`Eq`], [`Ord`], or [`Clone`] panic. A
/// mutation therefore marks the map as poisoned before its first index update
/// and clears that marker only after both indexes are consistent. If the
/// mutation unwinds, later collection operations panic instead of observing a
/// partially updated index. Discard the poisoned map; replacing it with
/// [`Default::default`] remains safe.
///
/// # Type Parameters
///
/// * `K` - Unique primary key type.
/// * `O` - Ordered secondary key type.
/// * `V` - Stored value type.
///
/// # Examples
///
/// ```
/// use qubit_collections::OrderedIndexMap;
///
/// let mut deadlines = OrderedIndexMap::new();
/// deadlines.insert("later", 20, "second");
/// deadlines.insert("earlier", 10, "first");
///
/// assert_eq!(Some((&"earlier", &10, &"first")), deadlines.first());
/// assert_eq!(vec!["earlier"], deadlines.unindex_through(&10));
/// assert_eq!(Some(&"first"), deadlines.get("earlier"));
/// assert_eq!(Some((&"later", &20, &"second")), deadlines.first());
/// ```
#[derive(Debug)]
#[must_use = "the map owns its entries and ordered index"]
pub struct OrderedIndexMap<K, O, V> {
    /// Primary records addressed by unique keys.
    entries: HashMap<K, OrderedIndexEntry<O, V>>,
    /// Indexed keys ordered by secondary key and stable insertion sequence.
    ordered_keys: BTreeMap<(O, u64), K>,
    /// Next stable insertion sequence.
    next_sequence: u64,
    /// Whether a prior mutation panicked after a cross-index update began.
    poisoned: bool,
}

impl<K, O, V> OrderedIndexMap<K, O, V> {
    /// Creates an empty map.
    ///
    /// # Returns
    ///
    /// An empty map with no primary records or ordered entries.
    #[inline]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            ordered_keys: BTreeMap::new(),
            next_sequence: 0,
            poisoned: false,
        }
    }

    /// Returns the number of primary records, including unindexed entries.
    ///
    /// # Returns
    ///
    /// The number of values addressable by primary key.
    ///
    /// # Panics
    ///
    /// Panics when a previous mutation poisoned the map.
    #[must_use]
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.assert_healthy();
        self.entries.len()
    }

    /// Reports whether the primary map contains no records.
    ///
    /// # Returns
    ///
    /// `true` when no values are addressable by primary key.
    ///
    /// # Panics
    ///
    /// Panics when a previous mutation poisoned the map.
    #[must_use]
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.assert_healthy();
        self.entries.is_empty()
    }

    /// Returns the number of records currently in the ordered index.
    ///
    /// # Returns
    ///
    /// The number of records visible to ordered operations.
    ///
    /// # Panics
    ///
    /// Panics when a previous mutation poisoned the map.
    #[must_use]
    #[inline(always)]
    pub fn indexed_len(&self) -> usize {
        self.assert_healthy();
        self.ordered_keys.len()
    }

    /// Reports whether the ordered index contains no records.
    ///
    /// Unindexed primary records do not affect this result.
    ///
    /// # Returns
    ///
    /// `true` when ordered operations cannot observe an entry.
    ///
    /// # Panics
    ///
    /// Panics when a previous mutation poisoned the map.
    #[must_use]
    #[inline(always)]
    pub fn is_index_empty(&self) -> bool {
        self.assert_healthy();
        self.ordered_keys.is_empty()
    }

    /// Removes every primary record and ordered-index entry.
    ///
    /// The stable insertion sequence is reset because no prior entry remains.
    ///
    /// # Panics
    ///
    /// Panics when a previous mutation poisoned the map or when dropping a
    /// stored key, order key, or value panics. A panic during clearing poisons
    /// the map.
    #[inline]
    pub fn clear(&mut self) {
        self.assert_healthy();
        self.poisoned = true;
        self.entries.clear();
        self.ordered_keys.clear();
        self.next_sequence = 0;
        self.poisoned = false;
    }

    /// Requires that no earlier cross-index mutation unwound.
    ///
    /// # Panics
    ///
    /// Panics when a previous mutation panicked after it began changing an
    /// internal index.
    #[inline(always)]
    fn assert_healthy(&self) {
        assert!(
            !self.poisoned,
            "OrderedIndexMap is poisoned after a prior mutation panic",
        );
    }
}

impl<K, O, V> OrderedIndexMap<K, O, V>
where
    K: Eq + Hash + Clone,
    O: Ord + Clone,
{
    /// Inserts or replaces one primary record and indexes its order key.
    ///
    /// Replacing an existing primary key assigns a new stable insertion
    /// sequence, even when the secondary key is unchanged.
    ///
    /// # Parameters
    ///
    /// * `key` - Unique primary key.
    /// * `order_key` - Secondary key used by ordered operations.
    /// * `value` - Value owned by the map.
    ///
    /// # Returns
    ///
    /// The previous secondary key and value for `key`, or `None` when the key
    /// was not present.
    ///
    /// # Panics
    ///
    /// Panics when the map is poisoned, after all supported stable insertion
    /// sequences are exhausted, when a key trait operation panics, or when an
    /// internal index invariant has been violated. A panic after index
    /// mutation begins poisons the map.
    pub fn insert(&mut self, key: K, order_key: O, value: V) -> Option<(O, V)> {
        self.assert_healthy();
        let sequence = self.allocate_sequence();
        let previous = self.remove(&key);
        self.poisoned = true;
        let previous_ordered_key = self
            .ordered_keys
            .insert((order_key.clone(), sequence), key.clone());
        assert!(
            previous_ordered_key.is_none(),
            "ordered index sequence must be unique",
        );
        let previous_entry = self.entries.insert(
            key,
            OrderedIndexEntry {
                order_key,
                sequence: Some(sequence),
                value,
            },
        );
        assert!(
            previous_entry.is_none(),
            "replaced primary entry must be removed before insertion",
        );
        self.poisoned = false;
        previous
    }

    /// Reports whether a primary key is present.
    ///
    /// Indexed and unindexed records are both visible.
    ///
    /// # Parameters
    ///
    /// * `key` - Borrowed form of the primary key to query.
    ///
    /// # Returns
    ///
    /// `true` when the primary map contains `key`.
    ///
    /// # Panics
    ///
    /// Panics when the map is poisoned or hashing or comparing `key` panics.
    #[must_use]
    #[inline(always)]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.assert_healthy();
        self.entries.contains_key(key)
    }

    /// Returns the retained secondary key for a primary key.
    ///
    /// The secondary key remains available after the record is unindexed.
    ///
    /// # Parameters
    ///
    /// * `key` - Borrowed form of the primary key to query.
    ///
    /// # Returns
    ///
    /// `Some(order_key)` for an existing primary record, or `None` when the
    /// primary key is absent.
    ///
    /// # Panics
    ///
    /// Panics when the map is poisoned or hashing or comparing `key` panics.
    #[must_use]
    #[inline(always)]
    pub fn order_key<Q>(&self, key: &Q) -> Option<&O>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.assert_healthy();
        self.entries.get(key).map(|entry| &entry.order_key)
    }

    /// Returns a shared reference to a value by primary key.
    ///
    /// Indexed and unindexed records are both visible.
    ///
    /// # Parameters
    ///
    /// * `key` - Borrowed form of the primary key to query.
    ///
    /// # Returns
    ///
    /// `Some(value)` for an existing primary record, or `None` when the key is
    /// absent.
    ///
    /// # Panics
    ///
    /// Panics when the map is poisoned or hashing or comparing `key` panics.
    #[must_use]
    #[inline(always)]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.assert_healthy();
        self.entries.get(key).map(|entry| &entry.value)
    }

    /// Returns an exclusive reference to a value by primary key.
    ///
    /// The secondary key cannot be modified through this reference, preserving
    /// index consistency.
    ///
    /// # Parameters
    ///
    /// * `key` - Borrowed form of the primary key to query.
    ///
    /// # Returns
    ///
    /// `Some(value)` for an existing primary record, or `None` when the key is
    /// absent.
    ///
    /// # Panics
    ///
    /// Panics when the map is poisoned or hashing or comparing `key` panics.
    #[must_use]
    #[inline(always)]
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.assert_healthy();
        self.entries.get_mut(key).map(|entry| &mut entry.value)
    }

    /// Removes one primary record and any matching ordered-index entry.
    ///
    /// # Parameters
    ///
    /// * `key` - Borrowed form of the primary key to remove.
    ///
    /// # Returns
    ///
    /// `Some((order_key, value))` for a removed record, or `None` when the key
    /// was absent.
    ///
    /// # Panics
    ///
    /// Panics when the map is poisoned, a key trait operation panics, an
    /// indexed primary record lacks its matching ordered entry, or the ordered
    /// entry points to a different primary key. A panic after removal begins
    /// poisons the map.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<(O, V)>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.assert_healthy();
        self.poisoned = true;
        let Some((stored_key, entry)) = self.entries.remove_entry(key) else {
            self.poisoned = false;
            return None;
        };
        if let Some(sequence) = entry.sequence {
            let indexed_key = self
                .ordered_keys
                .remove(&(entry.order_key.clone(), sequence))
                .expect("indexed primary record must have an ordered key");
            assert!(
                indexed_key == stored_key,
                "ordered key must point to its primary record",
            );
        }
        self.poisoned = false;
        Some((entry.order_key, entry.value))
    }

    /// Returns the smallest secondary key in the ordered index.
    ///
    /// Unindexed primary records are ignored.
    ///
    /// # Returns
    ///
    /// `Some(order_key)` for the first indexed record, or `None` when the
    /// ordered index is empty.
    ///
    /// # Panics
    ///
    /// Panics when a previous mutation poisoned the map.
    #[must_use]
    #[inline(always)]
    pub fn first_order_key(&self) -> Option<&O> {
        self.assert_healthy();
        self.ordered_keys
            .first_key_value()
            .map(|((order_key, _sequence), _key)| order_key)
    }

    /// Returns the first indexed primary key, secondary key, and value.
    ///
    /// Equal secondary keys are returned in insertion order.
    ///
    /// # Returns
    /// `Some((key, order_key, value))` for the first indexed record, or `None`
    /// when the ordered index is empty.
    ///
    /// # Panics
    ///
    /// Panics when the map is poisoned, a key trait operation panics, or an
    /// ordered entry lacks its matching primary record.
    #[must_use]
    pub fn first(&self) -> Option<(&K, &O, &V)> {
        self.assert_healthy();
        let ((order_key, sequence), key) =
            self.ordered_keys.first_key_value()?;
        let entry = self
            .entries
            .get(key)
            .expect("ordered key must have a primary record");
        assert!(
            entry.sequence == Some(*sequence) && &entry.order_key == order_key,
            "ordered key metadata must match its primary record",
        );
        Some((key, order_key, &entry.value))
    }

    /// Removes and returns the first indexed record from both views.
    ///
    /// Unindexed primary records are ignored and remain stored.
    ///
    /// # Returns
    ///
    /// `Some((key, order_key, value))` for the removed first record, or `None`
    /// when the ordered index is empty.
    ///
    /// # Panics
    ///
    /// Panics when the map is poisoned, a key trait operation panics, an
    /// ordered entry lacks its matching primary record, or its metadata
    /// disagrees with that record. A panic after removal begins poisons the
    /// map.
    pub fn pop_first(&mut self) -> Option<(K, O, V)> {
        self.assert_healthy();
        self.poisoned = true;
        let Some(((order_key, sequence), key)) = self.ordered_keys.pop_first()
        else {
            self.poisoned = false;
            return None;
        };
        let (_stored_key, entry) = self
            .entries
            .remove_entry(&key)
            .expect("ordered key must have a primary record");
        assert!(
            entry.sequence == Some(sequence) && entry.order_key == order_key,
            "ordered key metadata must match its primary record",
        );
        self.poisoned = false;
        Some((key, order_key, entry.value))
    }

    /// Removes one primary record from the ordered index without deleting it.
    ///
    /// # Parameters
    ///
    /// * `key` - Borrowed form of the primary key to unindex.
    ///
    /// # Returns
    ///
    /// `true` when an indexed record was detached. Returns `false` when the
    /// primary key is absent or already unindexed.
    ///
    /// # Panics
    ///
    /// Panics when the map is poisoned, a key or order-key trait operation
    /// panics, or the primary record lacks its matching ordered entry. A panic
    /// after detachment begins poisons the map.
    pub fn unindex<Q>(&mut self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.assert_healthy();
        let Some((stored_key, entry)) = self.entries.get_key_value(key) else {
            return false;
        };
        let Some(sequence) = entry.sequence else {
            return false;
        };
        let stored_key = stored_key.clone();
        let order_key = entry.order_key.clone();
        self.poisoned = true;
        let indexed_key = self
            .ordered_keys
            .remove(&(order_key, sequence))
            .expect("indexed primary record must have an ordered key");
        assert!(
            indexed_key == stored_key,
            "ordered key must point to its primary record",
        );
        self.entries
            .get_mut(key)
            .expect("unindexed primary record must remain present")
            .sequence = None;
        self.poisoned = false;
        true
    }

    /// Removes the first ordered entry while retaining its primary record.
    ///
    /// # Returns
    ///
    /// `Some((key, order_key))` for the detached first entry, or `None` when
    /// the ordered index is empty.
    ///
    /// # Panics
    ///
    /// Panics when the map is poisoned, a key trait operation panics, an
    /// ordered entry lacks its matching primary record, or its metadata
    /// disagrees with that record. A panic after detachment begins poisons the
    /// map.
    pub fn unindex_first(&mut self) -> Option<(K, O)> {
        self.assert_healthy();
        self.poisoned = true;
        let Some(((order_key, sequence), key)) = self.ordered_keys.pop_first()
        else {
            self.poisoned = false;
            return None;
        };
        let entry = self
            .entries
            .get_mut(&key)
            .expect("ordered key must have a primary record");
        assert!(
            entry.sequence == Some(sequence) && entry.order_key == order_key,
            "ordered key metadata must match its primary record",
        );
        entry.sequence = None;
        self.poisoned = false;
        Some((key, order_key))
    }

    /// Detaches every ordered entry at or below an inclusive upper bound.
    ///
    /// Primary records and their retained secondary keys remain stored.
    /// Returned keys follow secondary-key and stable insertion order.
    ///
    /// # Parameters
    ///
    /// * `upper_bound` - Inclusive largest secondary key to detach.
    ///
    /// # Returns
    ///
    /// Primary keys for all detached entries in ordered-index order.
    ///
    /// # Panics
    ///
    /// Panics when the map is poisoned, a key or order-key trait operation
    /// panics, or an ordered entry violates the primary-index invariants. A
    /// panic during one entry's cross-index update poisons the map; a panic
    /// between entries can leave an already detached, consistent prefix.
    pub fn unindex_through(&mut self, upper_bound: &O) -> Vec<K> {
        self.assert_healthy();
        let mut detached_keys = Vec::new();
        while self
            .first_order_key()
            .is_some_and(|order_key| order_key <= upper_bound)
        {
            let (key, _order_key) = self
                .unindex_first()
                .expect("bounded ordered prefix must have a first entry");
            detached_keys.push(key);
        }
        detached_keys
    }

    /// Allocates a stable sequence for one new ordered entry.
    ///
    /// # Returns
    ///
    /// A sequence not previously returned since construction or the last
    /// `clear`.
    ///
    /// # Panics
    ///
    /// Panics when the sequence counter can no longer advance.
    #[must_use]
    #[inline]
    fn allocate_sequence(&mut self) -> u64 {
        let sequence = self.next_sequence;
        self.next_sequence = sequence
            .checked_add(1)
            .expect("ordered index insertion sequences exhausted");
        sequence
    }
}

impl<K, O, V> Clone for OrderedIndexMap<K, O, V>
where
    K: Clone,
    O: Clone,
    V: Clone,
{
    /// Clones both consistent index views and their sequence state.
    ///
    /// # Panics
    ///
    /// Panics when the source map is poisoned or a stored key, order key, or
    /// value panics while being cloned.
    fn clone(&self) -> Self {
        self.assert_healthy();
        Self {
            entries: self.entries.clone(),
            ordered_keys: self.ordered_keys.clone(),
            next_sequence: self.next_sequence,
            poisoned: false,
        }
    }
}

impl<K, O, V> Default for OrderedIndexMap<K, O, V> {
    /// Creates an empty map.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
