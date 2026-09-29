// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration tests for reflected invocation APIs.

#[cfg(feature = "derive")]
mod adapter_tests;
#[cfg(feature = "derive")]
mod dispatch_tests;
mod dispatch_types_tests;
#[cfg(feature = "derive")]
mod error_tests;
#[cfg(feature = "derive")]
mod pinned_tests;
#[cfg(feature = "derive")]
mod recovery_tests;
#[cfg(feature = "derive")]
mod runtime_tests;
