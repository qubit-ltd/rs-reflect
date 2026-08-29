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
