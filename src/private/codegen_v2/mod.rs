// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Version-two protocol used by reflection code generated for this release.
//!
//! This module is not a hand-written application API. Generated code targets
//! one protocol version; incompatible protocol changes must introduce a new
//! sibling module instead of silently widening this one.

/// Field-access types required by generated code.
pub mod access;
/// Capability types and factories required by generated code.
pub mod capability;
/// Construction types required by generated code.
pub mod construct;
/// Descriptor types and factories required by generated code.
pub mod descriptor;
/// Error types required by generated code.
pub mod error;
/// Structural expression factories required by generated code.
pub mod expression;
/// Identity types required by generated code.
pub mod identity;
/// Invocation types and factories required by generated code.
pub mod invoke;
/// Distributed registration protocol required by generated code.
pub mod registration;
/// Dynamic-value types required by generated code.
pub mod value;

#[doc(hidden)]
pub use inventory;

#[doc(hidden)]
pub use crate::Reflect;
#[doc(hidden)]
pub use crate::TypeDescriptor;
