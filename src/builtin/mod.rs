// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Built-in reflected type descriptors.

mod array;
#[cfg(feature = "ecosystem-types")]
mod ecosystem;
mod function;
mod internal;
#[path = "../registry/interner.rs"]
pub(crate) mod interner;
mod map;
mod option;
mod pointer;
#[cfg(feature = "qubit-types")]
mod qubit;
mod raw_pointer;
mod reference;
mod scalar;
mod sequence;
mod set;
mod slice;
mod trait_object;
mod tuple;
