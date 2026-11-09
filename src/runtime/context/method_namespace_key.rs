// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::descriptor::trait_descriptor::TraitArgumentIdentity;
use crate::runtime::descriptor::trait_descriptor::TraitDefinitionId;

/// Identity scope used to distinguish methods from inherent and trait
/// namespaces.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum MethodNamespaceKey {
    /// Methods declared directly on the target type.
    Inherent,
    /// Methods associated with a typed trait definition and its arguments.
    TraitApplication {
        /// Identity of the linked trait declaration.
        definition: TraitDefinitionId,
        /// Generic arguments identifying this trait application.
        identity: &'static [TraitArgumentIdentity],
    },
    /// Methods associated with an unresolved source trait path.
    TraitPath {
        /// Source path used to display the trait namespace.
        path: &'static str,
        /// Generic arguments identifying this source application.
        identity: &'static [TraitArgumentIdentity],
        /// Distinguishes separately registered applications with the same
        /// spelling.
        application: Option<usize>,
    },
}
