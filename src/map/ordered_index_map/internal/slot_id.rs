// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Private stable index into the record arena.

/// Identifies an arena slot used by the map's internal indexes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct SlotId(
    /// Zero-based arena position; the slot may be reused after its value is
    /// removed.
    pub(crate) usize,
);
