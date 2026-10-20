// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

pub(crate) mod access;
pub(crate) mod capability;
pub(crate) mod construct;
pub(crate) mod context;
pub(crate) mod descriptor;
pub(crate) mod error;
pub(crate) mod expression;
#[cfg(any(feature = "ecosystem-types", feature = "qubit-types"))]
mod external_builtins;
pub(crate) mod identity;
pub(crate) mod invoke;
pub(crate) mod lookup;
pub(crate) mod value;

#[cfg(test)]
mod tests;
