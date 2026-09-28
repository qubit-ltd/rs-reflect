// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Complete identity of one concrete trait application.

use crate::descriptor::TraitId;
use crate::expression::GenericArgument;

/// The complete identity of one concrete trait application.
///
/// # Examples
///
/// ```
/// use std::any::TypeId;
/// use std::sync::LazyLock;
/// use qubit_reflect::descriptor::{TraitCompleteness, TraitDefinitionDescriptor, TraitDescriptor, TraitId};
/// use qubit_reflect::expression::GenericDefinitionDescriptor;
///
/// struct Marker;
/// static GENERICS: LazyLock<GenericDefinitionDescriptor> =
///     LazyLock::new(|| GenericDefinitionDescriptor::new([], []));
/// static DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
///     TraitDefinitionDescriptor::new(
///         TraitId::Reflected(TypeId::of::<Marker>()), "Example", "example::Example", "Example",
///         TraitCompleteness::Complete, &GENERICS,
///     )
/// });
/// let applied = TraitDescriptor::builder(&DEFINITION).build().expect("valid application");
/// assert!(applied.trait_id().arguments().is_empty());
/// ```
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AppliedTraitId {
    /// Reflected marker or external declaration identity.
    pub(in crate::descriptor::trait_descriptor) definition: TraitId,
    /// Concrete generic arguments in declaration order.
    pub(in crate::descriptor::trait_descriptor) arguments: Box<[GenericArgument]>,
    /// Concrete associated-type equalities in declaration order.
    pub(in crate::descriptor::trait_descriptor) associated_type_arguments: Box<[GenericArgument]>,
}

impl AppliedTraitId {
    /// Returns the reflected marker or external definition identity.
    ///
    /// # Returns
    ///
    /// Returns the identity of the trait declaration.
    #[must_use]
    #[inline]
    pub const fn definition(&self) -> &TraitId {
        &self.definition
    }

    /// Returns concrete type and const arguments in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the ordered runtime generic arguments.
    #[must_use]
    #[inline]
    pub const fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Returns concrete associated-type equalities in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the associated-type equalities contributing to application
    /// identity.
    #[must_use]
    #[inline]
    pub const fn associated_type_arguments(&self) -> &[GenericArgument] {
        &self.associated_type_arguments
    }
}
