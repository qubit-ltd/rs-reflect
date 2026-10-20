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
mod default_trait_thread_safe_isolation_tests;
#[cfg(feature = "derive")]
mod default_trait_thread_safe_parameter_modes_tests;
#[cfg(feature = "derive")]
mod dispatch_tests;
#[cfg(feature = "derive")]
mod dispatch_types_tests;
#[cfg(feature = "derive")]
mod error_tests;
#[cfg(feature = "derive")]
mod fixtures;
#[cfg(feature = "derive")]
mod generic_impl_thread_safe_binding_tests;
#[cfg(feature = "derive")]
mod pinned_tests;
#[cfg(feature = "derive")]
mod recovery_tests;
#[cfg(feature = "derive")]
mod runtime_tests;
#[cfg(feature = "derive")]
mod thread_safe_async_tests;
#[cfg(feature = "derive")]
mod trait_application_binding_tests;
