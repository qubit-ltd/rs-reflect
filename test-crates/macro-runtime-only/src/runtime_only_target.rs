// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect_derive::Reflect;
use qubit_reflect_derive::ReflectImpl;

/// Target used to verify derives with a runtime-only dependency.
#[Reflect]
struct RuntimeOnlyTarget;

#[ReflectImpl]
impl RuntimeOnlyTarget {
    /// Disabled method used to verify cfg handling in derive expansion.
    #[cfg(any())]
    fn disabled(&self) -> MissingImplType {
        unreachable!()
    }
}
