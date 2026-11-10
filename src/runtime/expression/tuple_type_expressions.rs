// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::TypeExpression;
use super::data::ExpressionData;
use crate::runtime::context::ReflectionContext;

/// A borrowed, context-bound view over tuple type-expression elements.
#[derive(Clone, Copy)]
pub struct TupleTypeExpressions<'ctx> {
    /// Static tuple elements retained by generated metadata.
    pub(super) elements: &'static [&'static ExpressionData],
    /// Context used to bind each yielded expression.
    pub(super) context: &'ctx ReflectionContext,
}

impl<'ctx> TupleTypeExpressions<'ctx> {
    /// Returns the number of tuple elements.
    pub const fn len(self) -> usize {
        self.elements.len()
    }

    /// Returns whether the tuple has no elements.
    pub const fn is_empty(self) -> bool {
        self.elements.is_empty()
    }

    /// Returns the element at `index`, if it exists.
    ///
    /// # Parameters
    ///
    /// - `self`: Borrowed tuple element collection to inspect.
    /// - `index`: Zero-based source position of the requested element.
    ///
    /// # Returns
    ///
    /// Returns the context-bound element when `index` is in range, or `None`
    /// otherwise.
    pub fn get(self, index: usize) -> Option<TypeExpression<'ctx>> {
        self.elements
            .get(index)
            .map(|element| TypeExpression::new(element, self.context))
    }

    /// Iterates over tuple elements in source order.
    ///
    /// # Parameters
    ///
    /// - `self`: Borrowed tuple element collection to iterate.
    pub fn iter(self) -> impl ExactSizeIterator<Item = TypeExpression<'ctx>> + 'ctx {
        self.elements
            .iter()
            .map(move |element| TypeExpression::new(element, self.context))
    }
}
