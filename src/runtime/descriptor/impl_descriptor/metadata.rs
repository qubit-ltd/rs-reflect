// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;
use std::hash::Hash;
use std::hash::Hasher;

use crate::runtime::descriptor::generic::GenericArgumentMetadata;
use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
use crate::runtime::descriptor::metadata::TypeMetadata;
use crate::runtime::descriptor::method::EffectiveMethod;
use crate::runtime::descriptor::method::MethodMetadata;
use crate::runtime::descriptor::trait_descriptor::TraitApplicationMetadata;
use crate::runtime::descriptor::trait_descriptor::TraitExpressionMetadata;
use crate::runtime::expression::ExpressionData;
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::Local;
use crate::runtime::value::ThreadSafe;

/// Process-local identity of one source `impl` block.
#[derive(Clone, Copy)]
pub struct ImplDefinitionId(fn() -> TypeId);

impl ImplDefinitionId {
    /// Creates an identity from a generated marker type.
    pub(crate) const fn of<T: 'static>() -> Self {
        Self(TypeId::of::<T>)
    }
    pub(crate) fn type_id(self) -> TypeId {
        (self.0)()
    }
}

impl std::fmt::Debug for ImplDefinitionId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.type_id().fmt(formatter)
    }
}
impl PartialEq for ImplDefinitionId {
    fn eq(&self, other: &Self) -> bool {
        self.type_id() == other.type_id()
    }
}
impl Eq for ImplDefinitionId {}
impl Hash for ImplDefinitionId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id().hash(state);
    }
}

/// Whether an implementation block is inherent or implements a trait.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImplKind {
    /// An inherent implementation with no trait path.
    Inherent,
    /// An implementation associated with a trait.
    Trait,
}

/// Qualifiers retained from an implementation block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImplQualifiers {
    pub(crate) is_unsafe: bool,
}

/// Origin of one final associated item binding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssociatedItemSource {
    /// The implementation explicitly defines the associated item.
    Declared,
    /// The implementation uses the trait item's default.
    Defaulted,
    /// The implementation replaces the trait item's default.
    Overridden,
    /// The required item has no usable binding in this application.
    RequiredButUnavailable,
}

/// Static metadata for a concrete associated type binding.
#[derive(Clone, Copy)]
pub struct AssociatedTypeBindingMetadata {
    pub(crate) definition: &'static crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata,
    pub(crate) expression: &'static ExpressionData,
    pub(crate) concrete: Option<fn() -> &'static TypeMetadata>,
    pub(crate) source: AssociatedItemSource,
}

/// Static metadata for a concrete associated constant binding.
#[derive(Clone, Copy)]
pub struct AssociatedConstBindingMetadata {
    pub(crate) definition: &'static crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata,
    pub(crate) expression: crate::runtime::expression::ConstExpression,
    pub(crate) source: AssociatedItemSource,
    pub(crate) value: Option<&'static AssociatedConstValueAdapter>,
}

/// Mode-specific constructor for a statically known associated constant value.
pub(crate) struct AssociatedConstValueAdapter {
    pub(crate) local: Option<fn() -> DynamicOwned<Local>>,
    pub(crate) thread_safe: Option<fn() -> DynamicOwned<ThreadSafe>>,
}

/// Static source facts for an `impl` declaration and context-visible
/// applications.
pub struct ImplDefinitionMetadata {
    pub(crate) id: ImplDefinitionId,
    pub(crate) kind: ImplKind,
    pub(crate) target: &'static ExpressionData,
    pub(crate) implemented_trait: Option<&'static TraitExpressionMetadata>,
    pub(crate) generics: &'static GenericDefinitionMetadata,
    pub(crate) qualifiers: ImplQualifiers,
    pub(crate) methods: &'static [MethodMetadata],
    pub(crate) associated_types:
        &'static [crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata],
    pub(crate) associated_consts:
        &'static [crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata],
}

/// Static metadata for one compiled implementation application.
pub struct ImplApplicationMetadata {
    pub(crate) definition: &'static ImplDefinitionMetadata,
    pub(crate) target: fn() -> &'static TypeMetadata,
    pub(crate) implemented_trait: Option<&'static TraitApplicationMetadata>,
    pub(crate) arguments: &'static [GenericArgumentMetadata],
    pub(crate) identity: &'static [crate::runtime::descriptor::trait_descriptor::TraitArgumentIdentity],
    pub(crate) methods: &'static [EffectiveMethod],
    pub(crate) associated_types: &'static [AssociatedTypeBindingMetadata],
    pub(crate) associated_consts: &'static [AssociatedConstBindingMetadata],
}

impl ImplDefinitionMetadata {
    /// Creates source metadata bound to marker type `D`.
    #[allow(clippy::too_many_arguments)]
    pub const fn new<D: 'static>(
        kind: ImplKind,
        target: &'static ExpressionData,
        implemented_trait: Option<&'static TraitExpressionMetadata>,
        generics: &'static GenericDefinitionMetadata,
        qualifiers: ImplQualifiers,
        methods: &'static [MethodMetadata],
        associated_types: &'static [crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata],
        associated_consts: &'static [crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata],
    ) -> Self {
        Self {
            id: ImplDefinitionId::of::<D>(),
            kind,
            target,
            implemented_trait,
            generics,
            qualifiers,
            methods,
            associated_types,
            associated_consts,
        }
    }
}

impl ImplApplicationMetadata {
    /// Creates an application for the exact target type `T`.
    #[allow(clippy::too_many_arguments)]
    pub const fn for_target<T: 'static>(
        definition: &'static ImplDefinitionMetadata,
        implemented_trait: Option<&'static TraitApplicationMetadata>,
        arguments: &'static [GenericArgumentMetadata],
        identity: &'static [crate::runtime::descriptor::trait_descriptor::TraitArgumentIdentity],
        methods: &'static [EffectiveMethod],
        associated_types: &'static [AssociatedTypeBindingMetadata],
        associated_consts: &'static [AssociatedConstBindingMetadata],
    ) -> Self {
        Self {
            definition,
            target: TypeMetadata::of::<T>,
            implemented_trait,
            arguments,
            identity,
            methods,
            associated_types,
            associated_consts,
        }
    }
}

impl ImplQualifiers {
    /// Creates source qualifiers for an implementation block.
    pub const fn new(is_unsafe: bool) -> Self {
        Self { is_unsafe }
    }
}

impl AssociatedTypeBindingMetadata {
    /// Creates an associated type binding linked to its declaration metadata.
    pub const fn new(
        definition: &'static crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata,
        expression: &'static ExpressionData,
        concrete: Option<fn() -> &'static TypeMetadata>,
        source: AssociatedItemSource,
    ) -> Self {
        Self {
            definition,
            expression,
            concrete,
            source,
        }
    }
}

impl AssociatedConstBindingMetadata {
    /// Creates a typed associated constant binding without a dynamic value
    /// adapter.
    pub const fn new(
        definition: &'static crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata,
        expression: crate::runtime::expression::ConstExpression,
        source: AssociatedItemSource,
    ) -> Self {
        Self {
            definition,
            expression,
            source,
            value: None,
        }
    }
}
