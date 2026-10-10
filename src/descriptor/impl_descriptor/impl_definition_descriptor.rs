// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! ImplDefinitionDescriptor metadata and behavior.

mod impl_associated_items;

use std::sync::OnceLock;

use self::impl_associated_items::ImplAssociatedItems;
use super::impl_associated_const_descriptor::ImplAssociatedConstDescriptor;
use super::impl_associated_type_descriptor::ImplAssociatedTypeDescriptor;
use super::impl_kind::validate_kind;
use crate::descriptor::ImplDescriptorBuildError;
use crate::descriptor::ImplKind;
use crate::descriptor::MethodDescriptor;
use crate::descriptor::TraitDefinitionDescriptor;
use crate::descriptor::TraitId;
use crate::expression::GenericDefinitionDescriptor;
use crate::expression::TypeExpression;
use crate::identity::FragmentIdentity;
use crate::registry::ReflectRegistry;

/// Declaration facts for a generic, blanket, or concrete impl block.
///
/// This descriptor is constructed by generated registration code.
///
/// # Examples
///
/// Most applications get this declaration metadata from `#[reflect_impl]`.
/// The example builds the source declaration explicitly so it does not need
/// to initialize the global inventory registry.
///
/// ```
/// mod example {
///     use std::sync::LazyLock;
///     use qubit_reflect::descriptor::ImplDefinitionDescriptor;
///     use qubit_reflect::descriptor::ImplKind;
///     use qubit_reflect::expression::ConcreteTypeExpression;
///     use qubit_reflect::expression::GenericDefinitionDescriptor;
///     use qubit_reflect::expression::TypeExpression;
///     use qubit_reflect::identity::FragmentIdentity;
///
///     static GENERICS: LazyLock<GenericDefinitionDescriptor> =
///         LazyLock::new(|| GenericDefinitionDescriptor::new([], []));
///     pub static DEFINITION: LazyLock<ImplDefinitionDescriptor> = LazyLock::new(|| {
///         ImplDefinitionDescriptor::new(
///             FragmentIdentity::new("example", module_path!(), line!(), 1, "impl", 1),
///             TypeExpression::Concrete(
///                 ConcreteTypeExpression::new(["example", "Service"], [])
///                     .expect("non-empty target path"),
///             ),
///             ImplKind::Inherent,
///             None,
///             &GENERICS,
///         )
///         .expect("valid inherent impl")
///     });
/// }
/// assert_eq!(example::DEFINITION.kind(), qubit_reflect::descriptor::ImplKind::Inherent);
/// assert!(matches!(
///     example::DEFINITION.target_type(),
///     qubit_reflect::expression::TypeExpression::Concrete(_),
/// ));
/// ```
#[derive(Debug)]
pub struct ImplDefinitionDescriptor {
    /// Stable identity of the source impl fragment.
    fragment_identity: FragmentIdentity,
    /// Target expression, which may contain generic parameters.
    target_type: TypeExpression,
    /// Whether the source impl is inherent or implements a trait.
    kind: ImplKind,
    /// Resolved trait declaration, when it is already available.
    implemented_trait: Option<&'static TraitDefinitionDescriptor>,
    /// Exact trait identity retained for registry linking.
    implemented_trait_id: Option<TraitId>,
    /// Diagnostic path retained even when the trait declaration is unresolved.
    implemented_trait_path: Option<Box<str>>,
    /// Generic parameters and predicates declared by this impl.
    generic_definition: &'static GenericDefinitionDescriptor,
    /// Lazily initialized methods in source order.
    methods: OnceLock<Box<[MethodDescriptor]>>,
    /// Lazily initialized associated-item facts.
    associated_items: OnceLock<ImplAssociatedItems>,
}

