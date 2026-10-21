// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

mod ordered_index_map_tests;

use qubit_collections::map::OrderedIndexMap;

/// Verifies that the map category reexports its core collection type.
#[test]
fn test_map_reexports_ordered_index_map() {
    let mut map = OrderedIndexMap::new();
    map.insert("key", 1, "value");

    assert_eq!(Some(&"value"), map.get("key"));
}
