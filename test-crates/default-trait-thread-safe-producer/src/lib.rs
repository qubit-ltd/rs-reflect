//! Producer-side fixture for default trait methods exposed to a thread-safe
//! consumer crate.

// The derive emits a fixture-only helper macro without user-facing docs.
#![allow(missing_docs)]

use qubit_reflect::Reflect;

/// A service whose default method is reflected across the producer boundary.
#[Reflect]
pub trait DefaultService {
    /// Renders the input using the trait's default implementation.
    fn render(&self, input: String) -> String {
        format!("rendered: {input}")
    }
}
