// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Producer-side fixture for default trait methods exposed to a thread-safe
//! consumer crate.

// The derive emits a fixture-only helper macro without user-facing docs.
#![allow(missing_docs)]

use qubit_reflect::Reflect;

/// A service whose default method is reflected across the producer boundary.
#[Reflect]
pub trait DefaultService {
    /// Renders the input using the trait's default implementation.
    ///
    /// # Parameters
    ///
    /// - `input`: The text to render.
    ///
    /// # Returns
    ///
    /// The rendered text.
    fn render(&self, input: String) -> String {
        format!("rendered: {input}")
    }
}
