// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Reflection registry APIs.

mod builder;
mod effective_type_view;
pub(crate) mod fragment;
mod indexes;
#[allow(
    clippy::module_inception,
    reason = "the planned file layout names the central registry type registry.rs"
)]
mod registry;

#[cfg(feature = "bench-internals")]
pub(crate) use builder::BenchmarkRegistryFacts;
#[cfg(feature = "bench-internals")]
pub(crate) use builder::aggregate_benchmark_registry_facts;
pub(crate) use builder::build_registry;
pub(crate) use builder::initialize_registry;
#[cfg(feature = "bench-internals")]
pub(crate) use builder::prepare_benchmark_registry_facts;
pub use effective_type_view::EffectiveTypeView;
pub use registry::ImplDefinitionCandidates;
pub use registry::ReflectRegistry;
pub use registry::TraitCandidates;
pub use registry::TypeCandidates;
