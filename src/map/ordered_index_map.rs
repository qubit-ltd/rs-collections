// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! A primary-key map with an independently managed ordered secondary index.

mod detach_range;
mod detached_entry_mut;
mod entry_mut;
mod entry_ref;
mod extract_range;
mod index_state;
mod internal;
mod owned_entry;
mod try_insert_error;

use std::borrow::Borrow;
use std::collections::hash_map::RandomState;
use std::fmt;
use std::hash::BuildHasher;
use std::hash::Hash;
use std::ops::Bound;
use std::ops::RangeBounds;

pub use self::detach_range::DetachRange;
pub use self::detached_entry_mut::DetachedEntryMut;
pub use self::entry_mut::EntryMut;
pub use self::entry_ref::EntryRef;
pub use self::extract_range::ExtractRange;
pub use self::index_state::IndexState;
use self::internal::InternalState;
use self::internal::Record;
use self::internal::Sequence;
use self::internal::SlotId;
pub use self::owned_entry::OwnedEntry;
pub use self::try_insert_error::TryInsertError;

/// Expanded bounds over an order key and its stable sequence.
type SequenceBounds<O> = (Bound<(O, Sequence)>, Bound<(O, Sequence)>);

/// A primary-key map with an independently managed ordered secondary index.
///
/// The primary key is stored exactly once. The ordered index stores private
/// arena identifiers, so `K` does not need to implement [`Clone`]. Equal order
/// keys retain attachment order. A record may be detached from ordered
/// operations while remaining addressable through its primary key.
///
/// `insert` replaces an occupied primary key and returns the previous record.
/// Use [`Self::try_insert`] when replacement would indicate a caller error.
///
/// This type performs no internal locking. It is [`Send`] and [`Sync`] exactly
/// when its type parameters and hash builder are, which makes it suitable for
/// external synchronization with types such as [`std::sync::Mutex`] or
/// [`std::sync::RwLock`].
///
/// # Interior Mutability
///
/// Keys and orders must remain logically unchanged while stored, including
/// changes made through interior-mutability types. Mutating either component
/// without rebuilding its index is a logic error and may make lookups or
/// ordered operations return incorrect results.
///
/// # Panic and Unwind Safety
///
/// Cross-index mutations are guarded by a poison flag. If a user-provided
/// [`Hash`], [`Eq`], [`Ord`], [`Clone`], or destructor implementation panics
/// after mutation starts, all later operations panic instead of exposing
/// potentially inconsistent indexes. The poisoned map should be discarded.
/// Stable attachment sequences use `u64`; insertion, attachment, or attached
/// reordering panics if that sequence space is exhausted. [`Self::clear`]
/// resets the sequence allocator.
///
/// # Examples
///
/// ```
/// use qubit_collections::map::OrderedIndexMap;
///
/// let mut queue = OrderedIndexMap::new();
/// queue.try_insert("job", 10, "ready")?;
/// let rejected = queue
///     .try_insert("job", 20, "duplicate")
///     .expect_err("the primary key is already occupied");
/// assert_eq!(("job", 20, "duplicate"), rejected.into_parts());
/// assert_eq!(Some(&"ready"), queue.get("job"));
/// # Ok::<(), qubit_collections::map::ordered_index_map::TryInsertError<&str, i32, &str>>(())
/// ```
///
/// # Type Parameters
///
/// * `K` - Unique primary key type.
/// * `O` - Possibly non-unique ordered secondary key type.
/// * `V` - Stored value type.
/// * `S` - Hash builder used by the primary index.
#[must_use = "the map owns its records and ordered index"]
pub struct OrderedIndexMap<K, O, V, S = RandomState> {
    /// Owning arena and private indexes.
    state: InternalState<K, O, V>,
    /// Hash builder for primary keys.
    hash_builder: S,
    /// Whether a cross-index mutation unwound before restoring consistency.
    poisoned: bool,
}

impl<K, O, V> OrderedIndexMap<K, O, V, RandomState> {
    /// Creates an empty map.
    ///
    /// # Returns
    ///
    /// An empty map using a randomly seeded hash builder.
    #[must_use = "the new map must be retained to store records"]
    #[inline]
    pub fn new() -> Self {
        Self::with_hasher(RandomState::new())
    }

