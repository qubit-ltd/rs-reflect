// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// One effective method viewed from either catalog storage strategy.
enum MethodCatalogMethod<'a> {
    /// References method metadata independent of a specific context.
    Static(
        /// Static registration used to build the effective descriptor.
        &'a EffectiveMethod,
    ),
    /// References method metadata resolved for a particular context.
    Context(
        /// Context-bound registration and its resolved specialization data.
        &'a ContextEffectiveMethod,
    ),
}

impl MethodCatalogMethod<'_> {
    /// Returns the static effective method record shared by the catalog view.
    fn effective(&self) -> &EffectiveMethod {
        match self {
            Self::Static(method) => method,
            Self::Context(method) => &method.effective,
        }
    }
}
