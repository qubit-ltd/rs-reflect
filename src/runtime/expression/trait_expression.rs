// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::associated_type_constraints::AssociatedTypeConstraints;
use super::data::LifetimeExpression;
use super::data::TraitExpressionData;
use super::generic_expression_arguments::GenericExpressionArguments;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::trait_descriptor::TraitDefinitionDescriptor;

/// A trait path attached to a projection or trait object.
#[derive(Clone, Copy)]
pub struct TraitExpression<'ctx> {
    /// Retained path and generic arguments for the trait expression.
    pub(super) data: &'static TraitExpressionData,
    /// Context used by nested generic argument views.
    pub(super) context: &'ctx ReflectionContext,
}

impl<'ctx> TraitExpression<'ctx> {
    /// Binds retained trait-expression data to its reflection context.
    ///
    /// # Parameters
    ///
    /// - `data`: Static source path and arguments for the trait expression.
    /// - `context`: Context inherited by nested argument views.
    pub(crate) const fn new(data: &'static TraitExpressionData, context: &'ctx ReflectionContext) -> Self {
        Self { data, context }
    }

    /// Returns the context used by nested trait argument views.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Returns the trait path as written in the source expression.
    pub const fn path(self) -> &'static str {
        self.data.path
    }

    /// Returns the definition associated with this expression, if available.
    ///
    /// This source expression carries only a path, not a trait identity, so a
    /// matching path in the context is not sufficient to establish a link.
    pub fn find_definition(self) -> Option<TraitDefinitionDescriptor<'ctx>> {
        None
    }

    /// Returns explicit lifetime arguments in source order.
    pub const fn lifetimes(self) -> &'static [LifetimeExpression] {
        self.data.lifetimes
    }
    /// Returns generic and associated-type arguments in source order.
    pub fn arguments(self) -> GenericExpressionArguments<'ctx> {
        GenericExpressionArguments::new(self.data.arguments, self.context)
    }

    /// Returns associated-type equality and bound facts from the source path.
    pub fn associated_type_constraints(self) -> AssociatedTypeConstraints<'ctx> {
        AssociatedTypeConstraints {
            arguments: self.data.arguments,
            context: self.context,
        }
    }
}
