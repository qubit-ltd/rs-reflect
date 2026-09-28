// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Trait declarations and their associated items.

use std::sync::LazyLock;
use std::sync::OnceLock;

use super::TraitApplicationSubstitutions;
use super::TraitId;
use crate::descriptor::MethodDescriptor;
use crate::expression::GenericDefinitionDescriptor;
use crate::expression::PredicateDescriptor;
use crate::expression::TypeExpression;
use crate::identity::Visibility;

/// How much of a trait declaration is known to reflection.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::TraitCompleteness;
///
/// assert_ne!(TraitCompleteness::Complete, TraitCompleteness::ExternalIncomplete);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraitCompleteness {
    /// The trait declaration, supertraits, and associated items are known.
    Complete,
    /// Only facts proven by an observed external trait impl are known.
    ExternalIncomplete,
}

/// Declaration-level facts shared by every concrete application of a trait.
///
/// This descriptor is created by generated metadata and shared by all
/// applications of the same trait declaration.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "derive")]
/// mod example {
///     use qubit_reflect;
///     use qubit_reflect::reflect;
///
///     #[reflect(crate = ::qubit_reflect)]
///     pub trait ExampleService {
///         fn run(&self);
///     }
/// }
///
/// # #[cfg(feature = "derive")]
/// let registry = qubit_reflect::ReflectRegistry::initialize()
///     .expect("valid reflection registry");
/// # #[cfg(feature = "derive")]
/// let definition = registry
///     .trait_definition_by_path(concat!(module_path!(), "::example::ExampleService"))
///     .expect("reflected trait definition");
/// # #[cfg(feature = "derive")]
/// assert_eq!(definition.query_name(), "ExampleService");
/// # #[cfg(feature = "derive")]
/// assert_eq!(definition.methods()[0].rust_name(), "run");
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

/// One associated type declaration.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::AssociatedTypeDescriptor;
///
/// let item = AssociatedTypeDescriptor::new(0, "Item", "item", Box::new([]), None);
/// assert_eq!(item.query_name(), "item");
/// ```
#[derive(Clone, Debug)]
pub struct AssociatedTypeDescriptor {
    /// Zero-based source position among associated types.
    index: usize,
    /// Rust declaration identifier.
    rust_name: &'static str,
    /// Reflection lookup name.
    query_name: &'static str,
    /// Declared bounds in source order.
    bounds: Box<[PredicateDescriptor]>,
    /// Optional symbolic default type.
    default: Option<TypeExpression>,
    /// Optional GAT parameters and predicates.
    generic_definition: Option<Box<GenericDefinitionDescriptor>>,
}

impl AssociatedTypeDescriptor {
    /// Creates associated type facts in declaration order.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based source declaration position.
    /// - `rust_name`: Source Rust identifier.
    /// - `query_name`: Reflection lookup name.
    /// - `bounds`: Declared bounds in source order.
    /// - `default`: Optional symbolic default type.
    ///
    /// # Returns
    ///
    /// Returns associated-type facts without GAT parameters.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        index: usize,
        rust_name: &'static str,
        query_name: &'static str,
        bounds: Box<[PredicateDescriptor]>,
        default: Option<TypeExpression>,
    ) -> Self {
        Self {
            index,
            rust_name,
            query_name,
            bounds,
            default,
            generic_definition: None,
        }
    }

    /// Creates associated type facts with GAT parameters and predicates.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based source declaration position.
    /// - `rust_name`: Source Rust identifier.
    /// - `query_name`: Reflection lookup name.
    /// - `bounds`: Declared bounds in source order.
    /// - `default`: Optional symbolic default type.
    /// - `generic_definition`: GAT parameters and where predicates.
    ///
    /// # Returns
    ///
    /// Returns associated-type facts with the supplied GAT definition.
    #[doc(hidden)]
    #[must_use]
    pub fn new_with_generic_definition(
        index: usize,
        rust_name: &'static str,
        query_name: &'static str,
        bounds: Box<[PredicateDescriptor]>,
        default: Option<TypeExpression>,
        generic_definition: GenericDefinitionDescriptor,
    ) -> Self {
        Self {
            index,
            rust_name,
            query_name,
            bounds,
            default,
            generic_definition: Some(Box::new(generic_definition)),
        }
    }

    /// Returns the source declaration index.
    ///
    /// # Returns
    ///
    /// Returns the zero-based position among associated types.
    #[must_use]
    #[inline]
    pub const fn index(&self) -> usize {
        self.index
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

    /// Returns the lookup name.
    ///
    /// # Returns
    ///
    /// Returns the reflection query name.
    #[must_use]
    #[inline]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns declared bounds in source order.
    ///
    /// # Returns
    ///
    /// Returns the declared predicate bounds.
    #[must_use]
    #[inline]
    pub const fn bounds(&self) -> &[PredicateDescriptor] {
        &self.bounds
    }

    /// Returns GAT parameters and where predicates in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the GAT definition, or an empty definition for an ordinary
    /// associated type.
    #[must_use]
    pub fn generic_definition(&self) -> &GenericDefinitionDescriptor {
        static EMPTY: LazyLock<GenericDefinitionDescriptor> = LazyLock::new(|| GenericDefinitionDescriptor {
            parameters: Box::new([]),
            predicates: Box::new([]),
            diagnostic: crate::expression::DiagnosticText::default(),
        });
        self.generic_definition.as_deref().unwrap_or(&EMPTY)
    }

    /// Returns the symbolic default type.
    ///
    /// `None` means the trait requires implementations to provide the binding.
    ///
    /// # Returns
    ///
    /// Returns the symbolic default type, or `None` when no default exists.
    #[must_use]
    #[inline]
    pub const fn default(&self) -> Option<&TypeExpression> {
        self.default.as_ref()
    }

    /// Applies one concrete trait application to this declaration.
    ///
    /// # Parameters
    ///
    /// - `substitutions`: Concrete generic and associated-type substitutions.
    ///
    /// # Returns
    ///
    /// Returns this declaration with its type expressions substituted.
    pub(super) fn substituted(self, substitutions: &TraitApplicationSubstitutions) -> Self {
        Self {
            bounds: self
                .bounds
                .iter()
                .map(|predicate| substitutions.predicate(predicate))
                .collect(),
            default: self
                .default
                .as_ref()
                .map(|expression| substitutions.type_expression(expression)),
            generic_definition: self
                .generic_definition
                .as_ref()
                .map(|definition| Box::new(substitutions.generic_definition(definition))),
            ..self
        }
    }
}

