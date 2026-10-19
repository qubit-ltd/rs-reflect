// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed capability registration contracts emitted by derive and helper macros.
//!
//! The runtime counterparts are `CapabilitySpec`, `CapabilityRegistration`,
//! `CapabilityTargetId`, and the built-in capability constructors in
//! `runtime::capability`. Registration currently validates the capability
//! contract internally and binds an adapter to a target; those crate-private
//! constructors need a safe typed façade. A public API must not permit callers
//! to pair an arbitrary string ID with an unrelated adapter type.
