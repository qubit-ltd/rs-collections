// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Benchmarks primary-key and ordered-prefix operations.

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
fn benchmark_unindex_prefix(criterion: &mut Criterion) {
    let mut group =
        criterion.benchmark_group("ordered_index_map/unindex_prefix");
    for entry_count in ENTRY_COUNTS {
        let upper_bound = entry_count / 2;
        group.throughput(Throughput::Elements((upper_bound + 1) as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(entry_count),
            &entry_count,
            |bencher, &entry_count| {
                bencher.iter_batched(
                    || populated_map(entry_count),
                    |mut map| {
                        let _keys = black_box(
                            map.unindex_through(black_box(&upper_bound)),
                        );
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    benchmark_insertion,
    benchmark_primary_lookup,
    benchmark_unindex_prefix,
);
criterion_main!(benches);
