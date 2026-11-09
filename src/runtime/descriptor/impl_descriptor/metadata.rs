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
pub struct ImplDefinitionId(
    /// Function that returns the process-local identity of the impl marker.
    fn() -> TypeId,
);

impl ImplDefinitionId {
    /// Creates an identity from a generated marker type.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Marker type used to identify this source impl block.
    pub(crate) const fn of<T: 'static>() -> Self {
        Self(TypeId::of::<T>)
    }
    /// Returns the process-local marker type identity.
    ///
    /// # Parameters
    ///
    /// - `self`: Impl identity whose marker type is queried.
    ///
    /// # Returns
    ///
    /// Returns the `TypeId` assigned to the generated marker type.
    pub(crate) fn type_id(self) -> TypeId {
        (self.0)()
    }
}

impl std::fmt::Debug for ImplDefinitionId {
    /// Formats the process-local marker identity.
    ///
    /// # Parameters
    ///
    /// - `self`: Identity to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Errors
    ///
    /// Returns an error if writing the identity fails.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.type_id().fmt(formatter)
    }
}
impl PartialEq for ImplDefinitionId {
    /// Compares impl identities by their marker type IDs.
    ///
    /// # Parameters
    ///
    /// - `self`: First implementation identity.
    /// - `other`: Second implementation identity.
    ///
    /// # Returns
    ///
    /// Returns `true` when both identities name the same marker type.
    fn eq(&self, other: &Self) -> bool {
        self.type_id() == other.type_id()
    }
}
impl Eq for ImplDefinitionId {}
impl Hash for ImplDefinitionId {
    /// Hashes the marker type ID so equal identities share a hash key.
    ///
    /// # Parameters
    ///
    /// - `self`: Identity whose marker type ID is hashed.
    /// - `state`: Hasher receiving the identity.
    ///
    /// # Type Parameters
    ///
    /// - `H`: Hasher implementation that receives the marker identity.
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
    /// Whether the source implementation block is declared `unsafe`.
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
    /// Trait associated type declaration this binding implements.
    pub(crate) definition: &'static crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata,
    /// Source expression for the bound type.
    pub(crate) expression: &'static ExpressionData,
    /// Runtime type metadata accessor when the binding is concrete.
    pub(crate) concrete: Option<fn() -> &'static TypeMetadata>,
    /// Whether the binding is declared, defaulted, overridden, or unavailable.
    pub(crate) source: AssociatedItemSource,
}

/// Static metadata for a concrete associated constant binding.
#[derive(Clone, Copy)]
pub struct AssociatedConstBindingMetadata {
    /// Trait associated constant declaration this binding implements.
    pub(crate) definition: &'static crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata,
    /// Structural const expression retained for the binding.
    pub(crate) expression: crate::runtime::expression::ConstExpression,
    /// Whether the binding is declared, defaulted, overridden, or unavailable.
    pub(crate) source: AssociatedItemSource,
    /// Optional adapter for materializing the value in either ownership mode.
    pub(crate) value: Option<&'static AssociatedConstValueAdapter>,
}

/// Mode-specific constructor for a statically known associated constant value.
pub(crate) struct AssociatedConstValueAdapter {
    /// Local-mode constructor, when the value is available.
    pub(crate) local: Option<fn() -> DynamicOwned<Local>>,
    /// Thread-safe-mode constructor, when the value is available.
    pub(crate) thread_safe: Option<fn() -> DynamicOwned<ThreadSafe>>,
}

