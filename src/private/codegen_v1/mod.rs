// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Versioned protocol used by reflection code generated for this release.
//!
//! This module is not a hand-written application API. Generated code targets
//! one protocol version; incompatible protocol changes must introduce a new
//! sibling module instead of silently widening this one.

/// Descriptor factories required by generated code.
pub mod descriptor;
/// Structural expression factories required by generated code.
pub mod expression;
/// Distributed registration protocol required by generated code.
pub mod registration;

#[doc(hidden)]
pub use inventory;
