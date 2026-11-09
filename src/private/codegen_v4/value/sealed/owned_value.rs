// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::Mode;
use crate::runtime::descriptor::Reflect;
use crate::runtime::value as runtime_value;

/// Marks a type that can be erased into an owned dynamic value.
///
/// # Type Parameters
///
/// * `M` — value mode that selects the runtime owner representation.
pub trait OwnedValue<M: Mode>: Reflect + Sized + 'static + runtime_value::RuntimeValueFor<M::Runtime> {
    // empty
}

impl<T: Reflect + Sized + 'static, M: Mode> OwnedValue<M> for T where T: runtime_value::RuntimeValueFor<M::Runtime> {}
