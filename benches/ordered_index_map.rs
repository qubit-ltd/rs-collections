// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Benchmarks primary-key, ordered iteration, and range mutations.

use criterion::{
    BatchSize,
    BenchmarkId,
    Criterion,
    Throughput,
    criterion_group,
    criterion_main,
};
use qubit_collections::OrderedIndexMap;
use std::hint::black_box;

/// Representative populations from small registries to larger schedulers.
const ENTRY_COUNTS: [usize; 3] = [64, 1_024, 16_384];

/// Creates an indexed map whose key and order key both increase from zero.
///
/// # Parameters
///
/// * `entry_count` - Number of entries to insert.
///
/// # Returns
///
/// A populated map with `entry_count` indexed values.
fn populated_map(entry_count: usize) -> OrderedIndexMap<usize, usize, usize> {
    let mut map = OrderedIndexMap::new();
    for key in 0..entry_count {
        map.insert(key, key, key);
    }
    map
}

/// Creates an indexed map whose records all share one order key.
fn equal_order_map(entry_count: usize) -> OrderedIndexMap<usize, usize, usize> {
    let mut map = OrderedIndexMap::new();
    for key in 0..entry_count {
        map.insert(key, 0, key);
    }
    map
}

/// Returns a deterministic permutation for primary-key removal.
fn removal_keys(entry_count: usize) -> Vec<usize> {
    const ODD_MULTIPLIER: usize = 0x9E37_79B1;

    (0..entry_count)
        .map(|index| index.wrapping_mul(ODD_MULTIPLIER) % entry_count)
        .collect()
}

/// Benchmarks insertion into an initially empty map.
///
/// # Parameters
///
/// * `criterion` - Criterion benchmark registry.
fn benchmark_insertion(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("ordered_index_map/insertion");
    for entry_count in ENTRY_COUNTS {
        group.throughput(Throughput::Elements(entry_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entry_count),
            &entry_count,
            |bencher, &entry_count| {
                bencher.iter_batched(
                    OrderedIndexMap::new,
                    |mut map| {
                        for key in 0..entry_count {
                            map.insert(
                                black_box(key),
                                black_box(key),
                                black_box(key),
                            );
                        }
                        black_box(map)
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

/// Benchmarks repeated primary-key lookup in a populated map.
///
/// # Parameters
///
/// * `criterion` - Criterion benchmark registry.
fn benchmark_primary_lookup(criterion: &mut Criterion) {
    let mut group =
        criterion.benchmark_group("ordered_index_map/primary_lookup");
    for entry_count in ENTRY_COUNTS {
        let map = populated_map(entry_count);
        group.throughput(Throughput::Elements(entry_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entry_count),
            &entry_count,
            |bencher, &entry_count| {
                bencher.iter(|| {
                    for key in 0..entry_count {
                        let _value = black_box(map.get(black_box(&key)));
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmarks detaching the inclusive lower half of an ordered index.
///
/// # Parameters
///
/// * `criterion` - Criterion benchmark registry.
fn benchmark_detach_range(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("ordered_index_map/detach_range");
    for entry_count in ENTRY_COUNTS {
        let upper_bound = entry_count / 2;
        group.throughput(Throughput::Elements((upper_bound + 1) as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entry_count),
            &entry_count,
            |bencher, &entry_count| {
                bencher.iter_batched_ref(
                    || populated_map(entry_count),
                    |map| {
                        let mut cursor =
                            map.detach_range(..=black_box(upper_bound));
                        while let Some(entry) = cursor.next() {
                            let _entry = black_box(entry);
                        }
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

/// Benchmarks consuming the inclusive lower half of an ordered index.
///
/// # Parameters
///
/// * `criterion` - Criterion benchmark registry.
fn benchmark_extract_range(criterion: &mut Criterion) {
    let mut group =
        criterion.benchmark_group("ordered_index_map/extract_range");
    for entry_count in ENTRY_COUNTS {
        let upper_bound = entry_count / 2;
        group.throughput(Throughput::Elements((upper_bound + 1) as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entry_count),
            &entry_count,
            |bencher, &entry_count| {
                bencher.iter_batched_ref(
                    || populated_map(entry_count),
                    |map| {
                        for entry in
                            map.extract_range(..=black_box(upper_bound))
                        {
                            let _entry = black_box(entry);
                        }
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

/// Benchmarks primary removal in a deterministic non-sequential order.
fn benchmark_primary_removal(criterion: &mut Criterion) {
    let mut group =
        criterion.benchmark_group("ordered_index_map/primary_removal");
    for entry_count in ENTRY_COUNTS {
        group.throughput(Throughput::Elements(entry_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entry_count),
            &entry_count,
            |bencher, &entry_count| {
                bencher.iter_batched_ref(
                    || (populated_map(entry_count), removal_keys(entry_count)),
                    |(map, keys)| {
                        for key in keys {
                            let removed = map.remove(black_box(key));
                            black_box(removed);
                        }
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

/// Benchmarks ordered traversal when every record has equal priority.
fn benchmark_equal_order_iteration(criterion: &mut Criterion) {
    let mut group =
        criterion.benchmark_group("ordered_index_map/equal_order_iteration");
    for entry_count in ENTRY_COUNTS {
        let map = equal_order_map(entry_count);
        group.throughput(Throughput::Elements(entry_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entry_count),
            &entry_count,
            |bencher, &_entry_count| {
                bencher.iter(|| {
                    for entry in map.iter_ordered() {
                        let _entry = black_box(entry);
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmarks traversal of values through the ordered index.
///
/// # Parameters
///
/// * `criterion` - Criterion benchmark registry.
fn benchmark_values_ordered(criterion: &mut Criterion) {
    let mut group =
        criterion.benchmark_group("ordered_index_map/values_ordered");
    for entry_count in ENTRY_COUNTS {
        let map = populated_map(entry_count);
        group.throughput(Throughput::Elements(entry_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entry_count),
            &entry_count,
            |bencher, &_entry_count| {
                bencher.iter(|| {
                    for value in map.values_ordered() {
                        black_box(value);
                    }
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    benchmark_insertion,
    benchmark_primary_lookup,
    benchmark_detach_range,
    benchmark_extract_range,
    benchmark_primary_removal,
    benchmark_equal_order_iteration,
    benchmark_values_ordered,
);
criterion_main!(benches);
