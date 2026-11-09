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
pub struct TraitDefinitionId(
    /// Function that returns the process-local identity of the marker type.
    fn() -> TypeId,
);

impl TraitDefinitionId {
    /// Creates an identity from a generated marker type.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Marker type whose `TypeId` identifies the reflected declaration.
    pub(crate) const fn of<T: 'static>() -> Self {
        Self(TypeId::of::<T>)
    }

    /// Returns the process-local marker type identity.
    ///
    /// # Parameters
    ///
    /// - `self`: Declaration identity whose marker type is queried.
    ///
    /// # Returns
    ///
    /// Returns the `TypeId` assigned to the generated marker type.
    pub(crate) fn type_id(self) -> TypeId {
        (self.0)()
    }
}

impl std::fmt::Debug for TraitDefinitionId {
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

impl PartialEq for TraitDefinitionId {
    /// Compares identities by their marker type IDs.
    ///
    /// # Parameters
    ///
    /// - `self`: First declaration identity.
    /// - `other`: Second declaration identity.
    ///
    /// # Returns
    ///
    /// Returns `true` when both identities name the same marker type.
    fn eq(&self, other: &Self) -> bool {
        self.type_id() == other.type_id()
    }
}

impl Eq for TraitDefinitionId {}

impl Hash for TraitDefinitionId {
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
    Restricted(
        /// Module path that limits where the item is visible.
        &'static str,
    ),
    /// The source visibility could not be resolved precisely.
    RestrictedUnknown,
    /// The item is private to its defining module.
    Private,
}

/// Static source facts for an associated type declaration.
#[derive(Clone, Copy)]
pub struct AssociatedTypeDefinitionMetadata {
    /// Source identifier of the associated type.
    pub(crate) name: &'static str,
    /// Visibility declared for the associated type.
    pub(crate) visibility: Visibility,
    /// Bounds declared on the associated type.
    pub(crate) bounds: &'static [PredicateData],
    /// Default type expression, when the declaration provides one.
    pub(crate) default: Option<&'static ExpressionData>,
}

/// Static source facts for an associated constant declaration.
#[derive(Clone, Copy)]
pub struct AssociatedConstDefinitionMetadata {
    /// Source identifier of the associated constant.
    pub(crate) name: &'static str,
    /// Visibility declared for the associated constant.
    pub(crate) visibility: Visibility,
    /// Type expression declared for the constant.
    pub(crate) ty: &'static ExpressionData,
    /// Structural default value, when the declaration provides one.
    pub(crate) default: Option<crate::runtime::expression::ConstExpression>,
}

/// A trait expression occurring in a declaration's source syntax.
#[derive(Clone, Copy)]
pub struct TraitExpressionMetadata {
    /// Trait path, generic arguments, and lifetime facts from source syntax.
    pub(crate) expression: &'static TraitExpressionData,
    /// Explicitly linked declaration metadata, when known.
    pub(crate) definition: Option<&'static TraitDefinitionMetadata>,
}

/// One concrete, target-independent application of a trait.
#[derive(Clone, Copy)]
pub struct TraitApplicationMetadata {
    /// Linked trait declaration, or `None` when identity is unresolved.
    pub(crate) definition: Option<&'static TraitDefinitionMetadata>,
    /// Original source path and arguments, when retained.
    pub(crate) expression: Option<&'static TraitExpressionData>,
    /// Concrete generic arguments in declaration order.
    pub(crate) arguments: &'static [GenericArgumentMetadata],
    /// Stable identity keys for the supplied arguments.
    pub(crate) identity: &'static [TraitArgumentIdentity],
    /// Direct supertrait applications.
    pub(crate) direct_supertraits: &'static [&'static TraitApplicationMetadata],
    /// Transitive supertrait applications in the resolved closure.
    pub(crate) all_supertraits: &'static [&'static TraitApplicationMetadata],
    /// Whether the retained supertrait closure is complete.
    pub(crate) supertrait_completeness: SupertraitCompleteness,
}

impl TraitApplicationMetadata {
    /// Creates an application linked to marker-bound declaration metadata.
    ///
    /// # Parameters
    ///
    /// - `definition`: Trait declaration linked to its marker type.
    /// - `arguments`: Concrete generic arguments for this application.
    /// - `identity`: Identity key for each generic argument.
    /// - `direct_supertraits`: Directly declared supertrait applications.
    /// - `all_supertraits`: Resolved transitive supertrait closure.
    /// - `supertrait_completeness`: Completeness of the retained closure.
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
    ///
    /// # Parameters
    ///
    /// - `definition`: Linked trait declaration metadata.
    /// - `expression`: Source expression retaining the path and arguments.
    /// - `arguments`: Concrete generic arguments for this application.
    /// - `identity`: Identity key for each generic argument.
    /// - `direct_supertraits`: Directly declared supertrait applications.
    /// - `all_supertraits`: Resolved transitive supertrait closure.
    /// - `supertrait_completeness`: Completeness of the retained closure.
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
    ///
    /// # Parameters
    ///
    /// - `expression`: Source path and generic arguments for this application.
    /// - `arguments`: Generic argument values retained for the application.
    /// - `identity`: Structural identity keys for the supplied arguments.
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
    Type(
        /// TypeId-producing function for the concrete argument type.
        fn() -> TypeId,
    ),
    /// A structural const argument.
    Const(
        /// Structural const value used as the argument identity.
        crate::runtime::expression::ConstExpression,
    ),
    /// A retained lifetime argument.
    Lifetime(
        /// Source lifetime spelling retained for diagnostics and display.
        &'static str,
    ),
    /// A source expression used when the argument cannot be typed concretely.
    Expression(
        /// Source expression spelling used when no concrete identity exists.
        &'static str,
    ),
}

