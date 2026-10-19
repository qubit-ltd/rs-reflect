mod data;
mod error;
mod predicate;
mod type_expression;

pub use data::{
    ConstExpression, ExpressionData, FunctionPointerData, GenericExpressionArgument,
    LifetimeExpression, OpaqueTypeData, PredicateData, TraitBoundModifier, TraitExpressionData,
    TraitObjectData,
};
pub(crate) use error::{GenericArgumentError, GenericArgumentKind, TypeExpressionError};
pub(crate) use predicate::{PredicateDescriptor, PredicateKind, Predicates};
pub(crate) use type_expression::{
    AssociatedTypeExpression, FunctionPointerExpression, GenericExpressionArgumentKind,
    GenericExpressionArgumentView, NamedTypeExpression, OpaqueTypeExpression, TraitExpression,
    TraitObjectExpression, TypeExpression, TypeExpressionKind,
};

#[cfg(test)]
#[path = "../tests/expression.rs"]
mod tests;
