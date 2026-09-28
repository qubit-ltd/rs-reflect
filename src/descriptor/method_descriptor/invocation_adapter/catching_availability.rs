// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Availability of requested panic-catching adapters.

/// Availability of an explicitly requested panic-catching invocation entry
/// point.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::CatchingAvailability;
/// let availability = CatchingAvailability::Available;
/// assert!(matches!(availability, CatchingAvailability::Available));
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CatchingAvailability {
    /// No catching adapter was requested for this method.
    NotRequested,
    /// The explicitly requested catching adapter is callable.
    Available,
    /// Catching was requested but the binary uses abort-on-panic semantics.
    UnavailablePanicAbort,
}
