// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structured invocation validation and caught-panic errors.

mod invocation_error;
mod invocation_error_kind;
mod invocation_panic;

pub use invocation_error::InvocationError;
pub use invocation_error_kind::InvocationErrorKind;
pub use invocation_panic::InvocationPanic;
