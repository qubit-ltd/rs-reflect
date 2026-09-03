// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Explicit generic symbols used by structural expression generation.

use std::collections::HashSet;

use crate::ir::GenericKindIr;
use crate::ir::GenericsIr;

/// Generic symbols visible at one generated metadata site.
#[derive(Clone, Debug, Default)]
pub(crate) struct GenericEnvironment {
    type_parameters: HashSet<String>,
    const_parameters: HashSet<String>,
    lifetime_parameters: HashSet<String>,
}

impl GenericEnvironment {
    /// Creates an empty environment for a non-generic declaration.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Creates an environment from one declaration's generic parameters.
    pub(crate) fn from_generics(generics: &GenericsIr) -> Self {
        Self::new().with_generics(generics)
    }

    /// Extends this environment with a nested generic scope.
    #[must_use]
    pub(crate) fn with_generics(mut self, generics: &GenericsIr) -> Self {
        for parameter in &generics.params {
            match parameter.kind {
                GenericKindIr::Type => {
                    self.type_parameters.insert(parameter.name.clone());
                }
                GenericKindIr::Const => {
                    self.const_parameters.insert(parameter.name.clone());
                }
                GenericKindIr::Lifetime => {
                    self.lifetime_parameters.insert(parameter.name.clone());
                }
            }
        }
        self
    }

    /// Returns whether `name` is a visible type parameter.
    pub(crate) fn is_type_parameter(&self, name: &str) -> bool {
        self.type_parameters.contains(name)
    }

    /// Returns whether `name` is a visible const parameter.
    pub(crate) fn is_const_parameter(&self, name: &str) -> bool {
        self.const_parameters.contains(name)
    }

    /// Adds one type parameter for focused generator tests.
    #[cfg(test)]
    pub(crate) fn with_type_parameter(mut self, name: impl Into<String>) -> Self {
        self.type_parameters.insert(name.into());
        self
    }

    /// Adds one const parameter for focused generator tests.
    #[cfg(test)]
    pub(crate) fn with_const_parameter(mut self, name: impl Into<String>) -> Self {
        self.const_parameters.insert(name.into());
        self
    }
}