/// Static source facts for an `impl` declaration and context-visible
/// applications.
pub struct ImplDefinitionMetadata {
    /// Process-local identity for the source impl block.
    pub(crate) id: ImplDefinitionId,
    /// Whether the block is inherent or implements a trait.
    pub(crate) kind: ImplKind,
    /// Source type expression of the impl target.
    pub(crate) target: &'static ExpressionData,
    /// Trait path from the impl header, when this is a trait impl.
    pub(crate) implemented_trait: Option<&'static TraitExpressionMetadata>,
    /// Generic parameters and predicates declared by the impl.
    pub(crate) generics: &'static GenericDefinitionMetadata,
    /// Unsafe and other source qualifiers retained for the block.
    pub(crate) qualifiers: ImplQualifiers,
    /// Methods declared by the implementation.
    pub(crate) methods: &'static [MethodMetadata],
    /// Associated types declared by the implementation.
    pub(crate) associated_types:
        &'static [crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata],
    /// Associated constants declared by the implementation.
    pub(crate) associated_consts:
        &'static [crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata],
}

/// Static metadata for one compiled implementation application.
pub struct ImplApplicationMetadata {
    /// Source implementation declaration applied to this target.
    pub(crate) definition: &'static ImplDefinitionMetadata,
    /// Runtime metadata accessor for the concrete target type.
    pub(crate) target: fn() -> &'static TypeMetadata,
    /// Resolved trait application for a trait impl, when available.
    pub(crate) implemented_trait: Option<&'static TraitApplicationMetadata>,
    /// Generic arguments supplied by this application.
    pub(crate) arguments: &'static [GenericArgumentMetadata],
    /// Identity keys corresponding to the supplied generic arguments.
    pub(crate) identity: &'static [crate::runtime::descriptor::trait_descriptor::TraitArgumentIdentity],
    /// Effective methods after trait defaults and overrides are applied.
    pub(crate) methods: &'static [EffectiveMethod],
    /// Final associated type bindings for the application.
    pub(crate) associated_types: &'static [AssociatedTypeBindingMetadata],
    /// Final associated constant bindings for the application.
    pub(crate) associated_consts: &'static [AssociatedConstBindingMetadata],
}

impl ImplDefinitionMetadata {
    /// Creates source metadata bound to marker type `D`.
    ///
    /// # Type Parameters
    ///
    /// - `D`: Generated marker type identifying the source impl block.
    ///
    /// # Parameters
    ///
    /// - `kind`: Inherent or trait implementation category.
    /// - `target`: Source expression for the implemented target type.
    /// - `implemented_trait`: Trait expression from the impl header, if
    ///   present.
    /// - `generics`: Generic declarations and predicates on the impl.
    /// - `qualifiers`: Unsafe qualifier retained from the source block.
    /// - `methods`: Declared methods in source order.
    /// - `associated_types`: Associated type declarations.
    /// - `associated_consts`: Associated constant declarations.
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
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete target type whose `TypeMetadata` accessor is stored.
    ///
    /// # Parameters
    ///
    /// - `definition`: Source impl declaration applied to `T`.
    /// - `implemented_trait`: Resolved trait application, when this is a trait
    ///   impl.
    /// - `arguments`: Generic arguments supplied to the implementation.
    /// - `identity`: Identity keys for the supplied generic arguments.
    /// - `methods`: Effective methods visible for this application.
    /// - `associated_types`: Final associated type bindings.
    /// - `associated_consts`: Final associated constant bindings.
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
    ///
    /// # Parameters
    ///
    /// - `is_unsafe`: Whether the source `impl` is marked `unsafe`.
    pub const fn new(is_unsafe: bool) -> Self {
        Self { is_unsafe }
    }
}

impl AssociatedTypeBindingMetadata {
    /// Creates an associated type binding linked to its declaration metadata.
    ///
    /// # Parameters
    ///
    /// - `definition`: Associated type declaration this binding implements.
    /// - `expression`: Source type expression for the implementation binding.
    /// - `concrete`: Optional accessor for its concrete runtime type metadata.
    /// - `source`: Origin of the final binding in the implementation.
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
    ///
    /// # Parameters
    ///
    /// - `definition`: Associated constant declaration this binding implements.
    /// - `expression`: Structural const value retained from source syntax.
    /// - `source`: Origin of the final binding in the implementation.
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