    /// Creates an empty map with primary capacity.
    ///
    /// # Parameters
    ///
    /// * `capacity` - Number of primary records to reserve.
    ///
    /// # Returns
    ///
    /// An empty map with space for at least `capacity` records.
    ///
    /// # Panics
    ///
    /// Panics if reserving `capacity` records overflows or allocation fails.
    #[must_use = "the new map must be retained to store records"]
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, RandomState::new())
    }
}

impl<K, O, V, S> OrderedIndexMap<K, O, V, S> {
    /// Creates an empty map with a caller-provided hash builder.
    ///
    /// # Parameters
    ///
    /// * `hash_builder` - Builder used to hash primary keys.
    ///
    /// # Returns
    ///
    /// An empty map using `hash_builder`.
    #[must_use = "the new map must be retained to store records"]
    #[inline]
    pub const fn with_hasher(hash_builder: S) -> Self {
        Self {
            state: InternalState::new(),
            hash_builder,
            poisoned: false,
        }
    }

    /// Creates an empty map with primary capacity and a hash builder.
    ///
    /// # Parameters
    ///
    /// * `capacity` - Number of primary records to reserve.
    /// * `hash_builder` - Builder used to hash primary keys.
    ///
    /// # Returns
    ///
    /// An empty map configured with both arguments.
    ///
    /// # Panics
    ///
    /// Panics if reserving `capacity` records overflows or allocation fails.
    #[must_use = "the new map must be retained to store records"]
    #[inline]
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            state: InternalState::with_capacity(capacity),
            hash_builder,
            poisoned: false,
        }
    }

    /// Returns the number of primary records in either attachment state.
    ///
    /// # Returns
    ///
    /// The total number of attached and detached records.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned.
    #[must_use]
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.assert_healthy();
        self.state.arena.len()
    }

    /// Reports whether the primary map contains no records.
    ///
    /// # Returns
    ///
    /// `true` when the map has neither attached nor detached records.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned.
    #[must_use]
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.assert_healthy();
        self.state.arena.len() == 0
    }

    /// Returns the number of records attached to the ordered index.
    ///
    /// # Returns
    ///
    /// The number of records visible through ordered operations.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned.
    #[must_use]
    #[inline(always)]
    pub fn attached_len(&self) -> usize {
        self.assert_healthy();
        self.state.ordered.len()
    }

    /// Reports whether the ordered index contains no records.
    ///
    /// # Returns
    ///
    /// `true` when no record is attached to the ordered index.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned.
    #[must_use]
    #[inline(always)]
    pub fn is_attached_empty(&self) -> bool {
        self.assert_healthy();
        self.state.ordered.is_empty()
    }

    /// Returns the number of records accepted without growing primary storage.
    ///
    /// # Returns
    ///
    /// The shared capacity supported by both primary storage structures.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned.
    #[must_use]
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.assert_healthy();
        self.state
            .arena
            .capacity()
            .min(self.state.primary.capacity())
    }

    /// Removes every record while retaining allocated primary capacity.
    ///
    /// # Panics
    ///
    /// Panics when the map is poisoned or a stored destructor panics.
    #[inline]
    pub fn clear(&mut self) {
        self.with_mutation(|state, _| {
            state.primary.clear();
            state.ordered.clear();
            state.arena.clear();
            state.next_sequence = 0;
        });
    }

    /// Returns views of every primary record in unspecified order.
    ///
    /// Detached records are included.
    ///
    /// # Returns
    ///
    /// An iterator over all attached and detached records.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = EntryRef<'_, K, O, V>> {
        self.assert_healthy();
        self.state.primary.iter().map(|id| self.entry_ref(*id))
    }

    /// Returns attached records in ascending secondary-key order.
    ///
    /// Equal secondary keys retain attachment order.
    ///
    /// # Returns
    ///
    /// A double-ended, exact-size iterator over attached records.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned.
    #[inline]
    pub fn iter_ordered(
        &self,
    ) -> impl DoubleEndedIterator<Item = EntryRef<'_, K, O, V>> + ExactSizeIterator
    {
        self.assert_healthy();
        self.state.ordered.values().map(|id| self.entry_ref(*id))
    }

    /// Returns attached values in ascending secondary-key order.
    ///
    /// Equal secondary keys retain attachment order.
    ///
    /// # Returns
    ///
    /// A double-ended, exact-size iterator over attached values.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned.
    #[inline]
    pub fn values_ordered(
        &self,
    ) -> impl DoubleEndedIterator<Item = &V> + ExactSizeIterator {
        self.iter_ordered().map(|entry| entry.value)
    }

    /// Returns the first attached record.
    ///
    /// # Returns
    ///
    /// The lowest ordered record, or `None` when no record is attached.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned.
    #[must_use]
    #[inline]
    pub fn first(&self) -> Option<EntryRef<'_, K, O, V>>
    where
        O: Ord,
    {
        self.assert_healthy();
        let id = *self.state.ordered.first_key_value()?.1;
        Some(self.entry_ref(id))
    }

    /// Returns attached records whose order keys fall within `range`.
    ///
    /// # Parameters
    ///
    /// * `range` - Inclusive or exclusive bounds over secondary keys.
    ///
    /// # Returns
    ///
    /// A double-ended iterator over attached records in stable ordered-index
    /// order.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned, the start bound is greater than the end
    /// bound, equal start and end bounds are both excluded, or cloning a range
    /// bound panics.
    #[inline]
    pub fn range<R>(
        &self,
        range: R,
    ) -> impl DoubleEndedIterator<Item = EntryRef<'_, K, O, V>>
    where
        O: Clone + Ord,
        R: RangeBounds<O>,
    {
        self.assert_healthy();
        self.state
            .ordered
            .range(order_range_bounds(&range))
            .map(|(_, id)| self.entry_ref(*id))
    }

    /// Requires that no earlier cross-index mutation unwound.
    #[inline(always)]
    fn assert_healthy(&self) {
        assert!(
            !self.poisoned,
            "OrderedIndexMap is poisoned after a prior mutation panic",
        );
    }

    /// Runs one cross-index update under the poison guard.
    #[inline]
    fn with_mutation<R>(
        &mut self,
        operation: impl FnOnce(&mut InternalState<K, O, V>, &S) -> R,
    ) -> R {
        self.assert_healthy();
        self.poisoned = true;
        let result = operation(&mut self.state, &self.hash_builder);
        self.poisoned = false;
        result
    }

    /// Converts one occupied slot into a shared public view.
    #[inline(always)]
    fn entry_ref(&self, id: SlotId) -> EntryRef<'_, K, O, V> {
        let record = self
            .state
            .arena
            .get(id)
            .expect("primary index must reference an occupied arena slot");
        EntryRef {
            key: &record.key,
            order: &record.order,
            value: &record.value,
            state: record.state(),
        }
    }

    /// Converts one occupied slot into an exclusive public value view.
    #[inline(always)]
    fn entry_mut(&mut self, id: SlotId) -> EntryMut<'_, K, O, V> {
        let record = self
            .state
            .arena
            .get_mut(id)
            .expect("primary index must reference an occupied arena slot");
        let state = record.state();
        EntryMut {
            key: &record.key,
            order: &record.order,
            value: &mut record.value,
            state,
        }
    }

    /// Converts one detached slot into its specialized value view.
    #[inline(always)]
    fn detached_entry_mut(
        &mut self,
        id: SlotId,
    ) -> DetachedEntryMut<'_, K, O, V> {
        let record = self
            .state
            .arena
            .get_mut(id)
            .expect("primary index must reference an occupied arena slot");
        assert!(
            record.sequence.is_none(),
            "detached entry view requires a detached record",
        );
        DetachedEntryMut {
            key: &record.key,
            order: &record.order,
            value: &mut record.value,
        }
    }
}

