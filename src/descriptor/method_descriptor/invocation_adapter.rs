// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Generated method invocation adapters and mode availability.

#[path = "invocation_adapter/invocation_adapter.rs"]
mod adapter;
mod catching_availability;
mod invocation_unavailable_reason;

pub use adapter::InvocationAdapter;
pub use catching_availability::CatchingAvailability;
pub use invocation_unavailable_reason::InvocationUnavailableReason;
