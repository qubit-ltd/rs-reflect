// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Machine-readable classes of registry aggregation failures.

/// The machine-readable class of a registry aggregation error.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RegistryErrorKind {
    /// Two registration fragments claim the same identity.
    DuplicateFragment,
    /// Fragment facts disagree about an identity's content.
    IdentityConflict,
    /// External trait registrations use an incompatible ID.
    ExternalTraitIdConflict,
    /// Capability registrations use an incompatible ID or contract.
    CapabilityConflict,
    /// A generic trait impl definition could not resolve one unique trait
    /// declaration.
    ImplTraitResolution,
    /// The target platform cannot support distributed registration.
    UnsupportedPlatform,
}