impl<K, O, V, S> OrderedIndexMap<K, O, V, S>
where
    S: BuildHasher,
{
    /// Reserves primary capacity for at least `additional` more records.
    ///
    /// # Parameters
    ///
    /// * `additional` - Additional primary records expected.
    ///
    /// # Panics
    ///
    /// Panics when capacity overflows, allocation fails, the map is poisoned,
    /// or hashing a stored key panics. A panic after reservation starts
    /// poisons the map.
    pub fn reserve(&mut self, additional: usize)
    where
        K: Hash,
    {
        self.with_mutation(|state, hash_builder| {
            state.arena.reserve(additional);
            let arena = &state.arena;
            state.primary.reserve(additional, |id| {
                hash_builder.hash_one(
                    &arena
                        .get(*id)
                        .expect("primary index must reference an occupied arena slot")
                        .key,
                )
            });
        });
    }

    /// Reports whether a borrowed primary key is present.
    ///
    /// # Parameters
    ///
    /// * `key` - Borrowed form of the primary key to locate.
    ///
    /// # Returns
    ///
    /// `true` when the primary index contains an equivalent key.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned or hashing or comparing `key` panics.
    #[must_use]
    #[inline]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.find_slot(key).is_some()
    }

    /// Returns a stored value by borrowed primary key.
    ///
    /// # Parameters
    ///
    /// * `key` - Borrowed form of the primary key to locate.
    ///
    /// # Returns
    ///
    /// The stored value, or `None` when `key` is absent.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned or hashing or comparing `key` panics.
    #[must_use]
    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let id = self.find_slot(key)?;
        Some(&self.state.arena.get(id)?.value)
    }

    /// Returns exclusive value access by borrowed primary key.
    ///
    /// # Parameters
    ///
    /// * `key` - Borrowed form of the primary key to locate.
    ///
    /// # Returns
    ///
    /// Exclusive access to the stored value, or `None` when `key` is absent.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned or hashing or comparing `key` panics.
    #[must_use]
    #[inline]
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let id = self.find_slot(key)?;
        Some(&mut self.state.arena.get_mut(id)?.value)
    }

    /// Returns a complete shared record view by borrowed primary key.
    ///
    /// # Parameters
    ///
    /// * `key` - Borrowed form of the primary key to locate.
    ///
    /// # Returns
    ///
    /// A shared record view, or `None` when `key` is absent.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned or hashing or comparing `key` panics.
    #[must_use]
    #[inline]
    pub fn get_entry<Q>(&self, key: &Q) -> Option<EntryRef<'_, K, O, V>>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.find_slot(key).map(|id| self.entry_ref(id))
    }

    /// Returns a complete record view with exclusive value access.
    ///
    /// # Parameters
    ///
    /// * `key` - Borrowed form of the primary key to locate.
    ///
    /// # Returns
    ///
    /// A record view with exclusive value access, or `None` when `key` is
    /// absent.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned or hashing or comparing `key` panics.
    #[must_use]
    #[inline]
    pub fn get_entry_mut<Q>(&mut self, key: &Q) -> Option<EntryMut<'_, K, O, V>>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let id = self.find_slot(key)?;
        Some(self.entry_mut(id))
    }

    /// Finds one arena slot through the primary hash index.
    #[inline]
    fn find_slot<Q>(&self, key: &Q) -> Option<SlotId>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.assert_healthy();
        let hash = self.hash_builder.hash_one(key);
        self.state
            .primary
            .find(hash, |id| {
                self.state
                    .arena
                    .get(*id)
                    .is_some_and(|record| record.key.borrow() == key)
            })
            .copied()
    }
}

