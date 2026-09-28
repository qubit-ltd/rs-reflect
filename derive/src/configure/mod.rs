// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Compiler-assisted conditional configuration for reflected traits and impls.

mod carrier;
mod conditional_attributes;
mod pipeline;

#[cfg(test)]
mod tests;

pub(crate) use pipeline::configure_attribute;
pub(crate) use pipeline::process_configured;
