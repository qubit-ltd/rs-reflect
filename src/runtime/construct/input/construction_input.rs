// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Sealed marker trait for supported construction input shapes.

use super::sealed;
use crate::runtime::value::ValueMode;

/// A supported named, positional, or unit construction input shape.
///
/// # Type Parameters
///
/// * `M` — dynamic value mode of the input fields.
pub(crate) trait ConstructionInput<M: ValueMode>: sealed::Sealed<M> {
    // empty
}