impl<K, O, V, S> OrderedIndexMap<K, O, V, S>
where
    K: Eq + Hash,
    O: Ord + Clone,
    S: BuildHasher,
{
    /// Inserts an attached record or replaces the record with the same key.
    ///
    /// Replacement returns the complete old record and its prior attachment
    /// state. The new record receives a fresh stable attachment sequence.
    ///
    /// # Parameters
    ///
    /// * `key` - Unique primary key for the new record.
    /// * `order` - Ordered secondary key for the new record.
    /// * `value` - Value stored by the new record.
    ///
    /// # Returns
    ///
    /// The replaced record in its prior attachment state, or `None` when
    /// `key` was vacant.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned, stable sequence allocation is exhausted,
    /// or user-provided hashing, equality, ordering, cloning, or destruction
    /// code panics. A panic after mutation starts poisons the map.
    pub fn insert(
        &mut self,
        key: K,
        order: O,
        value: V,
    ) -> Option<OwnedEntry<K, O, V>> {
        self.assert_healthy();
        let hash = self.hash_builder.hash_one(&key);
        let previous_id = self.find_slot(&key);
        let previous_ordered_key = previous_id.and_then(|id| {
            let record =
                self.state.arena.get(id).expect(
                    "primary index must reference an occupied arena slot",
                );
            record
                .sequence
                .map(|sequence| (record.order.clone(), sequence))
        });
        let indexed_order = order.clone();

        self.with_mutation(move |state, hash_builder| {
            let previous = previous_id.map(|id| {
                remove_primary_id(state, hash, id);
                if let Some(ordered_key) = previous_ordered_key {
                    let removed = state.ordered.remove(&ordered_key);
                    assert_eq!(
                        removed,
                        Some(id),
                        "ordered index must reference replaced slot"
                    );
                }
                owned_from_record(
                    state
                        .arena
                        .remove(id)
                        .expect("replaced primary slot must be occupied"),
                )
            });

            let sequence = state.allocate_sequence();
            let id = state.arena.insert(Record {
                key,
                order,
                value,
                sequence: Some(sequence),
            });
            let arena = &state.arena;
            state.primary.insert_unique(hash, id, |candidate| {
                hash_builder.hash_one(
                    &arena
                        .get(*candidate)
                        .expect("primary index must reference an occupied arena slot")
                        .key,
                )
            });
            let old = state.ordered.insert((indexed_order, sequence), id);
            assert!(old.is_none(), "ordered index sequence must be unique");
            previous
        })
    }

    /// Inserts a record only when its primary key is vacant.
    ///
    /// Unlike [`Self::insert`], this method never replaces an existing record.
    /// A rejected key leaves every map record and both indexes unchanged.
    ///
    /// # Parameters
    ///
    /// * `key` - Unique primary key proposed for the record.
    /// * `order` - Ordered secondary key proposed for the record.
    /// * `value` - Value proposed for the record.
    ///
    /// # Returns
    ///
    /// An exclusive view of the inserted record, or a [`TryInsertError`]
    /// containing the rejected key, order, and value when `key` is occupied.
    ///
    /// # Errors
    ///
    /// Returns [`TryInsertError`] containing the untouched input when `key`
    /// already exists.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned, if stable sequence allocation is
    /// exhausted, or if user-provided hashing, equality, ordering, cloning, or
    /// destruction code panics. A panic after mutation starts poisons the map.
    pub fn try_insert(
        &mut self,
        key: K,
        order: O,
        value: V,
    ) -> Result<EntryMut<'_, K, O, V>, TryInsertError<K, O, V>> {
        self.assert_healthy();
        if self.find_slot(&key).is_some() {
            return Err(TryInsertError::new(key, order, value));
        }
        let hash = self.hash_builder.hash_one(&key);
        let indexed_order = order.clone();
        let id = self.with_mutation(move |state, hash_builder| {
            let sequence = state.allocate_sequence();
            let id = state.arena.insert(Record {
                key,
                order,
                value,
                sequence: Some(sequence),
            });
            let arena = &state.arena;
            state.primary.insert_unique(hash, id, |candidate| {
                hash_builder.hash_one(
                    &arena
                        .get(*candidate)
                        .expect("primary index must reference an occupied arena slot")
                        .key,
                )
            });
            let previous = state.ordered.insert((indexed_order, sequence), id);
            assert!(previous.is_none(), "ordered index sequence must be unique");
            id
        });
        Ok(self.entry_mut(id))
    }

    /// Removes and returns a record in either attachment state.
    ///
    /// # Parameters
    ///
    /// * `key` - Borrowed form of the primary key to remove.
    ///
    /// # Returns
    ///
    /// The removed record and its prior attachment state, or `None` when
    /// `key` is absent.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned or user-provided hashing, equality,
    /// ordering, cloning, or destruction code panics. A panic after mutation
    /// starts poisons the map.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<OwnedEntry<K, O, V>>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.assert_healthy();
        let hash = self.hash_builder.hash_one(key);
        let id = self.find_slot(key)?;
        let ordered_key = {
            let record =
                self.state.arena.get(id).expect(
                    "primary index must reference an occupied arena slot",
                );
            record
                .sequence
                .map(|sequence| (record.order.clone(), sequence))
        };
        Some(self.remove_slot(hash, id, ordered_key))
    }

    /// Detaches a record from ordered operations while retaining it by key.
    ///
    /// # Parameters
    ///
    /// * `key` - Borrowed form of the primary key to detach.
    ///
    /// # Returns
    ///
    /// Exclusive value access for the detached record, or `None` when `key` is
    /// absent or already detached.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned or user-provided hashing, equality,
    /// ordering, cloning, or destruction code panics. A panic after mutation
    /// starts poisons the map.
    pub fn detach<Q>(
        &mut self,
        key: &Q,
    ) -> Option<DetachedEntryMut<'_, K, O, V>>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.assert_healthy();
        let id = self.find_slot(key)?;
        let ordered_key = {
            let record =
                self.state.arena.get(id).expect(
                    "primary index must reference an occupied arena slot",
                );
            (record.order.clone(), record.sequence?)
        };
        self.detach_slot(id, ordered_key);
        Some(self.detached_entry_mut(id))
    }

    /// Attaches a detached record using its retained order.
    ///
    /// # Parameters
    ///
    /// * `key` - Borrowed form of the primary key to attach.
    ///
    /// # Returns
    ///
    /// Exclusive value access for the attached record, or `None` when `key` is
    /// absent or already attached.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned, stable sequence allocation is exhausted,
    /// or user-provided hashing, equality, ordering, cloning, or destruction
    /// code panics. A panic after mutation starts poisons the map.
    pub fn attach<Q>(&mut self, key: &Q) -> Option<EntryMut<'_, K, O, V>>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.assert_healthy();
        let id = self.find_slot(key)?;
        let indexed_order = {
            let record =
                self.state.arena.get(id).expect(
                    "primary index must reference an occupied arena slot",
                );
            if record.sequence.is_some() {
                return None;
            }
            record.order.clone()
        };
        self.with_mutation(|state, _| {
            let sequence = state.allocate_sequence();
            let old = state.ordered.insert((indexed_order, sequence), id);
            assert!(old.is_none(), "ordered index sequence must be unique");
            state
                .arena
                .get_mut(id)
                .expect("attached primary slot must be occupied")
                .sequence = Some(sequence);
        });
        Some(self.entry_mut(id))
    }

    /// Replaces a record's retained order while preserving attachment state.
    ///
    /// Attached records receive a fresh stable sequence.
    ///
    /// # Parameters
    ///
    /// * `key` - Borrowed form of the primary key to reorder.
    /// * `order` - New ordered secondary key retained by the record.
    ///
    /// # Returns
    ///
    /// The previous order, or `None` when `key` is absent.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned, stable sequence allocation is exhausted,
    /// or user-provided hashing, equality, ordering, cloning, or destruction
    /// code panics. A panic after mutation starts poisons the map.
    pub fn set_order<Q>(&mut self, key: &Q, order: O) -> Option<O>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.assert_healthy();
        let id = self.find_slot(key)?;
        let (old_ordered_key, indexed_order) = {
            let record =
                self.state.arena.get(id).expect(
                    "primary index must reference an occupied arena slot",
                );
            (
                record
                    .sequence
                    .map(|sequence| (record.order.clone(), sequence)),
                record.sequence.map(|_| order.clone()),
            )
        };
        Some(self.with_mutation(|state, _| {
            let new_sequence =
                indexed_order.as_ref().map(|_| state.allocate_sequence());
            if let Some(old_key) = old_ordered_key {
                let removed = state.ordered.remove(&old_key);
                assert_eq!(
                    removed,
                    Some(id),
                    "ordered index must reference reordered slot"
                );
            }
            if let (Some(indexed_order), Some(sequence)) =
                (indexed_order, new_sequence)
            {
                let old = state.ordered.insert((indexed_order, sequence), id);
                assert!(old.is_none(), "ordered index sequence must be unique");
            }
            let record = state
                .arena
                .get_mut(id)
                .expect("reordered primary slot must be occupied");
            record.sequence = new_sequence;
            std::mem::replace(&mut record.order, order)
        }))
    }

    /// Returns a lending cursor that detaches attached records in `range`.
    ///
    /// Each `next` or `next_back` call returns exclusive access to the value
    /// just detached. The cursor keeps the map exclusively borrowed.
    /// Dropping the cursor early leaves unvisited records attached.
    ///
    /// # Parameters
    ///
    /// * `range` - Inclusive or exclusive bounds over secondary keys.
    ///
    /// # Returns
    ///
    /// A lending, double-ended cursor over matching attached records.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned, the start bound is greater than the end
    /// bound, equal start and end bounds are both excluded, or cloning a range
    /// bound panics.
    pub fn detach_range<R>(&mut self, range: R) -> DetachRange<'_, K, O, V, S>
    where
        R: RangeBounds<O>,
    {
        self.assert_healthy();
        DetachRange {
            map: self,
            bounds: order_range_bounds(&range),
        }
    }

    /// Returns an iterator that removes attached records in `range`.
    ///
    /// Dropping the iterator early leaves records not yet yielded in the map.
    ///
    /// # Parameters
    ///
    /// * `range` - Inclusive or exclusive bounds over secondary keys.
    ///
    /// # Returns
    ///
    /// A double-ended iterator that owns each removed record.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned, the start bound is greater than the end
    /// bound, equal start and end bounds are both excluded, or cloning a range
    /// bound panics.
    pub fn extract_range<R>(&mut self, range: R) -> ExtractRange<'_, K, O, V, S>
    where
        R: RangeBounds<O>,
    {
        self.assert_healthy();
        ExtractRange {
            map: self,
            bounds: order_range_bounds(&range),
        }
    }
}

