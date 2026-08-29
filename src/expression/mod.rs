//! Structural, parser-independent representations of Rust types and generics.

mod generic_argument;
mod generic_definition;
mod lifetime_expression;
mod predicate;
mod type_expression;

pub use generic_argument::{ConstExpression, ConstGenericArgument, GenericArgument};
pub use generic_definition::{GenericDefinitionDescriptor, GenericParameterDescriptor};
pub use lifetime_expression::LifetimeExpression;
pub use predicate::PredicateDescriptor;
pub use type_expression::{
    ArrayTypeExpression, AssociatedTypeExpression, ConcreteTypeExpression, DiagnosticText,
    FunctionAbi, FunctionPointerExpression, FunctionSafety, OpaqueTypeExpression,
    RawPointerTypeExpression, ReferenceTypeExpression, TraitObjectExpression, TypeExpression,
};
