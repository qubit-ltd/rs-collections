// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_collections::OrderedIndexMap;
use std::cmp::Ordering as CmpOrdering;
use std::collections::HashMap;
use std::hash::{
    Hash,
    Hasher,
};
use std::panic::{
    AssertUnwindSafe,
    catch_unwind,
};
use std::sync::Arc;
use std::sync::atomic::{
    AtomicUsize,
    Ordering,
};

/// Primary key that panics on one selected hash invocation.
#[derive(Clone, Debug)]
struct PanicOnNthHash {
    /// Logical key value.
    value: u64,
    /// Shared hash invocation count.
    hash_calls: Arc<AtomicUsize>,
    /// One-based hash invocation that must panic.
    panic_on_call: usize,
}

impl PartialEq for PanicOnNthHash {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for PanicOnNthHash {}

impl Hash for PanicOnNthHash {
    /// Hashes the logical key while panicking on the configured invocation.
    fn hash<H: Hasher>(&self, state: &mut H) {
        let call = self.hash_calls.fetch_add(1, Ordering::SeqCst) + 1;
        assert_ne!(
            self.panic_on_call, call,
            "intentional primary-key hash panic",
        );
        self.value.hash(state);
    }
}

/// Primary key whose clones retain equality while exposing their generation.
#[derive(Debug)]
struct CloneDistinctKey {
    /// Logical primary-key identity.
    identifier: u8,
    /// Number of clone operations from the originally inserted key.
    clone_generation: u8,
}

impl CloneDistinctKey {
    /// Creates an originally inserted primary key.
    fn new(identifier: u8) -> Self {
        Self {
            identifier,
            clone_generation: 0,
        }
    }
}

impl Clone for CloneDistinctKey {
    /// Produces an equal primary key with a distinct clone generation.
    fn clone(&self) -> Self {
        Self {
            identifier: self.identifier,
            clone_generation: self.clone_generation + 1,
        }
    }
}

impl PartialEq for CloneDistinctKey {
    /// Compares logical primary-key identities only.
    fn eq(&self, other: &Self) -> bool {
        self.identifier == other.identifier
    }
}

impl Eq for CloneDistinctKey {}

impl Hash for CloneDistinctKey {
    /// Hashes the logical primary-key identity only.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.identifier.hash(state);
    }
}

/// Secondary key whose clones retain ordering while exposing their generation.
#[derive(Debug)]
struct CloneDistinctOrderKey {
    /// Logical secondary-key priority.
    priority: u8,
    /// Number of clone operations from the originally inserted order key.
    clone_generation: u8,
}

impl CloneDistinctOrderKey {
    /// Creates an originally inserted secondary key.
    fn new(priority: u8) -> Self {
        Self {
            priority,
            clone_generation: 0,
        }
    }
}

impl Clone for CloneDistinctOrderKey {
    /// Produces an equal secondary key with a distinct clone generation.
    fn clone(&self) -> Self {
        Self {
            priority: self.priority,
            clone_generation: self.clone_generation + 1,
        }
    }
}

