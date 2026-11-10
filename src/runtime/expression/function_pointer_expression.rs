// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::data::FunctionPointerData;
use super::data::LifetimeExpression;
use super::type_expression::TypeExpression;
use crate::runtime::context::ReflectionContext;

/// Facts retained for a function-pointer expression.
#[derive(Clone, Copy)]
pub struct FunctionPointerExpression<'ctx> {
    /// Signature facts retained by the code generator.
    pub(super) data: &'static FunctionPointerData,
    /// Context used to bind argument and result expressions.
    pub(super) context: &'ctx ReflectionContext,
}

impl<'ctx> FunctionPointerExpression<'ctx> {
    /// Returns the context used by argument and result expressions.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Returns the function pointer's explicit higher-ranked lifetimes.
    pub const fn lifetimes(self) -> &'static [LifetimeExpression] {
        self.data.lifetimes
    }

    /// Returns parameter types in source order.
    pub fn arguments(self) -> Box<[TypeExpression<'ctx>]> {
        self.data
            .arguments
            .iter()
            .map(|argument| TypeExpression::new(argument, self.context))
            .collect()
    }
    /// Returns the explicit return type, or `None` when the source omits it.
    pub fn result(self) -> Option<TypeExpression<'ctx>> {
        self.data.result.map(|result| TypeExpression::new(result, self.context))
    }
    /// Returns whether the function pointer is declared `unsafe`.
    pub const fn is_unsafe(self) -> bool {
        self.data.unsafe_
    }
    /// Returns the ABI string, or `None` for Rust's default ABI.
    pub const fn abi(self) -> Option<&'static str> {
        self.data.abi
    }
    /// Returns whether the function pointer accepts variadic arguments.
    pub const fn is_variadic(self) -> bool {
        self.data.variadic
    }
}
