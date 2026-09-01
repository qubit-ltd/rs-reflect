// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Minimal downstream runtime facade used to verify macro delegation.

pub use qubit_reflect::Reflect;
pub use qubit_reflect::TypeDescriptor;
pub use qubit_reflect::access;
pub use qubit_reflect::capability;
pub use qubit_reflect::construct;
pub use qubit_reflect::descriptor;
pub use qubit_reflect::error;
pub use qubit_reflect::expression;
pub use qubit_reflect::identity;
pub use qubit_reflect::registry;
pub use qubit_reflect::value;

/// Re-exports the generated-code contract without exposing a model dependency.
#[doc(hidden)]
pub mod __private {
    pub use qubit_reflect::__private::codegen_v1;
}
