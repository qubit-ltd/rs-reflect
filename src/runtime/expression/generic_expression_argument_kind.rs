// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::data::ConstExpression;
use super::data::LifetimeExpression;
use super::type_expression::TypeExpression;

/// Structural kind of a source-level generic argument.
pub enum GenericExpressionArgumentKind<'ctx> {
    /// A nested type argument.
    Type(
        /// Nested type expression supplied as the generic argument.
        TypeExpression<'ctx>,
    ),
    /// A lifetime argument.
    Lifetime(
        /// Lifetime value written in the source argument list.
        LifetimeExpression,
    ),
    /// A structural const argument.
    Const(
        /// Structural const value retained from the source expression.
        ConstExpression,
    ),
    /// An associated-type equality such as `Item = u8`.
    AssociatedType {
        /// Name of the associated type.
        name: &'static str,
        /// Type assigned to the associated type.
        value: TypeExpression<'ctx>,
    },
    /// An associated-type bound such as `Item: Copy`.
    AssociatedTypeBound {
        /// Name of the associated type.
        name: &'static str,
        /// Predicates recorded for the associated type.
        bounds: super::predicate::Predicates<'ctx>,
    },
}
