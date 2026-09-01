// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Lossless aggregation of independent procedural-macro diagnostics.

/// Accumulates independent diagnostics while retaining every original span.
#[derive(Default)]
pub(crate) struct ErrorCollector {
    error: Option<syn::Error>,
}

impl ErrorCollector {
    /// Adds one diagnostic to the aggregate.
    pub(crate) fn push(&mut self, error: syn::Error) {
        if let Some(combined) = &mut self.error {
            combined.combine(error);
        } else {
            self.error = Some(error);
        }
    }

    /// Returns the accumulated diagnostic, if any.
    pub(crate) fn into_error(self) -> Option<syn::Error> {
        self.error
    }
}
