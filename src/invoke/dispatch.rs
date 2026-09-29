// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Dispatch availability diagnostics that preserve the caller's complete input.

mod invocation_dispatch_mode;
mod invocation_dispatch_reason;
mod invocation_dispatch_result;
mod invocation_unavailable;

pub use invocation_dispatch_mode::InvocationDispatchMode;
pub use invocation_dispatch_reason::InvocationDispatchReason;
pub use invocation_dispatch_result::InvocationDispatchResult;
pub use invocation_unavailable::InvocationUnavailable;
