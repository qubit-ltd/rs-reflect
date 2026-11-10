// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::data::GenericExpressionArgument;
use super::generic_expression_arguments::GenericExpressionArguments;
use super::trait_expression::TraitExpression;
use super::type_expression::TypeExpression;
use crate::runtime::context::ReflectionContext;

/// A projection such as `<T as Iterator>::Item`.
#[derive(Clone, Copy)]
pub struct AssociatedTypeExpression<'ctx> {
    /// Type expression on the left side of the projection.
    pub(super) self_type: TypeExpression<'ctx>,
    /// Optional trait path qualifying the projection.
    pub(super) trait_: Option<TraitExpression<'ctx>>,
    /// Projected associated-type identifier.
    pub(super) name: &'static str,
    /// Arguments applied to the associated type.
    pub(super) arguments: &'static [GenericExpressionArgument],
}

impl<'ctx> AssociatedTypeExpression<'ctx> {
    /// Returns the context used by the projected type and trait.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.self_type.context()
    }

    /// Returns the type from which the associated type is projected.
    pub fn self_type(self) -> TypeExpression<'ctx> {
        self.self_type
    }
    /// Returns the trait qualifier, or `None` for an unqualified projection.
    pub fn trait_(self) -> Option<TraitExpression<'ctx>> {
        self.trait_
    }
    /// Returns the projected associated-type name.
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Returns generic arguments written on the projected associated type.
    pub fn arguments(self) -> GenericExpressionArguments<'ctx> {
        GenericExpressionArguments::new(self.arguments, self.context())
    }
}
