// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Private registry construction data structures.

#[cfg(feature = "bench-internals")]
pub(crate) mod benchmark_registry_facts;
#[cfg(feature = "bench-internals")]
mod benchmark_target;
mod built_fragment;
#[cfg(feature = "bench-internals")]
mod fact_row;
mod materialized_fragment;
mod pending_fragment;

pub(crate) use built_fragment::BuiltFragment;
pub(crate) use materialized_fragment::MaterializedFragment;
pub(crate) use pending_fragment::PendingFragment;
