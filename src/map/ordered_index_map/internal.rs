// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Private implementation types for [`super::OrderedIndexMap`].

mod entry_arena;
mod internal_state;
mod record;
mod sequence;
mod slot_id;

pub(crate) use entry_arena::EntryArena;
pub(crate) use internal_state::InternalState;
pub(crate) use record::Record;
pub(crate) use sequence::Sequence;
pub(crate) use slot_id::SlotId;
