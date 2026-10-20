// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Fixture proving that reflected members follow a Cargo feature.

// The derives emit fixture-only helper macros whose docs are not user-facing.
#![allow(missing_docs)]

use qubit_reflect::Reflect;

/// A signature type available only when the fixture feature is enabled.
#[cfg(feature = "conditional-extra")]
#[Reflect]
pub struct FeatureOnlyType;

/// A trait whose members and helper validation follow the selected feature.
#[Reflect]
pub trait FeatureConditional {
    /// Exists on 64-bit targets.
    #[cfg(target_pointer_width = "64")]
    fn pointer_width_64(&self);

    /// Exists on targets that are not 64-bit.
    #[cfg(not(target_pointer_width = "64"))]
    fn other_pointer_width(&self);

    /// Returns the feature-gated signature type.
    #[cfg(feature = "conditional-extra")]
    fn feature_method(&self) -> FeatureOnlyType;
}