impl ImplDefinitionDescriptor {
    /// Creates an impl definition without claiming a concrete target instance.
    ///
    /// Returns [`ImplDescriptorBuildError`] when `kind` and
    /// `implemented_trait` disagree.
    ///
    /// # Parameters
    ///
    /// - `fragment_identity`: Stable source identity for this impl fragment.
    /// - `target_type`: Target type expression, possibly containing parameters.
    /// - `kind`: Whether the impl is inherent or a trait impl.
    /// - `implemented_trait`: Trait declaration for a trait impl, or `None` for
    ///   an inherent impl.
    /// - `generic_definition`: Generic parameters and predicates declared by
    ///   the impl.
    ///
    /// # Returns
    ///
    /// Returns the initialized declaration descriptor.
    ///
    /// # Errors
    ///
    /// Returns [`ImplDescriptorBuildError::InherentImplHasTrait`] when an
    /// inherent impl names a trait,
    /// or [`ImplDescriptorBuildError::TraitImplMissingTrait`] when a trait impl
    /// has no trait declaration.
    #[doc(hidden)]
    pub fn new(
        fragment_identity: FragmentIdentity,
        target_type: TypeExpression,
        kind: ImplKind,
        implemented_trait: Option<&'static TraitDefinitionDescriptor>,
        generic_definition: &'static GenericDefinitionDescriptor,
    ) -> Result<Self, ImplDescriptorBuildError> {
        validate_kind(kind, implemented_trait.is_some())?;
        Ok(Self {
            fragment_identity,
            target_type,
            kind,
            implemented_trait,
            implemented_trait_id: implemented_trait.map(|descriptor| descriptor.trait_id().clone()),
            implemented_trait_path: implemented_trait.map(|descriptor| descriptor.rust_path().into()),
            generic_definition,
            methods: OnceLock::new(),
            associated_items: OnceLock::new(),
        })
    }

    /// Creates a trait impl definition whose declaration link is resolved by
    /// the immutable registry after all trait fragments have been collected.
    ///
    /// # Parameters
    ///
    /// - `fragment_identity`: Stable source identity for this impl fragment.
    /// - `target_type`: Target type expression, possibly containing parameters.
    /// - `implemented_trait_path`: Diagnostic Rust path for the trait
    ///   declaration.
    /// - `implemented_trait_id`: Exact trait identity when available.
    /// - `generic_definition`: Generic parameters and predicates declared by
    ///   the impl.
    ///
    /// # Returns
    ///
    /// Returns an unresolved trait impl declaration for later registry linking.
    #[doc(hidden)]
    pub fn new_unresolved_trait(
        fragment_identity: FragmentIdentity,
        target_type: TypeExpression,
        implemented_trait_path: impl Into<Box<str>>,
        implemented_trait_id: Option<TraitId>,
        generic_definition: &'static GenericDefinitionDescriptor,
    ) -> Self {
        Self {
            fragment_identity,
            target_type,
            kind: ImplKind::Trait,
            implemented_trait: None,
            implemented_trait_id,
            implemented_trait_path: Some(implemented_trait_path.into()),
            generic_definition,
            methods: OnceLock::new(),
            associated_items: OnceLock::new(),
        }
    }

    /// Returns the source/content identity of this impl fragment.
    ///
    /// # Returns
    ///
    /// Returns the stable source identity.
    #[must_use]
    #[inline]
    pub const fn fragment_identity(&self) -> &FragmentIdentity {
        &self.fragment_identity
    }

    /// Returns the possibly symbolic target type expression.
    ///
    /// # Returns
    ///
    /// Returns the target type expression.
    #[must_use]
    #[inline]
    pub const fn target_type(&self) -> &TypeExpression {
        &self.target_type
    }

