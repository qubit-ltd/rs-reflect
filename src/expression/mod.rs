// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural, parser-independent representations of Rust types and generics.

mod expression_error;
mod expression_name;
mod expression_path;
mod generic_argument;
mod generic_definition_descriptor;
mod generic_parameter_descriptor;
mod lifetime_expression;
mod predicate_descriptor;
mod trait_bound_modifier;
mod type_expression;

pub use expression_error::ExpressionError;
pub use expression_name::ExpressionName;
pub use expression_path::ExpressionPath;
pub use generic_argument::ConstExpression;
pub use generic_argument::ConstGenericArgument;
pub use generic_argument::GenericArgument;
pub use generic_definition_descriptor::GenericDefinitionDescriptor;
pub use generic_parameter_descriptor::GenericParameterDescriptor;
pub use lifetime_expression::LifetimeExpression;
pub use predicate_descriptor::PredicateDescriptor;
pub use trait_bound_modifier::TraitBoundModifier;
pub use type_expression::ArrayTypeExpression;
pub use type_expression::AssociatedTypeExpression;
pub use type_expression::ConcretePathSegment;
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
