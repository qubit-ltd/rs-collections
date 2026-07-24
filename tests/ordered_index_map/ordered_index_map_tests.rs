// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::{
    Hash,
    Hasher,
};
use std::ops::Bound;
use std::panic::{
    AssertUnwindSafe,
    catch_unwind,
};
use std::sync::Arc;
use std::sync::atomic::{
    AtomicBool,
    Ordering as AtomicOrdering,
};

use qubit_collections::{
    IndexState,
    OrderedIndexMap,
};

/// Primary key that intentionally does not implement [`Clone`].
#[derive(Debug, Eq, Hash, PartialEq)]
struct NonCloneKey(u64);

/// Order key that can panic while an ordered index is being changed.
#[derive(Clone, Debug)]
struct PanicOrder {
    /// Logical order value.
    value: u64,
    /// Whether comparisons must panic.
    panic: Arc<AtomicBool>,
}

impl PartialEq for PanicOrder {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for PanicOrder {}

impl PartialOrd for PanicOrder {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PanicOrder {
    fn cmp(&self, other: &Self) -> Ordering {
        assert!(
            !self.panic.load(AtomicOrdering::SeqCst),
            "intentional order comparison panic",
        );
        self.value.cmp(&other.value)
    }
}

/// Primary key that panics when hashing is enabled.
#[derive(Clone, Debug)]
struct PanicHash {
    /// Logical key value.
    value: u64,
    /// Whether hashing must panic.
    panic: Arc<AtomicBool>,
}

impl PartialEq for PanicHash {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for PanicHash {}

impl Hash for PanicHash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        assert!(
            !self.panic.load(AtomicOrdering::SeqCst),
            "intentional hash panic",
        );
        self.value.hash(state);
    }
}

/// Minimal model record used by the bounded mixed-operation test.
#[derive(Clone, Debug)]
struct ModelEntry {
    /// Retained secondary order.
    order: u8,
    /// Stored value.
    value: i16,
    /// Stable sequence of the latest attachment.
    sequence: Option<u64>,
}

/// Advances the deterministic mixed-operation generator.
fn next_state(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

/// Returns the model's attached records in ordered iteration order.
fn ordered_model(model: &HashMap<u8, ModelEntry>) -> Vec<(u8, u8, i16)> {
    let mut records = model
        .iter()
        .filter_map(|(key, entry)| {
            entry
                .sequence
                .map(|sequence| (entry.order, sequence, *key, entry.value))
        })
        .collect::<Vec<_>>();
    records.sort_unstable();
    records
        .into_iter()
        .map(|(order, _sequence, key, value)| (key, order, value))
        .collect()
}

/// Verifies every public read view against the model.
fn assert_matches_model(
    map: &OrderedIndexMap<u8, u8, i16>,
    model: &HashMap<u8, ModelEntry>,
) {
    assert_eq!(model.len(), map.len());
    assert_eq!(
        model
            .values()
            .filter(|entry| entry.sequence.is_some())
            .count(),
        map.attached_len(),
    );
    let actual = map
        .iter_ordered()
        .map(|entry| (*entry.key(), *entry.order(), *entry.value()))
        .collect::<Vec<_>>();
    assert_eq!(ordered_model(model), actual);
    for key in 0..8 {
        let expected = model.get(&key);
        assert_eq!(expected.is_some(), map.contains_key(&key));
        assert_eq!(expected.map(|entry| entry.value), map.get(&key).copied());
        assert_eq!(
            expected.map(|entry| entry.order),
            map.get_entry(&key).map(|entry| *entry.order()),
        );
        assert_eq!(
            expected.map(|entry| {
                if entry.sequence.is_some() {
                    IndexState::Attached
                } else {
                    IndexState::Detached
                }
            }),
            map.get_entry(&key).map(|entry| entry.state()),
        );
    }
}

#[test]
fn test_ordered_index_map_construction_and_capacity() {
    let mut map = OrderedIndexMap::<u64, u64, &'static str>::with_capacity(16);
    assert!(map.is_empty());
    assert!(map.is_attached_empty());
    assert_eq!(0, map.len());
    assert_eq!(0, map.attached_len());
    assert!(map.capacity() >= 16);

    map.reserve(32);
    assert!(map.capacity() >= 32);
    assert!(OrderedIndexMap::<u64, u64, u64>::default().is_empty());
}

#[test]
fn test_ordered_index_map_supports_non_clone_primary_keys() {
    let mut map = OrderedIndexMap::new();
    assert!(map.insert(NonCloneKey(1), 10_u64, "value").is_none());

    let entry = map
        .get_entry(&NonCloneKey(1))
        .expect("inserted non-clone key should remain addressable");
    assert_eq!(&NonCloneKey(1), entry.key());
    assert_eq!(&10, entry.order());
    assert_eq!(&"value", entry.value());
    assert_eq!(IndexState::Attached, entry.state());
}

#[test]
fn test_ordered_index_map_insert_lookup_and_replace() {
    let mut map = OrderedIndexMap::new();
    map.insert(String::from("later"), 20, String::from("old"));
    map.insert(String::from("first"), 10, String::from("first"));

    assert_eq!(Some(&String::from("old")), map.get("later"));
    map.get_mut("later")
        .expect("inserted value should exist")
        .push_str("-updated");
    let previous = map
        .insert(String::from("later"), 5, String::from("replacement"))
        .expect("replacement should return the old record");
    assert_eq!(
        (
            String::from("later"),
            20,
            String::from("old-updated"),
            IndexState::Attached,
        ),
        previous.into_parts(),
    );
    let first = map.first().expect("an attached record should exist");
    assert_eq!("later", first.key());
    assert_eq!(&5, first.order());
    assert_eq!("replacement", first.value());
}

#[test]
fn test_ordered_index_map_entry_views_expose_every_component() {
    let mut map = OrderedIndexMap::new();
    map.insert(1, 10, String::from("value"));

    {
        let mut entry = map.get_entry_mut(&1).expect("mutable entry view");
        assert_eq!(&1, entry.key());
        assert_eq!(&10, entry.order());
        assert_eq!("value", entry.value());
        assert_eq!(IndexState::Attached, entry.state());
        assert!(entry.is_attached());
        assert!(!entry.is_detached());
        entry.value_mut().push_str("-mut");
    }
    map.get_entry_mut(&1)
        .expect("second mutable entry view")
        .into_value_mut()
        .push_str("-consumed");
    assert!(map.get_entry_mut(&2).is_none());

    {
        let entry = map.get_entry(&1).expect("shared attached entry view");
        assert!(entry.is_attached());
        assert!(!entry.is_detached());
    }
    {
        let mut entry = map.detach(&1).expect("first detached entry view");
        assert_eq!(&1, entry.key());
        assert_eq!(&10, entry.order());
        assert_eq!("value-mut-consumed", entry.value());
        entry.value_mut().push_str("-detached");
    }
    assert!(map.attach(&1).is_some());
    map.detach(&1)
        .expect("second detached entry view")
        .into_value_mut()
        .push_str("-consumed");
    let entry = map.get_entry(&1).expect("shared detached entry view");
    assert!(!entry.is_attached());
    assert!(entry.is_detached());
    assert_eq!("value-mut-consumed-detached-consumed", entry.value());
}

#[test]
fn test_ordered_index_map_owned_entry_accessors_and_consumers() {
    let mut map = OrderedIndexMap::new();
    map.insert(1, 10, String::from("one"));
    let mut entry = map.remove(&1).expect("owned entry");
    assert_eq!(&1, entry.key());
    assert_eq!(&10, entry.order());
    assert_eq!("one", entry.value());
    assert_eq!(IndexState::Attached, entry.state());
    entry.value_mut().push_str("-mut");
    assert_eq!("one-mut", entry.value());

    map.insert(2, 20, String::from("two"));
    assert_eq!(20, map.remove(&2).expect("owned order").into_order());
    map.insert(3, 30, String::from("three"));
    assert_eq!(
        String::from("three"),
        map.remove(&3).expect("owned value").into_value(),
    );
}

#[test]
fn test_ordered_index_map_equal_orders_are_stable() {
    let mut map = OrderedIndexMap::new();
    map.insert(2, 10, "first");
    map.insert(1, 10, "second");
    map.insert(3, 10, "third");

    let keys = map
        .iter_ordered()
        .map(|entry| *entry.key())
        .collect::<Vec<_>>();
    assert_eq!(vec![2, 1, 3], keys);
    assert_eq!(2, map.pop_first().expect("first record").into_key());
    assert_eq!(1, map.pop_first().expect("second record").into_key());
    assert_eq!(3, map.pop_first().expect("third record").into_key());
    assert!(map.pop_first().is_none());
}

#[test]
fn test_ordered_index_map_detach_attach_and_set_order() {
    let mut map = OrderedIndexMap::new();
    map.insert("later", 20, String::from("later"));
    map.insert("first", 10, String::from("first"));

    let mut detached = map.detach("first").expect("record should detach");
    detached.value_mut().push_str("-detached");
    drop(detached);
    assert!(map.detach("first").is_none());
    assert_eq!(1, map.attached_len());
    assert_eq!(
        vec!["later"],
        map.values_ordered().map(String::as_str).collect::<Vec<_>>(),
    );
    assert_eq!(Some(10), map.set_order("first", 5));

    let attached = map.attach("first").expect("record should attach");
    assert_eq!(IndexState::Attached, attached.state());
    assert_eq!(&5, attached.order());
    assert_eq!("first-detached", attached.value());
    assert!(map.attach("first").is_none());
    assert_eq!(
        vec!["first-detached", "later"],
        map.values_ordered().map(String::as_str).collect::<Vec<_>>(),
    );
}

#[test]
fn test_ordered_index_map_set_order_repositions_attached_record() {
    let mut map = OrderedIndexMap::new();
    map.insert("first", 10, 1);
    map.insert("second", 20, 2);

    assert_eq!(Some(20), map.set_order("second", 5));
    let entries = map
        .iter_ordered()
        .map(|entry| (*entry.key(), *entry.order()))
        .collect::<Vec<_>>();
    assert_eq!(vec![("second", 5), ("first", 10)], entries);
}

#[test]
fn test_ordered_index_map_range_and_values_ordered() {
    let mut map = OrderedIndexMap::new();
    map.insert(1, 10, "ten");
    map.insert(2, 20, "twenty-first");
    map.insert(3, 20, "twenty-second");
    map.insert(4, 30, "thirty");

    let values = map
        .range(15..=20)
        .map(|entry| *entry.value())
        .collect::<Vec<_>>();
    assert_eq!(vec!["twenty-first", "twenty-second"], values);
    assert_eq!(
        vec!["ten", "twenty-first", "twenty-second", "thirty"],
        map.values_ordered().copied().collect::<Vec<_>>(),
    );
}

#[test]
fn test_ordered_index_map_all_range_bound_forms() {
    let mut map = OrderedIndexMap::new();
    for key in 0..4 {
        map.insert(key, key, key);
    }

    assert_eq!(
        vec![1],
        map.range(1..2)
            .map(|entry| *entry.key())
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        vec![2, 3],
        map.range((Bound::Excluded(1), Bound::Unbounded,))
            .map(|entry| *entry.key())
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        vec![0, 1],
        map.range(..2).map(|entry| *entry.key()).collect::<Vec<_>>(),
    );
    assert_eq!(
        vec![2, 3],
        map.range(2..).map(|entry| *entry.key()).collect::<Vec<_>>(),
    );

    let mut detached = map.detach_range(..2);
    while detached.next().is_some() {}
    drop(detached);
    let extracted = map
        .extract_range((Bound::Excluded(2), Bound::Unbounded))
        .map(|entry| entry.into_key())
        .collect::<Vec<_>>();
    assert_eq!(vec![3], extracted);
}

#[test]
fn test_ordered_index_map_detach_range_is_lending_and_double_ended() {
    let mut map = OrderedIndexMap::new();
    for key in 0..5 {
        map.insert(key, key, key * 10);
    }

    let mut cursor = map.detach_range(1..=3);
    let mut first = cursor.next().expect("lower record should detach");
    assert_eq!(&1, first.key());
    *first.value_mut() += 1;
    drop(first);
    let last = cursor.next_back().expect("upper record should detach");
    assert_eq!(&3, last.key());
    drop(last);
    let middle = cursor.next().expect("middle record should detach");
    assert_eq!(&2, middle.key());
    drop(middle);
    assert!(cursor.is_empty());
    assert!(cursor.next().is_none());
    drop(cursor);

    assert_eq!(2, map.attached_len());
    assert_eq!(Some(&11), map.get(&1));
    assert_eq!(
        vec![0, 4],
        map.iter_ordered()
            .map(|entry| *entry.key())
            .collect::<Vec<_>>(),
    );
}

#[test]
fn test_ordered_index_map_extract_range_removes_only_yielded_records() {
    let mut map = OrderedIndexMap::new();
    for key in 0..5 {
        map.insert(key, key, key * 10);
    }

    let mut extracted = map.extract_range(1..=3);
    assert_eq!(1, extracted.next().expect("first extraction").into_key());
    assert_eq!(
        3,
        extracted.next_back().expect("last extraction").into_key()
    );
    drop(extracted);

    assert!(!map.contains_key(&1));
    assert!(map.contains_key(&2));
    assert!(!map.contains_key(&3));
    assert_eq!(3, map.len());
}

#[test]
fn test_ordered_index_map_remove_preserves_prior_state() {
    let mut map = OrderedIndexMap::new();
    map.insert(1, 10, "attached");
    map.insert(2, 20, "detached");
    assert!(map.detach(&2).is_some());

    assert_eq!(
        (1, 10, "attached", IndexState::Attached),
        map.remove(&1).expect("attached record").into_parts(),
    );
    assert_eq!(
        (2, 20, "detached", IndexState::Detached),
        map.remove(&2).expect("detached record").into_parts(),
    );
    assert!(map.remove(&3).is_none());
}

#[test]
fn test_ordered_index_map_iter_includes_detached_records() {
    let mut map = OrderedIndexMap::new();
    map.insert(1, 10, "attached");
    map.insert(2, 20, "detached");
    assert!(map.detach(&2).is_some());

    let mut states = map
        .iter()
        .map(|entry| (*entry.key(), entry.state()))
        .collect::<Vec<_>>();
    states.sort_unstable_by_key(|entry| entry.0);
    assert_eq!(
        vec![(1, IndexState::Attached), (2, IndexState::Detached),],
        states,
    );
}

#[test]
fn test_ordered_index_map_clear_retains_capacity() {
    let mut map = OrderedIndexMap::with_capacity(8);
    map.insert(1, 10, "attached");
    map.insert(2, 20, "detached");
    assert!(map.detach(&2).is_some());
    let capacity = map.capacity();

    map.clear();

    assert!(map.is_empty());
    assert!(map.is_attached_empty());
    assert_eq!(capacity, map.capacity());
}

#[test]
fn test_ordered_index_map_clone_is_independent() {
    let mut original = OrderedIndexMap::new();
    original.insert(1, 10, String::from("attached"));
    original.insert(2, 20, String::from("detached"));
    assert!(original.detach(&2).is_some());

    let mut cloned = original.clone();
    cloned.get_mut(&1).expect("cloned value").push_str("-clone");
    drop(cloned.attach(&2).expect("cloned detached record"));

    assert_eq!("attached", original.get(&1).expect("original value"));
    assert_eq!(1, original.attached_len());
    assert_eq!(2, cloned.attached_len());
    assert!(format!("{cloned:?}").contains("OrderedIndexMap"));
}

#[test]
fn test_ordered_index_map_is_send_and_sync_when_components_are() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<OrderedIndexMap<String, u64, Vec<u8>>>();
}

#[test]
fn test_ordered_index_map_preflight_hash_panic_does_not_poison() {
    let panic = Arc::new(AtomicBool::new(false));
    let mut map = OrderedIndexMap::new();
    map.insert(
        PanicHash {
            value: 1,
            panic: Arc::clone(&panic),
        },
        1,
        "value",
    );
    panic.store(true, AtomicOrdering::SeqCst);

    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _ = map.remove(&PanicHash {
                value: 1,
                panic: Arc::clone(&panic),
            });
        }))
        .is_err(),
    );
    panic.store(false, AtomicOrdering::SeqCst);
    assert_eq!(1, map.len());
    assert_eq!(Some(&"value"), map.get(&PanicHash { value: 1, panic }));
}

