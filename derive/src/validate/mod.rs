// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Validation of parsed reflection declarations.

mod declaration;

#[allow(
    unused_imports,
    reason = "the staged validation API is exercised directly by unit tests and later expansion tasks"
)]
pub(crate) use declaration::validate_declaration;
#[allow(
    unused_imports,
    reason = "the staged validation API is exercised directly by unit tests and later expansion tasks"
)]
pub(crate) use declaration::validation_error;
