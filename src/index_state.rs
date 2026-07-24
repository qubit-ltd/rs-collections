// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Attachment state of a primary record's ordered index.

/// Describes whether a primary record participates in ordered operations.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[must_use = "the state determines whether ordered operations can observe a record"]
pub enum IndexState {
    /// The record participates in the ordered secondary index.
    Attached,
    /// The record remains in the primary map but not in the ordered index.
    Detached,
}
