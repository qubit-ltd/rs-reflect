// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Structured, context-bound qualified method name.
#[derive(Clone, Copy)]
pub enum MethodQualifiedName<'ctx> {
    /// An inherent method qualified by its target type.
    Inherent {
        /// The type that owns the method.
        target: TypeDescriptor<'ctx>,
        /// The method's source name.
        name: &'static str,
    },
    /// A trait method qualified by its target type and trait application.
    Trait {
        /// The type that owns the method.
        target: TypeDescriptor<'ctx>,
        /// The trait that contributes the method.
        trait_: crate::runtime::descriptor::trait_descriptor::TraitDescriptor<'ctx>,
        /// The method's source name.
        name: &'static str,
    },
}

impl std::fmt::Display for MethodQualifiedName<'_> {
    /// Formats the method metadata using its structured Debug representation.
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the human-readable representation.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inherent { target, name } => {
                write!(formatter, "{}::{name}", target.rust_type_name())
            }
            Self::Trait { target, trait_, name } => {
                write!(formatter, "<{} as {}>::{name}", target.rust_type_name(), trait_.path())
            }
        }
    }
}
