// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect_derive::Reflect;

use super::runtime_only_impl::RuntimeOnlyImpl;

/// Trait used to verify direct derive macro expansion.
#[Reflect]
trait RuntimeOnlyTrait {
    /// Disabled method used to verify cfg handling in trait derives.
    #[cfg(any())]
    fn disabled(&self) -> MissingTraitType;
}

impl RuntimeOnlyTrait for RuntimeOnlyImpl {}

/// Exercises the directly derived trait in the compile fixture.
pub(super) fn assert_runtime_only_trait() {
    let _ = RuntimeOnlyImpl;
    let _: &dyn RuntimeOnlyTrait = &RuntimeOnlyImpl;
}
