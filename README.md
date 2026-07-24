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
keys with stable insertion order, and entries that remain in the primary map
after leaving the ordered index.

The crate has no runtime dependencies and provides no internal synchronization.
Wrap a map in the synchronization primitive appropriate for its owner.

## Complexity

| Operation | Complexity |
|---|---|
| Lookup or value mutation by primary key | Average `O(1)` |
| Inspect the smallest indexed order key | `O(log n)` |
| Insert, replace, remove, or detach one indexed entry | `O(log n)` |
| Detach a bounded prefix containing `k` entries | `O(k log n)` |
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

assert_eq!(Some((&"earlier", &10, &"first")), deadlines.first());
assert_eq!(vec!["earlier"], deadlines.unindex_through(&10));

// Detaching affects only the ordered view.
assert_eq!(Some(&"first"), deadlines.get("earlier"));
assert_eq!(Some((&"later", &20, &"second")), deadlines.first());
```

## Index lifecycle

Every insertion starts indexed. `pop_first` and `remove` delete a complete
record, while `unindex`, `unindex_first`, and `unindex_through` remove only the
ordered-index entry. The primary record retains its original order key and
remains accessible through `get`, `get_mut`, `order_key`, and `remove`.

The ordered key is intentionally not mutable through `get_mut`; changing it
without rebuilding the secondary index would violate the map's invariants.

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
