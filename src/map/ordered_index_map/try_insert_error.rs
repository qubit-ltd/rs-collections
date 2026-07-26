// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Rejected input returned by non-replacing insertion.

use std::error::Error;
use std::fmt;

/// Owned input rejected because its primary key was already present.
///
/// # Type Parameters
///
/// * `K` - Primary key type.
/// * `O` - Ordered secondary key type.
/// * `V` - Stored value type.
#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use = "the error contains the input that was not inserted"]
pub struct TryInsertError<K, O, V> {
    /// Rejected primary key.
    key: K,
    /// Rejected ordered key.
    order: O,
    /// Rejected value.
    value: V,
}

impl<K, O, V> TryInsertError<K, O, V> {
    /// Creates an error from rejected insertion input.
    #[inline(always)]
    pub(super) const fn new(key: K, order: O, value: V) -> Self {
        Self { key, order, value }
    }

    /// Returns the rejected primary key.
    ///
    /// # Returns
    ///
    /// A shared reference to the key that was not inserted.
    #[must_use]
    #[inline(always)]
    pub const fn key(&self) -> &K {
        &self.key
    }

    /// Returns the rejected ordered key.
    ///
    /// # Returns
    ///
    /// A shared reference to the ordered key that was not inserted.
    #[must_use]
    #[inline(always)]
    pub const fn order(&self) -> &O {
        &self.order
    }

    /// Returns the rejected value.
    ///
    /// # Returns
    ///
    /// A shared reference to the value that was not inserted.
    #[must_use]
    #[inline(always)]
    pub const fn value(&self) -> &V {
        &self.value
    }

    /// Splits the error into the rejected insertion components.
    ///
    /// # Returns
    ///
    /// The rejected primary key, ordered key, and value.
    #[must_use = "the returned components are the complete rejected input"]
    #[inline]
    pub fn into_parts(self) -> (K, O, V) {
        (self.key, self.order, self.value)
    }
}

impl<K, O, V> fmt::Display for TryInsertError<K, O, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("primary key is already present")
    }
}

impl<K, O, V> Error for TryInsertError<K, O, V>
where
    K: fmt::Debug,
    O: fmt::Debug,
    V: fmt::Debug,
{
}
