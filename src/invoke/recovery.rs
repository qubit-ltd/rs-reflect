// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Recovery payloads for invocation failures before user code runs.

mod invocation_failure;
mod invocation_recovery;

pub use invocation_failure::InvocationFailure;
pub use invocation_recovery::InvocationRecovery;
