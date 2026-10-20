// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::context::ReflectionContext;
use crate::runtime::identity::RegistrationOrigin;

/// Registration source metadata bound to the context that resolved it.
#[derive(Clone)]
pub struct ContextRegistrationOrigin<'ctx> {
    origin: RegistrationOrigin,
    context: &'ctx ReflectionContext,
}

impl<'ctx> ContextRegistrationOrigin<'ctx> {
    pub(crate) fn new(origin: RegistrationOrigin, context: &'ctx ReflectionContext) -> Self {
        Self { origin, context }
    }

    /// Returns the module and source location recorded for this registration.
    pub fn registration(&self) -> &RegistrationOrigin {
        &self.origin
    }

    /// Returns the context that resolved this registration.
    pub fn context(&self) -> &'ctx ReflectionContext {
        self.context
    }
}
