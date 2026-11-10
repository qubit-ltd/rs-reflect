// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::predicate::Predicates;
use super::type_expression::TypeExpression;

/// The source form of one associated-type constraint.
#[derive(Clone, Copy)]
pub enum AssociatedTypeConstraintKind<'ctx> {
    /// Assigns one type expression to the associated type, as in `Item = u8`.
    Equality(
        /// Type expression assigned to the associated type.
        TypeExpression<'ctx>,
    ),
    /// Records one or more predicates on the associated type, as in `Item:
    /// Copy`.
    Bounds(
        /// Predicates applied to the associated type.
        Predicates<'ctx>,
    ),
}
