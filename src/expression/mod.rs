//! Structural, parser-independent representations of Rust types and generics.

mod generic_argument;
mod generic_definition;
mod lifetime_expression;
mod predicate;
mod type_expression;

pub use generic_argument::ConstExpression;
pub use generic_argument::ConstGenericArgument;
pub use generic_argument::GenericArgument;
pub use generic_definition::GenericDefinitionDescriptor;
pub use generic_definition::GenericParameterDescriptor;
pub use lifetime_expression::LifetimeExpression;
pub use predicate::PredicateDescriptor;
pub use predicate::TraitBoundModifier;
pub use type_expression::ArrayTypeExpression;
pub use type_expression::AssociatedTypeExpression;
pub use type_expression::ConcreteTypeExpression;
pub use type_expression::DiagnosticText;
pub use type_expression::FunctionAbi;
pub use type_expression::FunctionPointerExpression;
pub use type_expression::FunctionSafety;
pub use type_expression::OpaqueTypeExpression;
pub use type_expression::RawPointerTypeExpression;
pub use type_expression::ReferenceTypeExpression;
pub use type_expression::TraitObjectExpression;
pub use type_expression::TypeExpression;
