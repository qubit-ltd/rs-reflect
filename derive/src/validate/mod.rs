// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Validation of parsed reflection declarations.

mod declaration;

#[cfg(test)]
pub(crate) use declaration::validate_declaration;
pub(crate) use declaration::validation_error;