#[test]
fn test_ordered_index_map_poisoned_after_reserve_hash_panic() {
    let panic = Arc::new(AtomicBool::new(false));
    let mut map = OrderedIndexMap::new();
    map.insert(
        PanicHash {
            value: 1,
            panic: Arc::clone(&panic),
        },
        1,
        "value",
    );
    panic.store(true, AtomicOrdering::SeqCst);

    assert!(catch_unwind(AssertUnwindSafe(|| map.reserve(1_024))).is_err(),);
    assert!(catch_unwind(AssertUnwindSafe(|| map.len())).is_err());
}

#[test]
fn test_ordered_index_map_poisoned_after_order_mutation_panic() {
    let panic = Arc::new(AtomicBool::new(false));
    let mut map = OrderedIndexMap::new();
    map.insert(
        1,
        PanicOrder {
            value: 1,
            panic: Arc::clone(&panic),
        },
        "first",
    );
    panic.store(true, AtomicOrdering::SeqCst);

    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            map.insert(
                2,
                PanicOrder {
                    value: 2,
                    panic: Arc::clone(&panic),
                },
                "second",
            );
        }))
        .is_err(),
    );
    assert!(catch_unwind(AssertUnwindSafe(|| map.len())).is_err());
}

#[test]
fn test_ordered_index_map_matches_bounded_mixed_operation_model() {
    const OPERATION_COUNT: usize = 1_024;

    let mut map = OrderedIndexMap::new();
    let mut model = HashMap::<u8, ModelEntry>::new();
    let mut sequence = 0_u64;
    let mut state = 0x6A09_E667_F3BC_C909_u64;

    for _ in 0..OPERATION_COUNT {
        let generated = next_state(&mut state);
        let operation = generated % 7;
        let key = ((generated >> 8) % 8) as u8;
        let order = ((generated >> 16) % 4) as u8;
        let value = (generated >> 24) as i16;

        match operation {
            0 => {
                let expected = model.insert(
                    key,
                    ModelEntry {
                        order,
                        value,
                        sequence: Some(sequence),
                    },
                );
                sequence += 1;
                assert_eq!(
                    expected.map(|entry| {
                        (
                            key,
                            entry.order,
                            entry.value,
                            if entry.sequence.is_some() {
                                IndexState::Attached
                            } else {
                                IndexState::Detached
                            },
                        )
                    }),
                    map.insert(key, order, value)
                        .map(|entry| entry.into_parts()),
                );
            }
            1 => {
                let expected = match model.get_mut(&key) {
                    Some(entry) if entry.sequence.is_some() => {
                        entry.sequence = None;
                        true
                    }
                    _ => false,
                };
                assert_eq!(expected, map.detach(&key).is_some());
            }
            2 => {
                let expected = match model.get_mut(&key) {
                    Some(entry) if entry.sequence.is_none() => {
                        entry.sequence = Some(sequence);
                        sequence += 1;
                        true
                    }
                    _ => false,
                };
                assert_eq!(expected, map.attach(&key).is_some());
            }
            3 => {
                let expected = model.get_mut(&key).map(|entry| {
                    let previous = std::mem::replace(&mut entry.order, order);
                    if entry.sequence.is_some() {
                        entry.sequence = Some(sequence);
                        sequence += 1;
                    }
                    previous
                });
                assert_eq!(expected, map.set_order(&key, order));
            }
            4 => {
                let expected = model.remove(&key).map(|entry| {
                    (
                        key,
                        entry.order,
                        entry.value,
                        if entry.sequence.is_some() {
                            IndexState::Attached
                        } else {
                            IndexState::Detached
                        },
                    )
                });
                assert_eq!(
                    expected,
                    map.remove(&key).map(|entry| entry.into_parts()),
                );
            }
            5 => {
                let expected = ordered_model(&model).first().copied();
                if let Some((first_key, _order, _value)) = expected {
                    model.remove(&first_key);
                }
                assert_eq!(
                    expected,
                    map.pop_first().map(|entry| {
                        let (key, order, value, state) = entry.into_parts();
                        assert_eq!(IndexState::Attached, state);
                        (key, order, value)
                    }),
                );
            }
            _ => {
                if let Some(entry) = model.get_mut(&key) {
                    entry.value = value;
                }
                if let Some(stored) = map.get_mut(&key) {
                    *stored = value;
                }
            }
        }
        assert_matches_model(&map, &model);
    }
}
