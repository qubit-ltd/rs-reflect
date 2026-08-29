//! Parsing of procedural macro inputs into reflection IR.

mod attributes;
mod declaration;
mod type_ir;

pub(crate) use declaration::parse_declaration;

#[cfg(test)]
mod tests;
