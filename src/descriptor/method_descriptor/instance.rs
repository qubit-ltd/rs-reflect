// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Concrete method specializations and descriptor-aware invocation.

mod method_implementation_source;
mod method_instance_build_error;
mod method_instance_descriptor;

pub use method_implementation_source::MethodImplementationSource;
pub use method_instance_build_error::MethodInstanceBuildError;
pub use method_instance_descriptor::MethodInstanceDescriptor;
