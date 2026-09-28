// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shared declaration-level facts for a reflected trait.

use std::sync::OnceLock;

use crate::descriptor::AssociatedConstDescriptor;
use crate::descriptor::AssociatedTypeDescriptor;
use crate::descriptor::MethodDescriptor;
use crate::descriptor::TraitCompleteness;
use crate::descriptor::TraitId;
use crate::expression::GenericDefinitionDescriptor;
use crate::identity::Visibility;

/// Declaration-level facts shared by every concrete application of a trait.
///
/// This descriptor is created by generated metadata and shared by all
/// applications of the same trait declaration.
///
/// # Examples
///
/// ```
/// use std::any::TypeId;
/// use std::sync::LazyLock;
/// use qubit_reflect::descriptor::{TraitCompleteness, TraitDefinitionDescriptor, TraitId};
/// use qubit_reflect::expression::GenericDefinitionDescriptor;
///
/// struct Marker;
/// static GENERICS: LazyLock<GenericDefinitionDescriptor> =
///     LazyLock::new(|| GenericDefinitionDescriptor::new([], []));
/// static DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
///     TraitDefinitionDescriptor::new(
///         TraitId::Reflected(TypeId::of::<Marker>()), "ExampleService", "example::ExampleService",
///         "ExampleService", TraitCompleteness::Complete, &GENERICS,
///     )
/// });
/// assert_eq!(DEFINITION.query_name(), "ExampleService");
/// ```
#[derive(Debug)]
pub struct TraitDefinitionDescriptor {
    /// Reflected marker or external identity of the declaration.
    trait_id: TraitId,
    /// Source Rust identifier for the trait.
    rust_name: &'static str,
    /// Diagnostic fully qualified source path.
    rust_path: &'static str,
    /// Stable name used by reflection queries.
    query_name: &'static str,
    /// Extent of declaration facts known to reflection.
    completeness: TraitCompleteness,
    /// Generic parameters and predicates shared by all applications.
    generic_definition: &'static GenericDefinitionDescriptor,
    /// Normalized visibility of the source declaration.
    visibility: Visibility,
    /// Lazily initialized associated-item facts shared across applications.
    members: OnceLock<TraitDefinitionMembers>,
}

/// Associated-item facts retained before a concrete trait application exists.
#[derive(Debug)]
struct TraitDefinitionMembers {
    /// Declared methods in source order.
    methods: Box<[MethodDescriptor]>,
    /// Declared associated types in source order.
    associated_types: Box<[AssociatedTypeDescriptor]>,
    /// Declared associated constants in source order.
    associated_consts: Box<[AssociatedConstDescriptor]>,
}

impl TraitDefinitionDescriptor {
    /// Returns whether two declarations can be merged for one external trait
    /// ID.
    ///
    /// # Parameters
    ///
    /// - `other`: Declaration registered under the same external identity.
    ///
    /// # Returns
    ///
    /// Returns `true` when both declarations have compatible facts.
    pub(crate) fn is_compatible_with(&self, other: &Self) -> bool {
        self.completeness() == other.completeness() && self.generic_definition() == other.generic_definition()
    }