    /// Returns whether this definition is inherent or implements a trait.
    ///
    /// # Returns
    ///
    /// Returns the implementation kind.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> ImplKind {
        self.kind
    }

    /// Returns the implemented trait definition.
    ///
    /// `None` identifies an inherent impl or an unresolved trait declaration.
    /// Use [`Self::implemented_trait_in`] for snapshot-resolved links.
    ///
    /// # Returns
    ///
    /// Returns the linked trait declaration, or `None` when this definition is
    /// inherent or unresolved.
    #[must_use]
    #[inline]
    pub fn implemented_trait(&self) -> Option<&'static TraitDefinitionDescriptor> {
        self.implemented_trait
    }

    /// Returns the diagnostic trait path recorded by the impl declaration.
    ///
    /// # Returns
    ///
    /// Returns the recorded trait path, or `None` for an inherent impl.
    #[must_use]
    #[inline]
    pub fn implemented_trait_path(&self) -> Option<&str> {
        self.implemented_trait_path.as_deref()
    }

    /// Returns an exact trait identity supplied by the impl declaration when
    /// one is available before registry linking.
    ///
    /// # Returns
    ///
    /// Returns the exact trait identity, or `None` when it was not supplied.
    #[must_use]
    #[inline]
    pub fn implemented_trait_id(&self) -> Option<&TraitId> {
        self.implemented_trait_id.as_ref()
    }

    /// Returns the trait link resolved in `registry`, or `None` when this
    /// definition has no trait link in that snapshot. Never initializes a
    /// global registry or changes this declaration.
    ///
    /// # Parameters
    ///
    /// - `registry`: Immutable registry snapshot used to resolve the trait
    ///   link.
    ///
    /// # Returns
    ///
    /// Returns the snapshot-resolved trait declaration, or `None` when
    /// unavailable.
    #[must_use]
    pub fn implemented_trait_in(
        &self,
        registry: &ReflectRegistry,
    ) -> Option<&'static TraitDefinitionDescriptor> {
        registry.impl_definition_trait(self)
    }

    /// Returns generic parameters and predicates in source order.
    ///
    /// # Returns
    ///
    /// Returns this impl's generic definition.
    #[must_use]
    #[inline]
    pub const fn generic_definition(&self) -> &'static GenericDefinitionDescriptor {
        self.generic_definition
    }

    /// Returns methods declared by this impl definition in source order.
    ///
    /// # Returns
    ///
    /// Returns the initialized methods, or an empty slice before
    /// initialization.
    #[must_use]
    #[inline]
    pub fn methods(&self) -> &[MethodDescriptor] {
        self.methods.get().map_or(&[], Box::as_ref)
    }

    /// Returns associated types explicitly bound by this impl in source order.
    ///
    /// # Returns
    ///
    /// Returns the initialized associated type bindings, or an empty slice
    /// before initialization.
    #[must_use = "the associated type declarations describe this impl"]
    #[inline]
    pub fn associated_types(&self) -> &[ImplAssociatedTypeDescriptor] {
        self.associated_items.get().map_or(&[], |items| items.types.as_ref())
    }

    /// Returns associated constants explicitly bound by this impl in source
    /// order.
    ///
    /// # Returns
    ///
    /// Returns the initialized associated constant bindings, or an empty slice
    /// before initialization.
    #[must_use = "the associated constant declarations describe this impl"]
    #[inline]
    pub fn associated_consts(&self) -> &[ImplAssociatedConstDescriptor] {
        self.associated_items.get().map_or(&[], |items| items.consts.as_ref())
    }

    /// Initializes declaration-level methods exactly once.
    ///
    /// # Parameters
    ///
    /// - `initialize`: Callback that builds methods from this static
    ///   definition.
    ///
    /// # Returns
    ///
    /// Returns `()`; subsequent calls leave the initialized value unchanged.
    #[doc(hidden)]
    pub fn initialize_methods(&'static self, initialize: impl FnOnce(&'static Self) -> Box<[MethodDescriptor]>) {
        self.methods.get_or_init(|| initialize(self));
    }

    /// Initializes declaration-level associated-item facts exactly once.
    ///
    /// # Parameters
    ///
    /// - `initialize`: Callback that builds associated types and constants from
    ///   this static definition.
    ///
    /// # Returns
    ///
    /// Returns `()`; subsequent calls leave the initialized value unchanged.
    #[doc(hidden)]
    pub fn initialize_associated_items(
        &'static self,
        initialize: impl FnOnce(
            &'static Self,
        ) -> (
            Box<[ImplAssociatedTypeDescriptor]>,
            Box<[ImplAssociatedConstDescriptor]>,
        ),
    ) {
        self.associated_items.get_or_init(|| {
            let (types, consts) = initialize(self);
            ImplAssociatedItems { types, consts }
        });
    }
}
