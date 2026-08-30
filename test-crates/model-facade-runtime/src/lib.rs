//! Minimal downstream runtime facade used to verify macro delegation.

pub use qubit_reflect::*;

/// Re-exports the generated-code contract without exposing a model dependency.
#[doc(hidden)]
pub mod __private {
    pub use qubit_reflect::__private::*;
}
