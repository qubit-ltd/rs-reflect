use crate::runtime::context::ReflectionContext;

use super::data::{LifetimeExpression, PredicateData, TraitBoundModifier};
use super::type_expression::{TraitExpression, TypeExpression};

/// Context-bound view of a declaration predicate.
#[derive(Clone, Copy)]
pub(crate) struct PredicateDescriptor<'ctx> {
    data: &'static PredicateData,
    context: &'ctx ReflectionContext,
}

impl<'ctx> PredicateDescriptor<'ctx> {
    pub(crate) const fn new(
        data: &'static PredicateData,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { data, context }
    }

    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    pub(crate) fn kind(self) -> PredicateKind<'ctx> {
        match *self.data {
            PredicateData::TraitBound {
                ty,
                trait_,
                modifier,
            } => PredicateKind::TraitBound {
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
            PredicateData::ConstEvaluatable(expression) => {
                PredicateKind::ConstEvaluatable(expression)
            }
        }
    }
}

/// Structural source fact represented by a generic predicate.
pub(crate) enum PredicateKind<'ctx> {
    TraitBound {
        ty: TypeExpression<'ctx>,
        trait_: TraitExpression<'ctx>,
        modifier: TraitBoundModifier,
    },
    LifetimeOutlives {
        lifetime: LifetimeExpression,
        outlives: LifetimeExpression,
    },
    TypeOutlives {
        ty: TypeExpression<'ctx>,
        lifetime: LifetimeExpression,
    },
    TypeEquality {
        left: TypeExpression<'ctx>,
        right: TypeExpression<'ctx>,
    },
    ConstEvaluatable(super::data::ConstExpression),
}

/// Ordered context-bound view over predicate metadata.
#[derive(Clone, Copy)]
pub(crate) struct Predicates<'ctx> {
    data: &'static [PredicateData],
    context: &'ctx ReflectionContext,
}

impl<'ctx> Predicates<'ctx> {
    pub(crate) const fn new(
        data: &'static [PredicateData],
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { data, context }
    }

    pub(crate) fn iter(self) -> impl ExactSizeIterator<Item = PredicateDescriptor<'ctx>> + 'ctx {
        self.data
            .iter()
            .map(move |data| PredicateDescriptor::new(data, self.context))
    }

    pub(crate) const fn is_empty(self) -> bool {
        self.data.is_empty()
    }
}
