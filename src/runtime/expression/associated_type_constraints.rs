// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::associated_type_constraint_descriptor::AssociatedTypeConstraintDescriptor;
use super::associated_type_constraint_kind::AssociatedTypeConstraintKind;
use super::data::GenericExpressionArgument;
use super::type_expression::TypeExpression;
use crate::runtime::context::ReflectionContext;

/// Source-level associated-type constraints attached to a trait expression.
#[derive(Clone, Copy)]
pub struct AssociatedTypeConstraints<'ctx> {
    /// Trait generic arguments containing associated type equality or bounds.
    pub(super) arguments: &'static [GenericExpressionArgument],
    /// Context used by the yielded constraint descriptors.
    pub(super) context: &'ctx ReflectionContext,
}

impl<'ctx> AssociatedTypeConstraints<'ctx> {
    /// Iterates through associated-type bounds in source order.
    pub fn iter(self) -> impl Iterator<Item = AssociatedTypeConstraintDescriptor<'ctx>> + 'ctx {
        self.arguments.iter().filter_map(move |argument| match argument {
            GenericExpressionArgument::AssociatedType { name, value } => Some(AssociatedTypeConstraintDescriptor {
                name,
                kind: AssociatedTypeConstraintKind::Equality(TypeExpression::new(value, self.context)),
            }),
            GenericExpressionArgument::AssociatedTypeBound { name, bounds } => {
                Some(AssociatedTypeConstraintDescriptor {
                    name,
                    kind: AssociatedTypeConstraintKind::Bounds(super::predicate::Predicates::new(bounds, self.context)),
                })
            }
            _ => None,
        })
    }

    /// Returns the number of associated-type bounds.
    pub fn len(self) -> usize {
        self.arguments
            .iter()
            .filter(|argument| {
                matches!(
                    argument,
                    GenericExpressionArgument::AssociatedType { .. }
                        | GenericExpressionArgument::AssociatedTypeBound { .. }
                )
            })
            .count()
    }

    /// Returns whether this trait expression has no associated-type bounds.
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
}
