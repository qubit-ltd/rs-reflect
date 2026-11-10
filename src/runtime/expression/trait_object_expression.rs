// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::data::LifetimeExpression;
use super::data::TraitObjectData;
use super::trait_expression::TraitExpression;
use crate::runtime::context::ReflectionContext;

/// Facts retained for a trait-object expression.
#[derive(Clone, Copy)]
pub struct TraitObjectExpression<'ctx> {
    /// Trait bounds and optional object lifetime retained from source.
    pub(super) data: &'static TraitObjectData,
    /// Context used by the yielded trait expression views.
    pub(super) context: &'ctx ReflectionContext,
}

impl<'ctx> TraitObjectExpression<'ctx> {
    /// Returns the context used by the trait descriptors.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Returns the explicit object lifetime, if one was written.
    pub const fn lifetime(self) -> Option<LifetimeExpression> {
        self.data.lifetime
    }
    /// Returns trait bounds in source order.
    pub fn traits(self) -> Box<[TraitExpression<'ctx>]> {
        self.data
            .traits
            .iter()
            .map(|data| TraitExpression {
                data,
                context: self.context,
            })
            .collect()
    }
}
