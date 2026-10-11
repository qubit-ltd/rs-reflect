// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Parsing of procedural macro inputs into reflection IR.

mod attributes;
mod declaration;
mod parsed_pipeline;
mod type_ir;

pub(crate) use declaration::parse_and_validate_declaration;
#[cfg(test)]
pub(crate) use declaration::parse_declaration;
pub(crate) use type_ir::convert_path;
pub(crate) use type_ir::convert_type;

#[cfg(test)]
mod tests;