impl PartialEq for TraitArgumentIdentity {
    /// Compares argument identities according to their structural category.
    ///
    /// Lifetime slots are treated as wildcards because Rust exposes no stable
    /// runtime identity token for lifetime arguments.
    ///
    /// # Parameters
    ///
    /// - `self`: First argument identity.
    /// - `other`: Second argument identity.
    ///
    /// # Returns
    ///
    /// Returns whether the two keys identify compatible trait applications.
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
    /// Process-local marker identity for this declaration.
    pub(crate) id: TraitDefinitionId,
    /// Trait identifier as declared in source.
    pub(crate) name: &'static str,
    /// Qualified source path of the trait.
    pub(crate) path: &'static str,
    /// Visibility retained from the declaration.
    pub(crate) visibility: Visibility,
    /// Whether declaration and item metadata are complete.
    pub(crate) completeness: TraitCompleteness,
    /// Generic parameters and where predicates of the trait.
    pub(crate) generics: &'static GenericDefinitionMetadata,
    /// Declared supertrait expressions in source order.
    pub(crate) supertraits: &'static [TraitExpressionMetadata],
    /// Methods declared by the trait.
    pub(crate) methods: &'static [MethodMetadata],
    /// Associated types declared by the trait.
    pub(crate) associated_types: &'static [AssociatedTypeDefinitionMetadata],
    /// Associated constants declared by the trait.
    pub(crate) associated_consts: &'static [AssociatedConstDefinitionMetadata],
}

impl TraitDefinitionMetadata {
    /// Creates declaration metadata bound to marker type `D`.
    ///
    /// # Type Parameters
    ///
    /// - `D`: Generated marker type that provides this declaration's identity.
    ///
    /// # Parameters
    ///
    /// - `name`: Trait identifier from the declaration.
    /// - `path`: Qualified source path of the trait.
    /// - `visibility`: Visibility retained from source.
    /// - `completeness`: Whether the complete declaration was reflected.
    /// - `generics`: Generic parameter and predicate metadata.
    /// - `supertraits`: Source supertrait expressions.
    /// - `methods`: Method metadata in declaration order.
    /// - `associated_types`: Associated type metadata.
    /// - `associated_consts`: Associated constant metadata.
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
    ///
    /// # Parameters
    ///
    /// - `expression`: Source trait expression without a known declaration
    ///   link.
    pub const fn unresolved(expression: &'static TraitExpressionData) -> Self {
        Self {
            expression,
            definition: None,
        }
    }

    /// Associates an expression with the marker-bound local declaration.
    ///
    /// # Parameters
    ///
    /// - `expression`: Source trait path and argument expression.
    /// - `definition`: Local declaration explicitly linked to the expression.
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
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete type used for the argument identity.
    pub const fn type_argument<T: ?Sized + 'static>() -> Self {
        Self::Type(TypeId::of::<T>)
    }

    /// Records a const argument value as its typed identity.
    ///
    /// # Parameters
    ///
    /// - `value`: Structural const value retained for the argument.
    pub const fn const_argument(value: crate::runtime::expression::ConstExpression) -> Self {
        Self::Const(value)
    }

    /// Records a lifetime argument without exposing an identity token.
    ///
    /// # Parameters
    ///
    /// - `value`: Lifetime spelling retained from source syntax.
    pub const fn lifetime(value: &'static str) -> Self {
        Self::Lifetime(value)
    }

    /// Records an unresolved source argument expression.
    ///
    /// # Parameters
    ///
    /// - `value`: Source spelling used when concrete argument identity is
    ///   unavailable.
    pub const fn expression(value: &'static str) -> Self {
        Self::Expression(value)
    }
}

impl AssociatedTypeDefinitionMetadata {
    /// Creates typed source metadata for a trait associated type.
    ///
    /// # Parameters
    ///
    /// - `name`: Associated type identifier.
    /// - `visibility`: Visibility retained from source.
    /// - `bounds`: Declared bounds in source order.
    /// - `default`: Default type expression, when declared.
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
    ///
    /// # Parameters
    ///
    /// - `name`: Associated constant identifier.
    /// - `visibility`: Visibility retained from source.
    /// - `ty`: Declared type expression.
    /// - `default`: Structural default value, when representable.
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
