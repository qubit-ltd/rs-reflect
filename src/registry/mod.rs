// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Reflection registry APIs.

mod capability_member;
mod effective_type_view;
pub(crate) mod fragment;
mod impl_definition_candidates;
mod indexes;
mod internal;
mod reflect_registry;
mod registry_builder;
mod registry_snapshot_builder;
mod trait_candidates;
mod type_candidates;
mod type_definition_candidates;

pub use capability_member::CapabilityMember;
pub use effective_type_view::EffectiveTypeView;
#[cfg(feature = "bench-internals")]
pub(crate) use effective_type_view::build_benchmark_effective_type_view;
pub use fragment::CapabilityTarget;
pub use impl_definition_candidates::ImplDefinitionCandidates;
#[cfg(feature = "bench-internals")]
pub(crate) use internal::benchmark_registry_facts::BenchmarkRegistryFacts;
#[cfg(feature = "bench-internals")]
pub(crate) use internal::benchmark_registry_facts::aggregate_benchmark_registry_facts;
#[cfg(feature = "bench-internals")]
pub(crate) use internal::benchmark_registry_facts::prepare_benchmark_registry_facts;
pub use reflect_registry::ReflectRegistry;
pub(crate) use registry_builder::build_registry;
pub(crate) use registry_builder::initialize_registry;
pub use registry_snapshot_builder::RegistrySnapshotBuilder;
pub use trait_candidates::TraitCandidates;
pub use type_candidates::TypeCandidates;
pub use type_definition_candidates::TypeDefinitionCandidates;