impl<K, O, V, S> OrderedIndexMap<K, O, V, S>
where
    K: Hash,
    O: Ord,
    S: BuildHasher,
{
    /// Removes and returns the first attached record.
    ///
    /// # Returns
    ///
    /// The lowest ordered record, or `None` when no record is attached.
    ///
    /// # Panics
    ///
    /// Panics if the map is poisoned or hashing the primary key or destroying
    /// the ordered-index key panics. A destructor panic after removal starts
    /// poisons the map.
    pub fn pop_first(&mut self) -> Option<OwnedEntry<K, O, V>> {
        self.assert_healthy();
        let id = *self.state.ordered.first_key_value().map(|(_, id)| id)?;
        let hash = {
            let record =
                self.state.arena.get(id).expect(
                    "ordered index must reference an occupied arena slot",
                );
            self.hash_builder.hash_one(&record.key)
        };
        Some(self.with_mutation(|state, _| {
            remove_primary_id(state, hash, id);
            let (ordered_key, removed_id) = state
                .ordered
                .pop_first()
                .expect("first ordered slot must remain present");
            assert_eq!(
                removed_id, id,
                "first ordered slot must match the preflight slot",
            );
            drop(ordered_key);
            owned_from_record(
                state
                    .arena
                    .remove(id)
                    .expect("removed primary slot must be occupied"),
            )
        }))
    }
}

