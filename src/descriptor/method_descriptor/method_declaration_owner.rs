// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Declaration owner for reflected methods.

use crate::descriptor::ImplDefinitionDescriptor;
use crate::descriptor::TraitDefinitionDescriptor;

/// The declaration that owns a method descriptor.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// #[cfg(feature = "derive")]
/// fn main() -> Result<(), qubit_reflect::error::RegistryError> {
///     use std::any::TypeId;
///     use std::sync::LazyLock;
///     use qubit_reflect::descriptor::{MethodDeclarationOwner, TraitCompleteness, TraitDefinitionDescriptor, TraitId};
///     use qubit_reflect::expression::GenericDefinitionDescriptor;
///     struct Marker;
///     static GENERICS: LazyLock<GenericDefinitionDescriptor> =
///         LazyLock::new(|| GenericDefinitionDescriptor::new([], []));
///     static DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
///         TraitDefinitionDescriptor::new(
///             TraitId::Reflected(TypeId::of::<Marker>()), "Service", "example::Service", "Service",
///             TraitCompleteness::Complete, &GENERICS,
///         )
///     });
///     let owner = MethodDeclarationOwner::Trait(&DEFINITION);
///     assert!(matches!(owner, MethodDeclarationOwner::Trait(_)));
///     Ok(())
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
#[derive(Clone, Copy, Debug)]
pub enum MethodDeclarationOwner {
    /// A method declared by a trait definition.
    Trait(
        /// Trait declaration that owns the method.
        &'static TraitDefinitionDescriptor,
    ),
    /// A method explicitly declared by an impl definition.
    Impl(
        /// Impl declaration that owns the method.
        &'static ImplDefinitionDescriptor,
    ),
}
