// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Primary-map record for an ordered secondary index.

/// Value and ordering metadata retained by the primary map.
///
/// # Type Parameters
///
/// * `O` - Ordered secondary key type.
/// * `V` - Stored value type.
#[derive(Clone, Debug)]
pub(crate) struct OrderedIndexEntry<O, V> {
    /// Secondary key retained even while the entry is unindexed.
    pub(crate) order_key: O,
    /// Stable ordering sequence, or `None` while the entry is unindexed.
    pub(crate) sequence: Option<u64>,
    /// User value owned by the primary map.
    pub(crate) value: V,
}
