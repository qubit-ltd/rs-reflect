use std::any::TypeId;
use std::hash::{Hash, Hasher};

use crate::runtime::descriptor::generic::{GenericArgumentMetadata, GenericDefinitionMetadata};
use crate::runtime::descriptor::method::MethodMetadata;
use crate::runtime::expression::{ExpressionData, PredicateData, TraitExpressionData};

/// Process-local identity of a reflected trait declaration.
#[derive(Clone, Copy)]
pub(crate) struct TraitDefinitionId(fn() -> TypeId);

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
pub(crate) enum TraitCompleteness {
    Complete,
    ExternalIncomplete,
}

/// Completeness of a resolved trait's transitive supertrait closure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SupertraitCompleteness {
    Complete,
    Incomplete,
}

/// Visibility retained from a Rust declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Visibility {
    Public,
    Restricted(&'static str),
    RestrictedUnknown,
    Private,
}

/// Static source facts for an associated type declaration.
#[derive(Clone, Copy)]
pub(crate) struct AssociatedTypeDefinitionMetadata {
    pub(crate) name: &'static str,
    pub(crate) visibility: Visibility,
    pub(crate) bounds: &'static [PredicateData],
    pub(crate) default: Option<&'static ExpressionData>,
}

/// Static source facts for an associated constant declaration.
#[derive(Clone, Copy)]
pub(crate) struct AssociatedConstDefinitionMetadata {
    pub(crate) name: &'static str,
    pub(crate) visibility: Visibility,
    pub(crate) ty: &'static ExpressionData,
    pub(crate) default: Option<crate::runtime::expression::ConstExpression>,
}

/// A trait expression occurring in a declaration's source syntax.
#[derive(Clone, Copy)]
pub(crate) struct TraitExpressionMetadata {
    pub(crate) expression: &'static TraitExpressionData,
    pub(crate) definition: Option<&'static TraitDefinitionMetadata>,
}

/// One concrete, target-independent application of a trait.
#[derive(Clone, Copy)]
pub(crate) struct TraitApplicationMetadata {
    pub(crate) definition: &'static TraitDefinitionMetadata,
    pub(crate) arguments: &'static [GenericArgumentMetadata],
    pub(crate) identity: &'static [TraitArgumentIdentity],
    pub(crate) direct_supertraits: &'static [&'static TraitApplicationMetadata],
    pub(crate) all_supertraits: &'static [&'static TraitApplicationMetadata],
    pub(crate) supertrait_completeness: SupertraitCompleteness,
}

/// Const-safe key used to distinguish compiled generic applications.
#[derive(Clone, Copy)]
pub(crate) enum TraitArgumentIdentity {
    Type(fn() -> TypeId),
    Const(crate::runtime::expression::ConstExpression),
    Lifetime(&'static str),
    Expression(&'static str),
}

impl PartialEq for TraitArgumentIdentity {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Type(left), Self::Type(right)) => left() == right(),
            (Self::Const(left), Self::Const(right)) => left == right,
            (Self::Lifetime(left), Self::Lifetime(right))
            | (Self::Expression(left), Self::Expression(right)) => left == right,
            _ => false,
        }
    }
}
impl Eq for TraitArgumentIdentity {}

/// Static source facts for a trait declaration and applications visible to a context.
pub(crate) struct TraitDefinitionMetadata {
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
