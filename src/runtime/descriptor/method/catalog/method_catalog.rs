// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// An explicit set of methods used by context registration to build lookups.
#[derive(Clone)]
pub(crate) struct MethodCatalog<'ctx> {
    /// Concrete target type whose method metadata is being registered.
    target: TypeDescriptor<'ctx>,
    /// Returns the static method slice, resolving it through the provider when
    /// configured.
    methods: MethodCatalogMethods<'ctx>,
}