impl<K, O, V, S> OrderedIndexMap<K, O, V, S>
where
    O: Ord,
{
    /// Detaches one slot with a prevalidated ordered-index key.
    fn detach_slot(&mut self, id: SlotId, ordered_key: (O, Sequence)) {
        self.with_mutation(|state, _| {
            let removed = state.ordered.remove(&ordered_key);
            assert_eq!(
                removed,
                Some(id),
                "ordered index must reference detached slot"
            );
            state
                .arena
                .get_mut(id)
                .expect("detached primary slot must be occupied")
                .sequence = None;
        });
    }

    /// Removes one slot and both of its index references.
    fn remove_slot(
        &mut self,
        hash: u64,
        id: SlotId,
        ordered_key: Option<(O, Sequence)>,
    ) -> OwnedEntry<K, O, V> {
        self.with_mutation(|state, _| {
            remove_primary_id(state, hash, id);
            if let Some(ordered_key) = ordered_key {
                let removed = state.ordered.remove(&ordered_key);
                assert_eq!(
                    removed,
                    Some(id),
                    "ordered index must reference removed slot"
                );
            }
            owned_from_record(
                state
                    .arena
                    .remove(id)
                    .expect("removed primary slot must be occupied"),
            )
        })
    }
}

impl<K, O, V, S> Default for OrderedIndexMap<K, O, V, S>
where
    S: Default,
{
    /// Creates an empty map with the hash builder's default value.
    #[inline]
    fn default() -> Self {
        Self::with_hasher(S::default())
    }
}

