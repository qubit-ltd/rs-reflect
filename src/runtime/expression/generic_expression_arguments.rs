// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::data::GenericExpressionArgument;
use super::generic_expression_argument_view::GenericExpressionArgumentView;
use crate::runtime::context::ReflectionContext;

/// An ordered context-bound view over type-expression arguments.
#[derive(Clone, Copy)]
pub struct GenericExpressionArguments<'ctx> {
    /// Ordered source arguments associated with the type expression.
    arguments: &'static [GenericExpressionArgument],
    /// Context used to bind each yielded argument view.
    context: &'ctx ReflectionContext,
}

impl<'ctx> GenericExpressionArguments<'ctx> {
    /// Creates an ordered argument view for one context.
    ///
    /// # Parameters
    ///
    /// - `arguments`: Static argument slice in source order.
    /// - `context`: Reflection context inherited by argument views.
    pub(super) const fn new(arguments: &'static [GenericExpressionArgument], context: &'ctx ReflectionContext) -> Self {
        Self { arguments, context }
    }

    /// Returns the context used by nested argument expressions.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Returns the number of source arguments.
    pub fn len(self) -> usize {
        self.arguments.len()
    }

    /// Returns whether the source argument list is empty.
    pub fn is_empty(self) -> bool {
        self.arguments.is_empty()
    }

    /// Returns one argument by its zero-based source position.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based index in the original generic argument list.
    ///
    /// # Returns
    ///
    /// Returns the argument view when the index is within the source list.
    pub fn get(self, index: usize) -> Option<GenericExpressionArgumentView<'ctx>> {
        self.arguments
            .get(index)
            .copied()
            .map(|argument| GenericExpressionArgumentView::new(argument, self.context))
    }

    /// Iterates over arguments in source order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = GenericExpressionArgumentView<'ctx>> + 'ctx {
        self.arguments
            .iter()
            .copied()
            .map(move |argument| GenericExpressionArgumentView::new(argument, self.context))
    }
}
