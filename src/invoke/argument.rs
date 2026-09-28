// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Positional argument values and their validation expectations.

mod argument_expectation;
mod invocation_arg;
mod invocation_binding;
mod invocation_input_mode;

pub use argument_expectation::ArgumentExpectation;
pub use invocation_arg::InvocationArg;
pub use invocation_binding::InvocationBinding;
pub use invocation_input_mode::InvocationInputMode;
