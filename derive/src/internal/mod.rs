// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shared implementation utilities without reflection-domain semantics.

mod dependency_ident;
mod error_collector;
mod source_ident;

pub(crate) use dependency_ident::dependency_ident;
pub(crate) use error_collector::ErrorCollector;
pub(crate) use source_ident::source_ident;
