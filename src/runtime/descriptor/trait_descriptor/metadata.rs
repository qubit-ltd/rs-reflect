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
use crate::runtime::descriptor::method::MethodMetadata;
use crate::runtime::expression::ExpressionData;
use crate::runtime::expression::PredicateData;
use crate::runtime::expression::TraitExpressionData;

/// Process-local identity of a reflected trait declaration.
#[derive(Clone, Copy)]
pub struct TraitDefinitionId(fn() -> TypeId);

impl TraitDefinitionId {
    /// Creates an identity from a generated marker type.
    pub(crate) const fn of<T: 'static>() -> Self {
        Self(TypeId::of::<T>)
    }

    pub(crate) fn type_id(self) -> TypeId {
        (self.0)()
    }
}

impl std::fmt::Debug for TraitDefinitionId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.type_id().fmt(formatter)
    }
}

impl PartialEq for TraitDefinitionId {
    fn eq(&self, other: &Self) -> bool {
        self.type_id() == other.type_id()
    }
}

impl Eq for TraitDefinitionId {}

impl Hash for TraitDefinitionId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id().hash(state);
    }
}

/// Whether complete source metadata is known for a trait.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraitCompleteness {
    /// The trait declaration and its reflected items are fully described.
    Complete,
    /// Only an external trait path or incomplete declaration facts are known.
    ExternalIncomplete,
}

/// Completeness of a resolved trait's transitive supertrait closure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupertraitCompleteness {
    /// All reachable supertrait applications are retained.
    Complete,
    /// Some supertrait applications could not be resolved or retained.
    Incomplete,
}

/// Visibility retained from a Rust declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Visibility {
    /// The item is public.
    Public,
    /// The item is restricted to the named module path.
    Restricted(&'static str),
    /// The source visibility could not be resolved precisely.
    RestrictedUnknown,
    /// The item is private to its defining module.
    Private,
}

/// Static source facts for an associated type declaration.
#[derive(Clone, Copy)]
pub struct AssociatedTypeDefinitionMetadata {
    pub(crate) name: &'static str,
    pub(crate) visibility: Visibility,
    pub(crate) bounds: &'static [PredicateData],
    pub(crate) default: Option<&'static ExpressionData>,
}

/// Static source facts for an associated constant declaration.
#[derive(Clone, Copy)]
pub struct AssociatedConstDefinitionMetadata {
    pub(crate) name: &'static str,
    pub(crate) visibility: Visibility,
    pub(crate) ty: &'static ExpressionData,
    pub(crate) default: Option<crate::runtime::expression::ConstExpression>,
}

/// A trait expression occurring in a declaration's source syntax.
#[derive(Clone, Copy)]
pub struct TraitExpressionMetadata {
    pub(crate) expression: &'static TraitExpressionData,
    pub(crate) definition: Option<&'static TraitDefinitionMetadata>,
}

/// One concrete, target-independent application of a trait.
#[derive(Clone, Copy)]
pub struct TraitApplicationMetadata {
    pub(crate) definition: Option<&'static TraitDefinitionMetadata>,
    pub(crate) expression: Option<&'static TraitExpressionData>,
    pub(crate) arguments: &'static [GenericArgumentMetadata],
    pub(crate) identity: &'static [TraitArgumentIdentity],
    pub(crate) direct_supertraits: &'static [&'static TraitApplicationMetadata],
    pub(crate) all_supertraits: &'static [&'static TraitApplicationMetadata],
    pub(crate) supertrait_completeness: SupertraitCompleteness,
}

impl TraitApplicationMetadata {
    /// Creates an application linked to marker-bound declaration metadata.
    pub const fn new(
        definition: &'static TraitDefinitionMetadata,
        arguments: &'static [GenericArgumentMetadata],
        identity: &'static [TraitArgumentIdentity],
        direct_supertraits: &'static [&'static TraitApplicationMetadata],
        all_supertraits: &'static [&'static TraitApplicationMetadata],
        supertrait_completeness: SupertraitCompleteness,
    ) -> Self {
        Self {
            definition: Some(definition),
            expression: None,
            arguments,
            identity,
            direct_supertraits,
            all_supertraits,
            supertrait_completeness,
        }
    }

    /// Creates a linked application while retaining the path written at its
    /// source.
    pub const fn linked(
        definition: &'static TraitDefinitionMetadata,
        expression: &'static TraitExpressionData,
        arguments: &'static [GenericArgumentMetadata],
        identity: &'static [TraitArgumentIdentity],
        direct_supertraits: &'static [&'static TraitApplicationMetadata],
        all_supertraits: &'static [&'static TraitApplicationMetadata],
        supertrait_completeness: SupertraitCompleteness,
    ) -> Self {
        Self {
            definition: Some(definition),
            expression: Some(expression),
            arguments,
            identity,
            direct_supertraits,
            all_supertraits,
            supertrait_completeness,
        }
    }

    /// Creates an application that retains source syntax without associating a
    /// trait definition. A matching path in a context does not establish trait
    /// identity; use [`Self::linked`] when the definition is known.
    pub const fn unresolved(
        expression: &'static TraitExpressionData,
        arguments: &'static [GenericArgumentMetadata],
        identity: &'static [TraitArgumentIdentity],
    ) -> Self {
        Self {
            definition: None,
            expression: Some(expression),
            arguments,
            identity,
            direct_supertraits: &[],
            all_supertraits: &[],
            supertrait_completeness: SupertraitCompleteness::Incomplete,
        }
    }
}