    /// Creates immutable trait definition facts.
    ///
    /// # Parameters
    ///
    /// * `trait_id`: identity of the reflected trait.
    /// * `rust_name`: source declaration name.
    /// * `rust_path`: fully qualified source path for diagnostics.
    /// * `query_name`: name used by reflection lookups.
    /// * `completeness`: extent of the declaration facts.
    /// * `generic_definition`: generic parameters and predicates.
    ///
    /// # Returns
    ///
    /// A definition with private visibility and uninitialized members.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        trait_id: TraitId,
        rust_name: &'static str,
        rust_path: &'static str,
        query_name: &'static str,
        completeness: TraitCompleteness,
        generic_definition: &'static GenericDefinitionDescriptor,
    ) -> Self {
        Self::new_with_visibility(
            trait_id,
            rust_name,
            rust_path,
            query_name,
            completeness,
            generic_definition,
            Visibility::Private,
        )
    }

    /// Creates immutable trait definition facts with normalized source
    /// visibility.
    ///
    /// # Parameters
    ///
    /// The parameters have the same meaning as [`Self::new`], with `visibility`
    /// recording the declaration's normalized source visibility.
    ///
    /// # Returns
    ///
    /// A definition retaining all supplied facts and uninitialized members.
    #[doc(hidden)]
    #[must_use]
    pub const fn new_with_visibility(
        trait_id: TraitId,
        rust_name: &'static str,
        rust_path: &'static str,
        query_name: &'static str,
        completeness: TraitCompleteness,
        generic_definition: &'static GenericDefinitionDescriptor,
        visibility: Visibility,
    ) -> Self {
        Self {
            trait_id,
            rust_name,
            rust_path,
            query_name,
            completeness,
            generic_definition,
            visibility,
            members: OnceLock::new(),
        }
    }

    /// Returns the trait declaration's normalized source visibility.
    ///
    /// # Returns
    ///
    /// Returns the source visibility retained by the descriptor.
    #[must_use]
    #[inline]
    pub const fn visibility(&self) -> &Visibility {
        &self.visibility
    }

    /// Returns the reflected marker or external trait identity.
    ///
    /// # Returns
    ///
    /// Returns the declaration identity.
    #[must_use]
    #[inline]
    pub const fn trait_id(&self) -> &TraitId {
        &self.trait_id
    }

    /// Returns the Rust declaration name.
    ///
    /// # Returns
    ///
    /// Returns the source identifier.
    #[must_use]
    #[inline]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the diagnostic fully qualified Rust path.
    ///
    /// # Returns
    ///
    /// Returns the source path.
    #[must_use]
    #[inline]
    pub const fn rust_path(&self) -> &'static str {
        self.rust_path
    }

    /// Returns the lookup name, which may differ from the Rust name.
    ///
    /// # Returns
    ///
    /// Returns the reflection query name.
    #[must_use]
    #[inline]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns whether the complete declaration is known.
    ///
    /// # Returns
    ///
    /// Returns the completeness classification.
    #[must_use]
    #[inline]
    pub const fn completeness(&self) -> TraitCompleteness {
        self.completeness
    }

    /// Returns generic parameters and predicates in source order.
    ///
    /// # Returns
    ///
    /// Returns the shared source generic definition.
    #[must_use]
    #[inline]
    pub const fn generic_definition(&self) -> &'static GenericDefinitionDescriptor {
        self.generic_definition
    }

    /// Returns methods declared by this trait in source order.
    ///
    /// # Returns
    ///
    /// Returns initialized declaration methods, or an empty slice before
    /// generated member initialization.
    #[must_use]
    pub fn methods(&self) -> &[MethodDescriptor] {
        self.members.get().map_or(&[], |members| members.methods.as_ref())
    }

    /// Returns associated types declared by this trait in source order.
    ///
    /// # Returns
    ///
    /// Returns initialized declaration associated types, or an empty slice
    /// before generated member initialization.
    #[must_use]
    #[inline]
    pub fn associated_types(&self) -> &[AssociatedTypeDescriptor] {
        self.members
            .get()
            .map_or(&[], |members| members.associated_types.as_ref())
    }

    /// Returns associated constants declared by this trait in source order.
    ///
    /// # Returns
    ///
    /// Returns initialized declaration associated constants, or an empty slice
    /// before generated member initialization.
    #[must_use]
    #[inline]
    pub fn associated_consts(&self) -> &[AssociatedConstDescriptor] {
        self.members
            .get()
            .map_or(&[], |members| members.associated_consts.as_ref())
    }

    /// Initializes declaration-level associated-item facts exactly once.
    ///
    /// # Parameters
    ///
    /// - `initialize`: Factory for methods, associated types, and constants.
    ///
    /// # Panics
    ///
    /// Panics if called after another initializer wins the one-time cell.
    #[doc(hidden)]
    pub fn initialize_members(
        &'static self,
        initialize: impl FnOnce(
            &'static Self,
        ) -> (
            Box<[MethodDescriptor]>,
            Box<[AssociatedTypeDescriptor]>,
            Box<[AssociatedConstDescriptor]>,
        ),
    ) {
        self.members.get_or_init(|| {
            let (methods, associated_types, associated_consts) = initialize(self);
            TraitDefinitionMembers {
                methods,
                associated_types,
                associated_consts,
            }
        });
    }
}
