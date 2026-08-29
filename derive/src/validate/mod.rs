//! Validation of parsed reflection declarations.

mod declaration;

pub(crate) use declaration::{validate_declaration, validation_error};
