// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

mod associated_type_constraint_descriptor;
mod associated_type_constraint_kind;
mod associated_type_constraints;
mod associated_type_expression;
mod data;
mod error;
mod function_pointer_expression;
mod generic_expression_argument_kind;
mod generic_expression_argument_view;
mod generic_expression_arguments;
mod internal;
mod named_type_expression;
mod opaque_type_expression;
mod predicate;
mod structural_eq;
mod trait_expression;
mod trait_object_expression;
mod tuple_type_expressions;
mod type_expression;
mod type_expression_kind;

pub use associated_type_constraint_descriptor::AssociatedTypeConstraintDescriptor;
pub use associated_type_constraint_kind::AssociatedTypeConstraintKind;
pub use associated_type_constraints::AssociatedTypeConstraints;
pub use associated_type_expression::AssociatedTypeExpression;
pub use data::ConstExpression;
pub use data::ExpressionData;
pub use data::FunctionPointerData;
pub use data::GenericExpressionArgument;
pub use data::LifetimeExpression;
pub use data::OpaqueTypeData;
pub use data::PredicateData;
pub use data::TraitBoundModifier;
pub use data::TraitExpressionData;
pub use data::TraitObjectData;
pub use error::GenericArgumentError;
pub use error::GenericArgumentKind;
pub use error::TypeExpressionError;
pub use function_pointer_expression::FunctionPointerExpression;
pub use generic_expression_argument_kind::GenericExpressionArgumentKind;
pub use generic_expression_argument_view::GenericExpressionArgumentView;
pub use generic_expression_arguments::GenericExpressionArguments;
pub use named_type_expression::NamedTypeExpression;
pub use opaque_type_expression::OpaqueTypeExpression;
pub use predicate::PredicateDescriptor;
pub use predicate::PredicateKind;
pub use predicate::Predicates;
pub use trait_expression::TraitExpression;
pub use trait_object_expression::TraitObjectExpression;
pub use tuple_type_expressions::TupleTypeExpressions;
pub use type_expression::TypeExpression;
pub use type_expression_kind::TypeExpressionKind;
