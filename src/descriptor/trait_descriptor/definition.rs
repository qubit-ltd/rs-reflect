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
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraitCompleteness {
    /// The trait declaration, supertraits, and associated items are known.
    Complete,
    /// Only facts proven by an observed external trait impl are known.
    ExternalIncomplete,
}

/// Declaration-level facts shared by every concrete application of a trait.
#[derive(Debug)]
pub struct TraitDefinitionDescriptor {
    trait_id: TraitId,
    rust_name: &'static str,
    rust_path: &'static str,
    query_name: &'static str,
    completeness: TraitCompleteness,
    generic_definition: &'static GenericDefinitionDescriptor,
    visibility: Visibility,
    members: OnceLock<TraitDefinitionMembers>,
}

/// Associated-item facts retained before a concrete trait application exists.
#[derive(Debug)]
struct TraitDefinitionMembers {
    methods: Box<[MethodDescriptor]>,
    associated_types: Box<[AssociatedTypeDescriptor]>,
    associated_consts: Box<[AssociatedConstDescriptor]>,
}

impl TraitDefinitionDescriptor {
    /// Returns whether two declarations can be merged for one external trait
    /// ID.
    pub(crate) fn is_compatible_with(&self, other: &Self) -> bool {
        self.completeness() == other.completeness() && self.generic_definition() == other.generic_definition()
    }

    /// Creates immutable trait definition facts.
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
    #[must_use]
    #[inline(always)]
    pub const fn visibility(&self) -> &Visibility {
        &self.visibility
    }

    /// Returns the reflected marker or external trait identity.
    #[must_use]
    #[inline(always)]
    pub const fn trait_id(&self) -> &TraitId {
        &self.trait_id
    }

    /// Returns the Rust declaration name.
    #[must_use]
    #[inline(always)]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the diagnostic fully qualified Rust path.
    #[must_use]
    #[inline(always)]
    pub const fn rust_path(&self) -> &'static str {
        self.rust_path
    }

    /// Returns the lookup name, which may differ from the Rust name.
    #[must_use]
    #[inline(always)]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns whether the complete declaration is known.
    #[must_use]
    #[inline(always)]
    pub const fn completeness(&self) -> TraitCompleteness {
        self.completeness
    }

    /// Returns generic parameters and predicates in source order.
    #[must_use]
    #[inline(always)]
    pub const fn generic_definition(&self) -> &'static GenericDefinitionDescriptor {
        self.generic_definition
    }

    /// Returns methods declared by this trait in source order.
    #[must_use]
    pub fn methods(&self) -> &[MethodDescriptor] {
        self.members.get().map_or(&[], |members| members.methods.as_ref())
    }

    /// Returns associated types declared by this trait in source order.
    #[must_use]
    #[inline(always)]
    pub fn associated_types(&self) -> &[AssociatedTypeDescriptor] {
        self.members
            .get()
            .map_or(&[], |members| members.associated_types.as_ref())
    }

    /// Returns associated constants declared by this trait in source order.
    #[must_use]
    #[inline(always)]
    pub fn associated_consts(&self) -> &[AssociatedConstDescriptor] {
        self.members
            .get()
            .map_or(&[], |members| members.associated_consts.as_ref())
    }

    /// Initializes declaration-level associated-item facts exactly once.
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
#[derive(Clone, Debug)]
pub struct AssociatedTypeDescriptor {
    index: usize,
    rust_name: &'static str,
    query_name: &'static str,
    bounds: Box<[PredicateDescriptor]>,
    default: Option<TypeExpression>,
    generic_definition: Option<Box<GenericDefinitionDescriptor>>,
}

impl AssociatedTypeDescriptor {
    /// Creates associated type facts in declaration order.
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
    #[must_use]
    #[inline(always)]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the Rust declaration name.
    #[must_use]
    #[inline(always)]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the lookup name.
    #[must_use]
    #[inline(always)]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns declared bounds in source order.
    #[must_use]
    #[inline(always)]
    pub const fn bounds(&self) -> &[PredicateDescriptor] {
        &self.bounds
    }

    /// Returns GAT parameters and where predicates in declaration order.
    #[must_use]
    #[inline(always)]
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
    #[must_use]
    #[inline(always)]
    pub const fn default(&self) -> Option<&TypeExpression> {
        self.default.as_ref()
    }

    /// Applies one concrete trait application to this declaration.
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
#[derive(Clone, Debug)]
pub struct AssociatedConstDescriptor {
    index: usize,
    rust_name: &'static str,
    query_name: &'static str,
    declared_type: TypeExpression,
    has_default: bool,
}

impl AssociatedConstDescriptor {
    /// Creates associated constant facts in declaration order.
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
    #[must_use]
    #[inline(always)]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the Rust declaration name.
    #[must_use]
    #[inline(always)]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the lookup name.
    #[must_use]
    #[inline(always)]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns the declared constant type.
    #[must_use]
    #[inline(always)]
    pub const fn declared_type(&self) -> &TypeExpression {
        &self.declared_type
    }

    /// Returns whether the trait declaration provides a default value.
    #[must_use]
    #[inline(always)]
    pub const fn has_default(&self) -> bool {
        self.has_default
    }

    /// Applies one concrete trait application to this declaration.
    pub(super) fn substituted(self, substitutions: &TraitApplicationSubstitutions) -> Self {
        Self {
            declared_type: substitutions.type_expression(&self.declared_type),
            ..self
        }
    }
}
