// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::data::LifetimeExpression;
use super::data::PredicateData;
use super::data::TraitBoundModifier;
use super::type_expression::TraitExpression;
use super::type_expression::TypeExpression;
use crate::runtime::context::ReflectionContext;

/// Context-bound view of a declaration predicate.
#[derive(Clone, Copy)]
pub struct PredicateDescriptor<'ctx> {
    data: &'static PredicateData,
    context: &'ctx ReflectionContext,
}

impl<'ctx> PredicateDescriptor<'ctx> {
    pub(crate) const fn new(data: &'static PredicateData, context: &'ctx ReflectionContext) -> Self {
        Self { data, context }
    }

    /// Returns the context used by nested type and trait expressions.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Returns the structural predicate fact represented by this descriptor.
    pub fn kind(self) -> PredicateKind<'ctx> {
        match *self.data {
            PredicateData::TraitBound { ty, trait_, modifier } => PredicateKind::TraitBound {
                ty: TypeExpression::new(ty, self.context),
                trait_: TraitExpression::new(trait_, self.context),
                modifier,
            },
            PredicateData::LifetimeOutlives { lifetime, outlives } => {
                PredicateKind::LifetimeOutlives { lifetime, outlives }
            }
            PredicateData::TypeOutlives { ty, lifetime } => PredicateKind::TypeOutlives {
                ty: TypeExpression::new(ty, self.context),
                lifetime,
            },
            PredicateData::TypeEquality { left, right } => PredicateKind::TypeEquality {
                left: TypeExpression::new(left, self.context),
                right: TypeExpression::new(right, self.context),
            },
            PredicateData::ConstEvaluatable(expression) => PredicateKind::ConstEvaluatable(expression),
        }
    }
}

/// Structural source fact represented by a generic predicate.
pub enum PredicateKind<'ctx> {
    /// A type constrained to implement a trait.
    TraitBound {
        /// Type subject to the bound.
        ty: TypeExpression<'ctx>,
        /// Required trait and its source arguments.
        trait_: TraitExpression<'ctx>,
        /// Whether the bound uses Rust's `?Trait` modifier.
        modifier: TraitBoundModifier,
    },
    /// A lifetime required to outlive another lifetime.
    LifetimeOutlives {
        /// Lifetime required to outlive `outlives`.
        lifetime: LifetimeExpression,
        /// Lifetime that `lifetime` must outlive.
        outlives: LifetimeExpression,
    },
    /// A type required to outlive a lifetime.
    TypeOutlives {
        /// Type subject to the outlives requirement.
        ty: TypeExpression<'ctx>,
        /// Lifetime that the type must outlive.
        lifetime: LifetimeExpression,
    },
    /// Two type expressions required to be equal.
    TypeEquality {
        /// Left side of the equality predicate.
        left: TypeExpression<'ctx>,
        /// Right side of the equality predicate.
        right: TypeExpression<'ctx>,
    },
    /// A structural const expression retained from a const-evaluatable bound.
    ConstEvaluatable(super::data::ConstExpression),
}

/// Ordered context-bound view over predicate metadata.
#[derive(Clone, Copy)]
pub struct Predicates<'ctx> {
    data: &'static [PredicateData],
    context: &'ctx ReflectionContext,
}

impl<'ctx> Predicates<'ctx> {
    pub(crate) const fn new(data: &'static [PredicateData], context: &'ctx ReflectionContext) -> Self {
        Self { data, context }
    }

    /// Returns the context used by nested predicate expressions.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Iterates over predicates in declaration/source order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = PredicateDescriptor<'ctx>> + 'ctx {
        self.data
            .iter()
            .map(move |data| PredicateDescriptor::new(data, self.context))
    }

    /// Returns the number of source predicates.
    pub const fn len(self) -> usize {
        self.data.len()
    }

    /// Returns whether this predicate list is empty.
    pub fn is_empty(self) -> bool {
        self.data.is_empty()
    }
}
