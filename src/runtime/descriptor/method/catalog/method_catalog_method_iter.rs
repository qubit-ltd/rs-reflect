// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Iterates over effective methods in either borrowed or shared storage.
enum MethodCatalogMethodIter<'a> {
    /// Iterates static method registrations.
    Borrowed(
        /// Source-ordered static methods for the target type.
        std::slice::Iter<'a, EffectiveMethod>,
    ),
    /// Iterates context-bound method registrations.
    Shared(
        /// Methods resolved for the current reflection context.
        std::slice::Iter<'a, ContextEffectiveMethod>,
    ),
}

impl<'a> Iterator for MethodCatalogMethodIter<'a> {
    /// Provides the item operation used to resolve or dispatch reflected
    /// methods.
    type Item = MethodCatalogMethod<'a>;

    /// Advances the catalog iterator and wraps the next effective method.
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Borrowed(methods) => methods.next().map(MethodCatalogMethod::Static),
            Self::Shared(methods) => methods.next().map(MethodCatalogMethod::Context),
        }
    }
}
