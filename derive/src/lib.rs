//! Procedural macros for `qubit-reflect`.

#![forbid(unsafe_code)]

use proc_macro::TokenStream;

mod ir;
mod parse;
mod validate;
mod expand;

use ir::MacroKind;
use parse::parse_and_validate_declaration;

/// Parses and validates a `Reflect` derive declaration.
#[proc_macro_derive(Reflect, attributes(reflect))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    process_macro(MacroKind::Derive, TokenStream::new(), input)
}

/// Parses and validates a reflected trait declaration.
#[proc_macro_attribute]
pub fn reflect(attribute: TokenStream, item: TokenStream) -> TokenStream {
    process_macro(MacroKind::Trait, attribute, item)
}

/// Parses and validates a reflected impl declaration.
#[proc_macro_attribute]
pub fn reflect_impl(attribute: TokenStream, item: TokenStream) -> TokenStream {
    process_macro(MacroKind::Impl, attribute, item)
}

/// Runs the shared parse/validate pipeline and returns the declaration retained for expansion.
fn process_macro(kind: MacroKind, args: TokenStream, input: TokenStream) -> TokenStream {
    let result = parse_and_validate_declaration(kind, args.into(), input.into());
    match result {
        Ok(validated) => expand::expand(validated.declaration).into(),
        Err(error) => error.into_compile_error().into(),
    }
}
