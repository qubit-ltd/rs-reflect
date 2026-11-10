// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Storage strategy for methods in a catalog tied to a reflection context.
#[derive(Clone)]
#[allow(dead_code)] // Retained for crate-level tests that construct static catalogs directly.
enum MethodCatalogMethods<'ctx> {
    /// Borrows static effective method metadata for the catalog lifetime.
    Borrowed(
        /// Methods shared by all contexts for a given target type.
        &'ctx [EffectiveMethod],
    ),
    /// Shares context-bound method metadata through reference counting.
    Shared(
        /// Methods resolved for the catalog's reflection context.
        std::sync::Arc<[ContextEffectiveMethod]>,
    ),
}
