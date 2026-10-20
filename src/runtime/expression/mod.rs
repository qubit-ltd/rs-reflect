// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

mod data;
mod error;
mod predicate;
mod type_expression;

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
pub use predicate::PredicateDescriptor;
pub use predicate::PredicateKind;
pub use predicate::Predicates;
pub use type_expression::AssociatedTypeConstraintDescriptor;
pub use type_expression::AssociatedTypeConstraintKind;
pub use type_expression::AssociatedTypeConstraints;
pub use type_expression::AssociatedTypeExpression;
pub use type_expression::FunctionPointerExpression;
pub use type_expression::GenericExpressionArgumentKind;
pub use type_expression::GenericExpressionArgumentView;
pub use type_expression::GenericExpressionArguments;
pub use type_expression::NamedTypeExpression;
pub use type_expression::OpaqueTypeExpression;
pub use type_expression::TraitExpression;
pub use type_expression::TraitObjectExpression;
pub use type_expression::TypeExpression;
pub use type_expression::TypeExpressionKind;