impl<K, O, V, S> Clone for OrderedIndexMap<K, O, V, S>
where
    K: Clone,
    O: Clone,
    V: Clone,
    S: Clone,
{
    fn clone(&self) -> Self {
        self.assert_healthy();
        Self {
            state: self.state.clone(),
            hash_builder: self.hash_builder.clone(),
            poisoned: false,
        }
    }
}

impl<K, O, V, S> fmt::Debug for OrderedIndexMap<K, O, V, S>
where
    K: fmt::Debug,
    O: fmt::Debug,
    V: fmt::Debug,
    S: fmt::Debug,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.assert_healthy();
        formatter
            .debug_struct("OrderedIndexMap")
            .field("state", &self.state)
            .field("hash_builder", &self.hash_builder)
            .finish()
    }
}

/// Removes one exact slot identifier from the primary hash index.
fn remove_primary_id<K, O, V>(
    state: &mut InternalState<K, O, V>,
    hash: u64,
    id: SlotId,
) {
    let entry = state
        .primary
        .find_entry(hash, |candidate| *candidate == id)
        .expect("primary index must reference removed slot");
    let (removed, _) = entry.remove();
    assert_eq!(removed, id, "primary index removed an unexpected slot");
}

/// Converts a private arena record into its public owned representation.
fn owned_from_record<K, O, V>(record: Record<K, O, V>) -> OwnedEntry<K, O, V> {
    let state = record.state();
    OwnedEntry {
        key: record.key,
        order: record.order,
        value: record.value,
        state,
    }
}

