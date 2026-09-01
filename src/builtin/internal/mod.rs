// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shared implementation details for built-in descriptors.

#[cfg(any(feature = "ecosystem-types", feature = "qubit-types"))]
mod reflected_opaque;

#[cfg(any(feature = "ecosystem-types", feature = "qubit-types"))]
pub(crate) use reflected_opaque::reflected_opaque;
