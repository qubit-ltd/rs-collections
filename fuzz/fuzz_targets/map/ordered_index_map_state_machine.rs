// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Fuzzes bounded `OrderedIndexMap` state transitions against an independent
//! model.
//!
//! Every operation uses the public API. After each transition, primary lookup,
//! attachment counts, record states, ordered iteration, and range reads must
//! agree with the model.

#![no_main]

use std::collections::HashMap;
use std::ops::Bound;

use libfuzzer_sys::fuzz_target;
use qubit_collections::map::OrderedIndexMap;
use qubit_collections::map::ordered_index_map::IndexState;

/// Maximum bytes processed by one fuzz iteration.
const MAX_INPUT_SIZE: usize = 4 * 1024;
/// Number of bytes decoded for one state transition.
const OPERATION_WIDTH: usize = 5;

/// Independent record representation used by the fuzz oracle.
#[derive(Clone, Copy, Debug)]
struct ModelEntry {
    /// Retained order key.
    order: u8,
    /// Stored value.
    value: i16,
    /// Stable sequence of the latest attachment.
    sequence: Option<u64>,
}

/// Returns attached model records in ordered iteration order.
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

/// Returns whether one order key lies within `bounds`.
fn order_in_bounds(order: u8, bounds: &(Bound<u8>, Bound<u8>)) -> bool {
    let starts_after_lower = match bounds.0 {
        Bound::Included(lower) => order >= lower,
        Bound::Excluded(lower) => order > lower,
        Bound::Unbounded => true,
    };
    let ends_before_upper = match bounds.1 {
        Bound::Included(upper) => order <= upper,
        Bound::Excluded(upper) => order < upper,
        Bound::Unbounded => true,
    };
    starts_after_lower && ends_before_upper
}

/// Builds a valid order range from the bounded fuzz input.
fn order_bounds(selector: u8, lower: u8, upper: u8) -> (Bound<u8>, Bound<u8>) {
    match selector % 9 {
        0 => (Bound::Unbounded, Bound::Unbounded),
        1 => (Bound::Unbounded, Bound::Included(upper)),
        2 => (Bound::Unbounded, Bound::Excluded(upper)),
        3 => (Bound::Included(lower), Bound::Unbounded),
        4 => (Bound::Excluded(lower), Bound::Unbounded),
        5 => (Bound::Included(lower), Bound::Included(upper)),
        6 => (Bound::Included(lower), Bound::Excluded(upper)),
        7 => (Bound::Excluded(lower), Bound::Included(upper)),
        _ if lower < upper => (Bound::Excluded(lower), Bound::Excluded(upper)),
        _ => (Bound::Included(lower), Bound::Included(upper)),
    }
}

/// Returns attached model records whose orders lie within `bounds`.
fn ordered_model_range(
    model: &HashMap<u8, ModelEntry>,
    bounds: &(Bound<u8>, Bound<u8>),
) -> Vec<(u8, u8, i16)> {
    ordered_model(model)
        .into_iter()
        .filter(|(_, order, _)| order_in_bounds(*order, bounds))
        .collect()
}

/// Verifies public map views against the independent model.
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
    assert_eq!(
        ordered_model(model),
        map.iter_ordered()
            .map(|entry| (*entry.key(), *entry.order(), *entry.value()))
            .collect::<Vec<_>>(),
    );
    for key in 0..16 {
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

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_SIZE {
        return;
    }

    let mut map = OrderedIndexMap::new();
    let mut model = HashMap::<u8, ModelEntry>::new();
    let mut sequence = 0_u64;

    for operation in data.chunks_exact(OPERATION_WIDTH) {
        let selector = operation[0] % 12;
        let key = operation[1] % 16;
        let order = operation[2] % 8;
        let other_order = operation[3] % 8;
        let value = i16::from_le_bytes([operation[3], operation[4]]);
        let lower = order.min(other_order);
        let upper = order.max(other_order);
        let bounds = order_bounds(operation[4], lower, upper);

        match selector {
            0 => {
                let previous = model.insert(
                    key,
                    ModelEntry {
                        order,
                        value,
                        sequence: Some(sequence),
                    },
                );
                sequence += 1;
                assert_eq!(
                    previous.map(|entry| {
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
                if let Some(entry) = model.get(&key) {
                    assert_eq!(
                        (key, order, value),
                        map.try_insert(key, order, value)
                            .expect_err("occupied key must be rejected")
                            .into_parts(),
                    );
                    assert_eq!(Some(entry.value), map.get(&key).copied());
                } else {
                    let _inserted = map
                        .try_insert(key, order, value)
                        .expect("vacant key must be inserted");
                    model.insert(
                        key,
                        ModelEntry {
                            order,
                            value,
                            sequence: Some(sequence),
                        },
                    );
                    sequence += 1;
                }
            }
            2 => {
                let expected = match model.get_mut(&key) {
                    Some(entry) if entry.sequence.is_some() => {
                        entry.sequence = None;
                        true
                    }
                    _ => false,
                };
                assert_eq!(expected, map.detach(&key).is_some());
            }
            3 => {
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
            4 => {
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
            5 => {
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
            6 => {
                let expected = ordered_model(&model).first().copied();
                if let Some((first_key, _, _)) = expected {
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
            7 => {
                if let Some(entry) = model.get_mut(&key) {
                    entry.value = value;
                }
                if let Some(stored) = map.get_mut(&key) {
                    *stored = value;
                }
            }
            8 => {
                assert_eq!(
                    ordered_model_range(&model, &bounds),
                    map.range(bounds.clone())
                        .map(|entry| (
                            *entry.key(),
                            *entry.order(),
                            *entry.value()
                        ))
                        .collect::<Vec<_>>(),
                );
            }
            9 => {
                let expected =
                    ordered_model_range(&model, &bounds).first().copied();
                let actual =
                    map.detach_range(bounds.clone()).next().map(|entry| {
                        (*entry.key(), *entry.order(), *entry.value())
                    });
                assert_eq!(expected, actual);
                if let Some((detached_key, _, _)) = expected {
                    model
                        .get_mut(&detached_key)
                        .expect("detached model record must exist")
                        .sequence = None;
                }
            }
            10 => {
                let expected =
                    ordered_model_range(&model, &bounds).last().copied();
                let actual =
                    map.extract_range(bounds).next_back().map(|entry| {
                        let (key, order, value, state) = entry.into_parts();
                        assert_eq!(IndexState::Attached, state);
                        (key, order, value)
                    });
                assert_eq!(expected, actual);
                if let Some((extracted_key, _, _)) = expected {
                    model.remove(&extracted_key);
                }
            }
            _ => {
                map.clear();
                model.clear();
                sequence = 0;
            }
        }
        assert_matches_model(&map, &model);
    }
});
