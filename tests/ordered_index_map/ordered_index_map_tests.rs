// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_collections::OrderedIndexMap;
use std::hash::{Hash, Hasher};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

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
    fn hash<H: Hasher>(&self, state: &mut H) {
        let call = self.hash_calls.fetch_add(1, Ordering::SeqCst) + 1;
        assert_ne!(
            self.panic_on_call, call,
            "intentional primary-key hash panic",
        );
        self.value.hash(state);
    }
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
