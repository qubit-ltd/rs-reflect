// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Borrowed views over named and positional input values.

use crate::runtime::value::DynamicOwned;
use crate::runtime::value::ValueMode;

/// A borrowed input entry whose variant records its binding shape.
///
/// # Type Parameters
///
/// * `'a` — lifetime of the borrowed name and value.
/// * `M` — dynamic value mode of the referenced value.
pub(crate) enum InputEntryRef<'a, M: ValueMode> {
    /// Named entry with its source name and owned value.
    Named {
        /// Source field name.
        name: &'a str,
        /// Value currently associated with that name.
        value: &'a DynamicOwned<M>,
    },
    /// Positional entry with its owned value.
    Positional(
        /// Value at its source-order position.
        &'a DynamicOwned<M>,
    ),
}
