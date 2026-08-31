//! Parsing of procedural macro inputs into reflection IR.

mod attributes;
mod declaration;
mod type_ir;

#[allow(
    unused_imports,
    reason = "the staged parse API is exercised directly by unit tests and later expansion tasks"
)]
pub(crate) use declaration::parse_and_validate_declaration;
#[allow(
    unused_imports,
    reason = "the staged parse API is exercised directly by unit tests and later expansion tasks"
)]
pub(crate) use declaration::parse_declaration;
pub(crate) use type_ir::convert_type;

#[cfg(test)]
mod tests;
