# Qubit Collections

[![Rust CI](https://github.com/qubit-ltd/rs-collections/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-collections/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-collections/coverage-badge.json)](https://qubit-ltd.github.io/rs-collections/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-collections.svg?color=blue)](https://crates.io/crates/qubit-collections)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Focused Rust collection types for query patterns not directly represented by
the standard library.

## Overview

Qubit Collections starts with `OrderedIndexMap<K, O, V>`, a primary-key map
with an independently managed ordered secondary index. It supports fast lookup
by `K`, earliest-entry and bounded-prefix operations by `O`, duplicate order
keys with stable attachment order, explicit attached/detached states, bounded
range cursors, and entries that remain in the primary map after leaving the
ordered index. Primary keys are stored once and do not need to be cloneable.

The crate uses `hashbrown`'s raw hash-table abstraction for its private
primary-key index. It does not use `slotmap`: a compact private
`Vec<Option<_>>` arena is sufficient because slot identifiers never escape the
map. The crate provides no internal synchronization. A map is `Send` and
`Sync` when its components are; wrap shared mutable access in the
synchronization primitive appropriate for its owner.

## Complexity

| Operation | Complexity |
|---|---|
| Lookup or value mutation by primary key | Average `O(1)` |
| Inspect the smallest attached record | `O(log n)` |
| Insert, replace, remove, attach, detach, or reorder one record | `O(log n)` |
| Detach or extract a range containing `k` records | `O(k log n)` |
| Storage | `O(n)` |

## Installation

```toml
[dependencies]
qubit-collections = "0.1"
```

## Quick start

```rust
use qubit_collections::OrderedIndexMap;

let mut deadlines = OrderedIndexMap::new();
deadlines.insert("later", 20, "second");
deadlines.insert("earlier", 10, "first");

let first = deadlines.first().expect("an attached deadline exists");
assert_eq!(&"earlier", first.key());

// Detaching affects only the ordered view and gives direct value access.
let detached = deadlines.detach("earlier").expect("record is attached");
assert_eq!(&"first", detached.value());
drop(detached);
assert_eq!(Some(&"first"), deadlines.get("earlier"));
assert_eq!(
    vec![&"second"],
    deadlines.values_ordered().collect::<Vec<_>>(),
);
```

## Index lifecycle

Every insertion starts attached. `pop_first`, `remove`, and `extract_range`
delete complete records. `detach` and `detach_range` retain primary records and
return value access without another hash lookup; `attach` restores ordered
visibility. `set_order` changes the retained order while preserving the
attachment state.

`get_entry`, `iter`, `iter_ordered`, `range`, and `first` expose a record's key,
order, value, and `IndexState`. `values_ordered` is the concise path for
downstream code that only needs values. The primary key and order are not
directly mutable because changing either without rebuilding its index would
violate collection invariants.

If a user-defined key trait panics while a cross-index mutation is in progress,
the map becomes poisoned and rejects later operations. Discard that instance
instead of observing a partially updated index.

## Testing

```bash
# Run tests with the default feature set
cargo test

# Run tests with all declared features
cargo test --all-features

# Project CI checks
./ci-check.sh

# Check code coverage
./coverage.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the
full license text.

## Contributing

Contributions are welcome. Please follow the Rust API guidelines, keep public
API documentation and tests current, and run `./align-ci.sh` to format code and
`./ci-check.sh` to satisfy CI requirements before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-collections](https://github.com/qubit-ltd/rs-collections)
