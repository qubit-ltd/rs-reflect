// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed construction and update adapter contracts emitted by derive.
//!
//! Generated constructors need safe entry points for struct, tuple, unit, and
//! enum-variant construction, plus field update. The matching runtime pieces
//! are `ConstructionDescriptor`, `ConstructField`, `ConstructionProviders`,
//! `ConstructionTarget`, `ConstructionInput`, and the validated adapter
//! contracts under `runtime::construct`. They are currently crate-private;
//! generated code must not construct or mutate validated inputs directly.

pub use crate::runtime::construct::{
    ConstructField, ConstructionAvailability, ConstructionFieldPolicy, ConstructionPlan,
    ConstructionProviders, ConstructionShape, ConstructionUnavailableReason,
    TypedConstructProvider as ConstructProvider, UpdateFieldPolicy, UpdateUnavailableReason,
};