impl PartialEq for CloneDistinctOrderKey {
    /// Compares logical secondary-key priorities only.
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for CloneDistinctOrderKey {}

impl PartialOrd for CloneDistinctOrderKey {
    /// Compares logical secondary-key priorities only.
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

impl Ord for CloneDistinctOrderKey {
    /// Orders secondary keys by their logical priority only.
    fn cmp(&self, other: &Self) -> CmpOrdering {
        self.priority.cmp(&other.priority)
    }
}

/// Minimal reference representation for one primary record.
#[derive(Debug)]
struct ReferenceEntry {
    /// Secondary key retained by the primary record.
    order_key: u8,
    /// Stable sequence assigned when the record was last inserted.
    sequence: u64,
    /// Value stored for the primary record.
    value: i16,
    /// Whether the record currently participates in ordered operations.
    indexed: bool,
}

/// Advances the deterministic state used to generate mixed operations.
fn next_reference_state(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

/// Finds the current first indexed record in the reference model.
fn reference_first(
    model: &HashMap<u8, ReferenceEntry>,
) -> Option<(u8, u8, i16)> {
    model
        .iter()
        .filter(|(_key, entry)| entry.indexed)
        .min_by_key(|(key, entry)| (entry.order_key, entry.sequence, **key))
        .map(|(key, entry)| (*key, entry.order_key, entry.value))
}

/// Asserts that every observable collection view matches the reference model.
fn assert_matches_reference_model(
    map: &OrderedIndexMap<u8, u8, i16>,
    model: &HashMap<u8, ReferenceEntry>,
    key_space: u8,
) {
    assert_eq!(model.len(), map.len());
    assert_eq!(
        model.values().filter(|entry| entry.indexed).count(),
        map.indexed_len(),
    );
    assert_eq!(model.is_empty(), map.is_empty());
    assert_eq!(
        model.values().all(|entry| !entry.indexed),
        map.is_index_empty(),
    );

    for key in 0..key_space {
        assert_eq!(model.contains_key(&key), map.contains_key(&key));
        assert_eq!(
            model.get(&key).map(|entry| entry.value),
            map.get(&key).copied()
        );
        assert_eq!(
            model.get(&key).map(|entry| entry.order_key),
            map.order_key(&key).copied(),
        );
    }

    let expected_first = reference_first(model);
    let actual_first = map
        .first()
        .map(|(key, order_key, value)| (*key, *order_key, *value));
    assert_eq!(expected_first, actual_first);
    assert_eq!(
        expected_first.map(|(_key, order_key, _value)| order_key),
        map.first_order_key().copied(),
    );
}

#[test]
fn test_ordered_index_map_new_is_empty() {
    let map = OrderedIndexMap::<u64, u64, &'static str>::new();
    assert_eq!(0, map.len());
    assert_eq!(0, map.indexed_len());
    assert!(map.is_empty());
    assert!(map.is_index_empty());
    assert_eq!(None, map.first_order_key());
    assert_eq!(None, map.first());

    let default_map = OrderedIndexMap::<u64, u64, &'static str>::default();
    assert!(default_map.is_empty());
}

#[test]
fn test_ordered_index_map_insert_get_and_first() {
    let mut map = OrderedIndexMap::new();
    assert_eq!(None, map.insert(2_u64, 20_u64, "later"));
    assert_eq!(None, map.insert(1_u64, 10_u64, "earlier"));

    assert_eq!(2, map.len());
    assert_eq!(2, map.indexed_len());
    assert!(map.contains_key(&1));
    assert_eq!(Some(&"later"), map.get(&2));
    assert_eq!(Some(&10), map.order_key(&1));
    assert_eq!(Some(&10), map.first_order_key());
    assert_eq!(Some((&1, &10, &"earlier")), map.first());
}

#[test]
fn test_ordered_index_map_get_mut_updates_only_value() {
    let mut map = OrderedIndexMap::new();
    map.insert(1_u64, 10_u64, String::from("value"));

    map.get_mut(&1)
        .expect("inserted value should remain addressable")
        .push_str("-updated");

    assert_eq!(Some(&String::from("value-updated")), map.get(&1));
    assert_eq!(Some(&10), map.order_key(&1));
}

#[test]
fn test_ordered_index_map_replacement_returns_previous_entry() {
    let mut map = OrderedIndexMap::new();
    map.insert(7_u64, 20_u64, "old");

    assert_eq!(Some((20, "old")), map.insert(7, 5, "new"));
    assert_eq!(1, map.len());
    assert_eq!(1, map.indexed_len());
    assert_eq!(Some((&7, &5, &"new")), map.first());
}

#[test]
fn test_ordered_index_map_replacement_gets_new_stable_sequence() {
    let mut map = OrderedIndexMap::new();
    map.insert(1_u64, 10_u64, "first");
    map.insert(2_u64, 10_u64, "second");

    assert_eq!(Some((10, "first")), map.insert(1, 10, "replacement"));
    assert_eq!(Some((2, 10, "second")), map.pop_first());
    assert_eq!(Some((1, 10, "replacement")), map.pop_first());
}

#[test]
fn test_ordered_index_map_replacement_reindexes_detached_entry() {
    let mut map = OrderedIndexMap::new();
    map.insert(1_u64, 10_u64, "detached");
    assert!(map.unindex(&1));

    assert_eq!(Some((10, "detached")), map.insert(1, 20, "reindexed"));
    assert_eq!(1, map.len());
    assert_eq!(1, map.indexed_len());
    assert_eq!(Some((&1, &20, &"reindexed")), map.first());
}

#[test]
fn test_ordered_index_map_duplicate_order_keys_are_stable() {
    let mut map = OrderedIndexMap::new();
    map.insert(2_u64, 10_u64, "first");
    map.insert(1_u64, 10_u64, "second");
    map.insert(3_u64, 10_u64, "third");

    assert_eq!(Some((2, 10, "first")), map.pop_first());
    assert_eq!(Some((1, 10, "second")), map.pop_first());
    assert_eq!(Some((3, 10, "third")), map.pop_first());
    assert_eq!(None, map.pop_first());
}

#[test]
fn test_ordered_index_map_unindex_handles_absent_and_empty_bounds() {
    let mut map = OrderedIndexMap::<u64, u64, &'static str>::new();
    assert!(!map.unindex(&1));
    assert_eq!(None, map.unindex_first());
    assert!(map.unindex_through(&10).is_empty());

    map.insert(1, 20, "later");
    assert!(map.unindex_through(&10).is_empty());
    assert_eq!(1, map.len());
    assert_eq!(1, map.indexed_len());
}

#[test]
fn test_ordered_index_map_unindex_preserves_primary_entry() {
    let mut map = OrderedIndexMap::new();
    map.insert(1_u64, 10_u64, "detached");
    map.insert(2_u64, 20_u64, "indexed");

    assert!(map.unindex(&1));
    assert!(!map.unindex(&1));

    assert_eq!(2, map.len());
    assert_eq!(1, map.indexed_len());
    assert_eq!(Some(&"detached"), map.get(&1));
    assert_eq!(Some(&10), map.order_key(&1));
    assert_eq!(Some((&2, &20, &"indexed")), map.first());
}

#[test]
fn test_ordered_index_map_unindex_first_preserves_primary_entry() {
    let mut map = OrderedIndexMap::new();
    map.insert(2_u64, 20_u64, "later");
    map.insert(1_u64, 10_u64, "first");

    assert_eq!(Some((1, 10)), map.unindex_first());
    assert_eq!(Some(&"first"), map.get(&1));
    assert_eq!(Some(&10), map.order_key(&1));
    assert_eq!(Some((&2, &20, &"later")), map.first());
}

#[test]
fn test_ordered_index_map_unindex_through_removes_bounded_prefix() {
    let mut map = OrderedIndexMap::new();
    map.insert(1_u64, 10_u64, "ten");
    map.insert(2_u64, 20_u64, "twenty-first");
    map.insert(3_u64, 20_u64, "twenty-second");
    map.insert(4_u64, 30_u64, "thirty");

    assert_eq!(vec![1, 2, 3], map.unindex_through(&20));
    assert_eq!(4, map.len());
    assert_eq!(1, map.indexed_len());
    assert_eq!(Some(&"ten"), map.get(&1));
    assert_eq!(Some(&"twenty-first"), map.get(&2));
    assert_eq!(Some(&"twenty-second"), map.get(&3));
    assert_eq!(Some((&4, &30, &"thirty")), map.first());
}

#[test]
fn test_ordered_index_map_remove_handles_indexed_and_unindexed_entries() {
    let mut map = OrderedIndexMap::new();
    map.insert(1_u64, 10_u64, "indexed");
    map.insert(2_u64, 20_u64, "detached");
    assert!(map.unindex(&2));

    assert_eq!(Some((10, "indexed")), map.remove(&1));
    assert_eq!(Some((20, "detached")), map.remove(&2));
    assert_eq!(None, map.remove(&3));
    assert!(map.is_empty());
    assert!(map.is_index_empty());
}

#[test]
fn test_ordered_index_map_pop_first_removes_complete_entry() {
    let mut map = OrderedIndexMap::new();
    map.insert(1_u64, 20_u64, "later");
    map.insert(2_u64, 10_u64, "earlier");

    assert_eq!(Some((2, 10, "earlier")), map.pop_first());
    assert!(!map.contains_key(&2));
    assert_eq!(1, map.len());
    assert_eq!(1, map.indexed_len());
}

#[test]
fn test_ordered_index_map_clear_removes_both_views() {
    let mut map = OrderedIndexMap::new();
    map.insert(1_u64, 10_u64, "indexed");
    map.insert(2_u64, 20_u64, "detached");
    assert!(map.unindex(&2));

    map.clear();

    assert!(map.is_empty());
    assert!(map.is_index_empty());
    assert_eq!(None, map.get(&1));
    assert_eq!(None, map.get(&2));
}

#[test]
fn test_ordered_index_map_clone_preserves_independent_views() {
    let mut original = OrderedIndexMap::new();
    original.insert(1_u64, 10_u64, String::from("indexed"));
    original.insert(2_u64, 20_u64, String::from("detached"));
    assert!(original.unindex(&2));

    let mut cloned = original.clone();
    assert_eq!(2, cloned.len());
    assert_eq!(1, cloned.indexed_len());
    assert_eq!(Some((&1, &10, &String::from("indexed"))), cloned.first());
    assert_eq!(Some((10, String::from("indexed"))), cloned.remove(&1));

    assert!(cloned.contains_key(&2));
    assert!(original.contains_key(&1));
    assert_eq!(Some((&1, &10, &String::from("indexed"))), original.first());
}

#[test]
fn test_ordered_index_map_supports_borrowed_key_queries() {
    let mut map = OrderedIndexMap::new();
    map.insert(String::from("alpha"), 1_u64, 10_u64);

    assert!(map.contains_key("alpha"));
    assert_eq!(Some(&1), map.order_key("alpha"));
    assert_eq!(Some(&10), map.get("alpha"));
    *map.get_mut("alpha")
        .expect("borrowed key should provide mutable access") = 20;
    assert!(map.unindex("alpha"));
    assert_eq!(Some((1, 20)), map.remove("alpha"));
}

#[test]
fn test_ordered_index_map_poisoned_after_mutation_panic() {
    let hash_calls = Arc::new(AtomicUsize::new(0));
    let key = PanicOnNthHash {
        value: 1,
        hash_calls,
        panic_on_call: 2,
    };
    let mut map = OrderedIndexMap::new();
    map.insert(
        PanicOnNthHash {
            value: 0,
            hash_calls: Arc::new(AtomicUsize::new(0)),
            panic_on_call: usize::MAX,
        },
        0_u64,
        "sentinel",
    );

    let insertion = catch_unwind(AssertUnwindSafe(|| {
        map.insert(key, 10_u64, "value");
    }));
    assert!(insertion.is_err());

    let later_access = catch_unwind(AssertUnwindSafe(|| map.len()));
    assert!(
        later_access.is_err(),
        "a partially updated map must reject subsequent use",
    );
}

#[test]
fn test_ordered_index_map_first_and_pop_first_return_primary_owned_keys() {
    let mut map = OrderedIndexMap::new();
    map.insert(
        CloneDistinctKey::new(7),
        CloneDistinctOrderKey::new(3),
        "value",
    );

    let (first_key, first_order_key, first_value) = map
        .first()
        .expect("inserted entry should be the first indexed entry");
    assert_eq!(0, first_key.clone_generation);
    assert_eq!(0, first_order_key.clone_generation);
    assert_eq!(&"value", first_value);

    let (popped_key, popped_order_key, popped_value) = map.pop_first().expect(
        "inserted entry should be removable as the first indexed entry",
    );
    assert_eq!(0, popped_key.clone_generation);
    assert_eq!(0, popped_order_key.clone_generation);
    assert_eq!("value", popped_value);
}

#[test]
fn test_ordered_index_map_matches_reference_model_for_mixed_operations() {
    const KEY_SPACE: u8 = 8;
    const OPERATION_COUNT: usize = 512;

    let mut map = OrderedIndexMap::new();
    let mut model: HashMap<u8, ReferenceEntry> = HashMap::new();
    let mut next_sequence = 0_u64;
    let mut state = 0x6A09_E667_F3BC_C909_u64;

    for _ in 0..OPERATION_COUNT {
        let generated = next_reference_state(&mut state);
        let operation = generated % 8;
        let key = ((generated >> 8) % u64::from(KEY_SPACE)) as u8;
        let order_key = ((generated >> 16) % 4) as u8;
        let value = (generated >> 24) as i16;

        match operation {
            0 => {
                let expected_previous = model
                    .remove(&key)
                    .map(|entry| (entry.order_key, entry.value));
                model.insert(
                    key,
                    ReferenceEntry {
                        order_key,
                        sequence: next_sequence,
                        value,
                        indexed: true,
                    },
                );
                next_sequence += 1;
                assert_eq!(
                    expected_previous,
                    map.insert(key, order_key, value)
                );
            }
            1 => {
                let expected = match model.get_mut(&key) {
                    Some(entry) if entry.indexed => {
                        entry.indexed = false;
                        true
                    }
                    _ => false,
                };
                assert_eq!(expected, map.unindex(&key));
            }
            2 => {
                let expected = model
                    .remove(&key)
                    .map(|entry| (entry.order_key, entry.value));
                assert_eq!(expected, map.remove(&key));
            }
            3 => {
                let expected = reference_first(&model).map(
                    |(first_key, first_order_key, _value)| {
                        model
                            .get_mut(&first_key)
                            .expect("reference first record must remain stored")
                            .indexed = false;
                        (first_key, first_order_key)
                    },
                );
                assert_eq!(expected, map.unindex_first());
            }
            4 => {
                let expected = reference_first(&model).map(
                    |(first_key, first_order_key, first_value)| {
                        model.remove(&first_key);
                        (first_key, first_order_key, first_value)
                    },
                );
                assert_eq!(expected, map.pop_first());
            }
            5 => {
                let mut detached_entries: Vec<(u8, u64, u8)> = model
                    .iter()
                    .filter(|(_key, entry)| {
                        entry.indexed && entry.order_key <= order_key
                    })
                    .map(|(indexed_key, entry)| {
                        (entry.order_key, entry.sequence, *indexed_key)
                    })
                    .collect();
                detached_entries.sort_unstable();
                let expected: Vec<u8> = detached_entries
                    .iter()
                    .map(|(_entry_order_key, _sequence, indexed_key)| {
                        *indexed_key
                    })
                    .collect();
                for (_entry_order_key, _sequence, indexed_key) in
                    detached_entries
                {
                    model
                        .get_mut(&indexed_key)
                        .expect("reference detached record must remain stored")
                        .indexed = false;
                }
                assert_eq!(expected, map.unindex_through(&order_key));
            }
            6 => {
                if let Some(actual_value) = map.get_mut(&key) {
                    *actual_value = value;
                    model
                        .get_mut(&key)
                        .expect(
                            "mutable map access requires a reference record",
                        )
                        .value = value;
                }
            }
            _operation => {
                map.clear();
                model.clear();
                next_sequence = 0;
            }
        }

        assert_matches_reference_model(&map, &model, KEY_SPACE);
    }
}

#[test]
fn test_ordered_index_map_iter_ordered_skips_unindexed_entries() {
    let mut map = OrderedIndexMap::new();
    map.insert("late", 30, "late");
    map.insert("first", 10, "first");
    map.insert("middle", 20, "middle");
    assert!(map.unindex(&"middle"));

    let entries = map
        .iter_ordered()
        .map(|(key, order_key, value)| (*key, *order_key, *value))
        .collect::<Vec<_>>();

    assert_eq!(vec![("first", 10, "first"), ("late", 30, "late")], entries);
}
