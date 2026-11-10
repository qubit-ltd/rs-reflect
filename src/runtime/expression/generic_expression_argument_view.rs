// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::data::GenericExpressionArgument;
use super::generic_expression_argument_kind::GenericExpressionArgumentKind;
use super::type_expression::TypeExpression;
use crate::runtime::context::ReflectionContext;

/// A context-bound view of one named, lifetime, const, or associated argument.
#[derive(Clone, Copy)]
pub struct GenericExpressionArgumentView<'ctx> {
    /// Structural source argument represented by this view.
    data: GenericExpressionArgument,
    /// Context used when the argument contains nested expressions.
    context: &'ctx ReflectionContext,
}

impl<'ctx> GenericExpressionArgumentView<'ctx> {
    /// Binds one retained argument to the context used by nested views.
    ///
    /// # Parameters
    ///
    /// - `data`: Retained generic argument to expose.
    /// - `context`: Context inherited by nested type and predicate views.
    pub(super) const fn new(data: GenericExpressionArgument, context: &'ctx ReflectionContext) -> Self {
        Self { data, context }
    }

    /// Returns the context used by nested expressions and predicates.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Returns the structural form of this argument, binding nested views to
    /// its context.
    pub fn kind(self) -> GenericExpressionArgumentKind<'ctx> {
        match self.data {
            GenericExpressionArgument::Type(ty) => {
                GenericExpressionArgumentKind::Type(TypeExpression::new(ty, self.context))
            }
            GenericExpressionArgument::Lifetime(value) => GenericExpressionArgumentKind::Lifetime(value),
            GenericExpressionArgument::Const(value) => GenericExpressionArgumentKind::Const(value),
            GenericExpressionArgument::AssociatedType { name, value } => {
                GenericExpressionArgumentKind::AssociatedType {
                    name,
                    value: TypeExpression::new(value, self.context),
                }
            }
            GenericExpressionArgument::AssociatedTypeBound { name, bounds } => {
                GenericExpressionArgumentKind::AssociatedTypeBound {
                    name,
                    bounds: super::predicate::Predicates::new(bounds, self.context),
                }
            }
        }
    }
}