/// Expands order bounds to cover all stable sequences for matching orders.
fn order_range_bounds<O, R>(range: &R) -> SequenceBounds<O>
where
    O: Clone + Ord,
    R: RangeBounds<O>,
{
    assert_valid_order_range(range);
    (
        match range.start_bound() {
            Bound::Included(order) => {
                Bound::Included((order.clone(), Sequence(0)))
            }
            Bound::Excluded(order) => {
                Bound::Excluded((order.clone(), Sequence(u64::MAX)))
            }
            Bound::Unbounded => Bound::Unbounded,
        },
        match range.end_bound() {
            Bound::Included(order) => {
                Bound::Included((order.clone(), Sequence(u64::MAX)))
            }
            Bound::Excluded(order) => {
                Bound::Excluded((order.clone(), Sequence(0)))
            }
            Bound::Unbounded => Bound::Unbounded,
        },
    )
}

/// Requires an order range to satisfy the standard ordered-map contract.
///
/// # Panics
///
/// Panics if the start bound is greater than the end bound, or equal start and
/// end bounds are both excluded.
fn assert_valid_order_range<O, R>(range: &R)
where
    O: Ord,
    R: RangeBounds<O>,
{
    let (start, start_excluded) = match range.start_bound() {
        Bound::Included(order) => (Some(order), false),
        Bound::Excluded(order) => (Some(order), true),
        Bound::Unbounded => (None, false),
    };
    let (end, end_excluded) = match range.end_bound() {
        Bound::Included(order) => (Some(order), false),
        Bound::Excluded(order) => (Some(order), true),
        Bound::Unbounded => (None, false),
    };
    if let (Some(start), Some(end)) = (start, end) {
        assert!(start <= end, "range start is greater than range end",);
        assert!(
            start != end || !(start_excluded && end_excluded),
            "range start and end are equal and excluded",
        );
    }
}