/// Const-safe key used to distinguish compiled generic applications.
#[derive(Clone, Copy)]
pub enum TraitArgumentIdentity {
    /// A concrete type argument identified by its `TypeId`.
    Type(fn() -> TypeId),
    /// A structural const argument.
    Const(crate::runtime::expression::ConstExpression),
    /// A retained lifetime argument.
    Lifetime(&'static str),
    /// A source expression used when the argument cannot be typed concretely.
    Expression(&'static str),
}

impl PartialEq for TraitArgumentIdentity {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Type(left), Self::Type(right)) => left() == right(),
            (Self::Const(left), Self::Const(right)) => left == right,
            // Lifetime spellings are source metadata, not runtime type
            // identity. Rust does not expose a stable runtime token for a
            // lifetime argument, so every lifetime slot is a wildcard key.
            (Self::Lifetime(_), Self::Lifetime(_)) => true,
            (Self::Expression(left), Self::Expression(right)) => left == right,
            _ => false,
        }
    }
}
impl Eq for TraitArgumentIdentity {}

/// Static source facts for a trait declaration and applications visible to a
/// context.
pub struct TraitDefinitionMetadata {
    pub(crate) id: TraitDefinitionId,
    pub(crate) name: &'static str,
    pub(crate) path: &'static str,
    pub(crate) visibility: Visibility,
    pub(crate) completeness: TraitCompleteness,
    pub(crate) generics: &'static GenericDefinitionMetadata,
    pub(crate) supertraits: &'static [TraitExpressionMetadata],
    pub(crate) methods: &'static [MethodMetadata],
    pub(crate) associated_types: &'static [AssociatedTypeDefinitionMetadata],
    pub(crate) associated_consts: &'static [AssociatedConstDefinitionMetadata],
}

impl TraitDefinitionMetadata {
    /// Creates declaration metadata bound to marker type `D`.
    #[allow(clippy::too_many_arguments)]
    pub const fn new<D: 'static>(
        name: &'static str,
        path: &'static str,
        visibility: Visibility,
        completeness: TraitCompleteness,
        generics: &'static GenericDefinitionMetadata,
        supertraits: &'static [TraitExpressionMetadata],
        methods: &'static [MethodMetadata],
        associated_types: &'static [AssociatedTypeDefinitionMetadata],
        associated_consts: &'static [AssociatedConstDefinitionMetadata],
    ) -> Self {
        Self {
            id: TraitDefinitionId::of::<D>(),
            name,
            path,
            visibility,
            completeness,
            generics,
            supertraits,
            methods,
            associated_types,
            associated_consts,
        }
    }
}

impl TraitExpressionMetadata {
    /// Retains a trait expression when its declaration is external or
    /// unresolved.
    pub const fn unresolved(expression: &'static TraitExpressionData) -> Self {
        Self {
            expression,
            definition: None,
        }
    }

    /// Associates an expression with the marker-bound local declaration.
    pub const fn for_definition(
        expression: &'static TraitExpressionData,
        definition: &'static TraitDefinitionMetadata,
    ) -> Self {
        Self {
            expression,
            definition: Some(definition),
        }
    }
}

impl TraitArgumentIdentity {
    /// Binds an application argument identity to concrete Rust type `T`.
    pub const fn type_argument<T: ?Sized + 'static>() -> Self {
        Self::Type(TypeId::of::<T>)
    }

    /// Records a const argument value as its typed identity.
    pub const fn const_argument(value: crate::runtime::expression::ConstExpression) -> Self {
        Self::Const(value)
    }

    /// Records a lifetime argument without exposing an identity token.
    pub const fn lifetime(value: &'static str) -> Self {
        Self::Lifetime(value)
    }

    /// Records an unresolved source argument expression.
    pub const fn expression(value: &'static str) -> Self {
        Self::Expression(value)
    }
}

impl AssociatedTypeDefinitionMetadata {
    /// Creates typed source metadata for a trait associated type.
    pub const fn new(
        name: &'static str,
        visibility: Visibility,
        bounds: &'static [PredicateData],
        default: Option<&'static ExpressionData>,
    ) -> Self {
        Self {
            name,
            visibility,
            bounds,
            default,
        }
    }
}

impl AssociatedConstDefinitionMetadata {
    /// Creates typed source metadata for a trait associated constant.
    pub const fn new(
        name: &'static str,
        visibility: Visibility,
        ty: &'static ExpressionData,
        default: Option<crate::runtime::expression::ConstExpression>,
    ) -> Self {
        Self {
            name,
            visibility,
            ty,
            default,
        }
    }
}
