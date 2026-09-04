// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Reflection registry APIs.

mod effective_type_view;
pub(crate) mod fragment;
mod indexes;
mod internal;
#[allow(
    clippy::module_inception,
    reason = "the planned file layout names the central registry type registry.rs"
)]
mod registry;
mod registry_builder;

pub use effective_type_view::EffectiveTypeView;
#[cfg(feature = "bench-internals")]
pub(crate) use internal::benchmark_registry_facts::BenchmarkRegistryFacts;
#[cfg(feature = "bench-internals")]
pub(crate) use internal::benchmark_registry_facts::aggregate_benchmark_registry_facts;
#[cfg(feature = "bench-internals")]
pub(crate) use internal::benchmark_registry_facts::prepare_benchmark_registry_facts;
pub use registry::ImplDefinitionCandidates;
pub use registry::ReflectRegistry;
pub use registry::TraitCandidates;
pub use registry::TypeCandidates;
pub use registry::TypeDefinitionCandidates;
pub(crate) use registry_builder::build_registry;
pub(crate) use registry_builder::initialize_registry;
