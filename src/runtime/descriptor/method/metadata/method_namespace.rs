// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Context-bound namespace of an effective method implementation.
#[derive(Clone, Copy)]
pub enum MethodNamespace<'ctx> {
    /// An inherent method provided by an implementation on the target type.
    Inherent {
        /// The inherent `impl` block that provides this method.
        implementation: crate::runtime::descriptor::impl_descriptor::ImplDescriptor<'ctx>,
    },
    /// A trait method provided by an implementation on the target type.
    Trait {
        /// The trait application that provides this method.
        trait_: crate::runtime::descriptor::trait_descriptor::TraitDescriptor<'ctx>,
        /// The `impl` block that provides this method.
        implementation: crate::runtime::descriptor::impl_descriptor::ImplDescriptor<'ctx>,
    },
}
