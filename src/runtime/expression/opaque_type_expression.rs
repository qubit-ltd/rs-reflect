// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::data::OpaqueTypeData;
use crate::runtime::context::ReflectionContext;

/// Bounds retained when an opaque source type cannot be navigated.
#[derive(Clone, Copy)]
pub struct OpaqueTypeExpression<'ctx> {
    /// Bounds retained for the opaque type.
    pub(super) data: &'static OpaqueTypeData,
    /// Context used by nested predicate views.
    pub(super) context: &'ctx ReflectionContext,
}

impl<'ctx> OpaqueTypeExpression<'ctx> {
    /// Returns the predicates retained from the opaque type's bounds.
    pub fn bounds(self) -> super::predicate::Predicates<'ctx> {
        super::predicate::Predicates::new(self.data.bounds, self.context)
    }
    /// Returns the context used by nested predicate expressions.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
}
