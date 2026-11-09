// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Compatibility conversion for runtime and generated owned values.

use crate::runtime::value::DynamicOwned;
use crate::runtime::value::Local;
use crate::runtime::value::ValueMode;

/// Converts supported owned value wrappers to the runtime representation.
///
/// # Type Parameters
///
/// * `M` — dynamic value mode used by the converted owner.
pub(crate) trait IntoRuntimeOwned<M: ValueMode> {
    /// Transfers the value into the runtime-owned wrapper for `M`.
    fn into_runtime_owned(self) -> DynamicOwned<M>;
}

impl<M: ValueMode> IntoRuntimeOwned<M> for DynamicOwned<M> {
    /// Returns an already converted runtime value unchanged.
    fn into_runtime_owned(self) -> DynamicOwned<M> {
        self
    }
}

impl IntoRuntimeOwned<Local> for crate::__private::codegen_v4::value::DynamicOwned<Local> {
    /// Converts a generated local owner without cloning the contained value.
    fn into_runtime_owned(self) -> DynamicOwned<Local> {
        self.into_runtime()
    }
}

impl IntoRuntimeOwned<crate::runtime::value::ThreadSafe>
    for crate::__private::codegen_v4::value::DynamicOwned<crate::__private::codegen_v4::value::ThreadSafe>
{
    /// Converts a generated thread-safe owner without cloning its value.
    fn into_runtime_owned(self) -> DynamicOwned<crate::runtime::value::ThreadSafe> {
        self.into_runtime()
    }
}
