// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Conformance and benchmark hooks that are not part of the codegen protocol.

#[doc(hidden)]
pub use crate::__private::registration::CapabilityRegistration;
#[cfg(feature = "bench-internals")]
#[doc(hidden)]
pub use crate::__private::registration::aggregate_benchmark_registry_facts;
#[doc(hidden)]
pub use crate::__private::registration::build_registry;
#[doc(hidden)]
pub use crate::__private::registration::initialize_registry;
#[cfg(feature = "bench-internals")]
#[doc(hidden)]
pub use crate::__private::registration::prepare_benchmark_registry_facts;
