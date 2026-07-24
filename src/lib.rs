// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! # qubit-collections
//!
//! Focused collection types with explicitly maintained secondary indexes.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod internal;
pub mod ordered_index_map;

pub use ordered_index_map::OrderedIndexMap;