/// One associated constant declaration.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::AssociatedConstDescriptor;
/// use qubit_reflect::expression::{ConcreteTypeExpression, TypeExpression};
///
/// let item = AssociatedConstDescriptor::new(
///     0,
///     "LIMIT",
///     "limit",
///     TypeExpression::Concrete(
///         ConcreteTypeExpression::new(["usize"], []).expect("non-empty path"),
///     ),
///     false,
/// );
/// assert_eq!(item.query_name(), "limit");
/// ```
#[derive(Clone, Debug)]
pub struct AssociatedConstDescriptor {
    /// Zero-based source position among associated constants.
    index: usize,
    /// Rust declaration identifier.
    rust_name: &'static str,
    /// Reflection lookup name.
    query_name: &'static str,
    /// Declared structural type.
    declared_type: TypeExpression,
    /// Whether the trait declaration provides a default value.
    has_default: bool,
}

impl AssociatedConstDescriptor {
    /// Creates associated constant facts in declaration order.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based source declaration position.
    /// - `rust_name`: Source Rust identifier.
    /// - `query_name`: Reflection lookup name.
    /// - `declared_type`: Structural type of the constant.
    /// - `has_default`: Whether the declaration supplies a value.
    ///
    /// # Returns
    ///
    /// Returns immutable associated-constant facts.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        index: usize,
        rust_name: &'static str,
        query_name: &'static str,
        declared_type: TypeExpression,
        has_default: bool,
    ) -> Self {
        Self {
            index,
            rust_name,
            query_name,
            declared_type,
            has_default,
        }
    }

    /// Returns the source declaration index.
    ///
    /// # Returns
    ///
    /// Returns the zero-based position among associated constants.
    #[must_use]
    #[inline]
    pub const fn index(&self) -> usize {
        self.index
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

    /// Returns the lookup name.
    ///
    /// # Returns
    ///
    /// Returns the reflection query name.
    #[must_use]
    #[inline]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns the declared constant type.
    ///
    /// # Returns
    ///
    /// Returns the structural declared type.
    #[must_use]
    #[inline]
    pub const fn declared_type(&self) -> &TypeExpression {
        &self.declared_type
    }

    /// Returns whether the trait declaration provides a default value.
    ///
    /// # Returns
    ///
    /// Returns `true` when the source trait provides a default.
    #[must_use]
    #[inline]
    pub const fn has_default(&self) -> bool {
        self.has_default
    }

    /// Applies one concrete trait application to this declaration.
    ///
    /// # Parameters
    ///
    /// - `substitutions`: Concrete generic substitutions.
    ///
    /// # Returns
    ///
    /// Returns this declaration with its type expression substituted.
    pub(super) fn substituted(self, substitutions: &TraitApplicationSubstitutions) -> Self {
        Self {
            declared_type: substitutions.type_expression(&self.declared_type),
            ..self
        }
    }
}
