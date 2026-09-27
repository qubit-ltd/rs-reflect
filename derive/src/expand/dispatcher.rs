// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Declaration dispatch after parse and validation.

use std::env;

use proc_macro2::TokenStream;
use syn::Result;

use super::ExpansionContext;
use crate::ir::DeclarationIr;
use crate::ir::TypeDeclarationKindIr;

/// Expands one validated declaration through its dedicated backend.
///
/// # Parameters
///
/// - `declaration`: Parsed and validated type, trait, or impl declaration.
///
/// # Returns
///
/// Returns generated Rust tokens, or a diagnostic if runtime resolution fails.
///
/// # Errors
///
/// Returns a runtime facade, malformed declaration, or unsupported declaration
/// diagnostic when expansion cannot proceed.
#[must_use]
pub(crate) fn dispatch(declaration: DeclarationIr) -> Result<TokenStream> {
    let attributes = match &declaration {
        DeclarationIr::Type(value) => &value.attributes,
        DeclarationIr::Trait(value) => &value.attributes,
        DeclarationIr::Impl(value) => &value.attributes,
    };
    let context = match ExpansionContext::from_attributes(attributes) {
        Ok(context) => context,
        Err(_) if env::var("CARGO_PKG_NAME").as_deref() == Ok("qubit-reflect-derive") => {
            return Ok(match declaration {
                DeclarationIr::Type(_) => TokenStream::new(),
                DeclarationIr::Trait(value) => value.retained_tokens,
                DeclarationIr::Impl(value) => value.retained_tokens,
            });
        }
        Err(error) => return Err(error),
    };
    match declaration {
        DeclarationIr::Type(declaration) => match declaration.kind {
            TypeDeclarationKindIr::Struct => super::structs::expand(declaration, &context),
            TypeDeclarationKindIr::Enum => super::enums::expand(declaration, &context),
            TypeDeclarationKindIr::Union => Err(syn::Error::new(declaration.span, "cannot expand Reflect for union")),
        },
        DeclarationIr::Trait(declaration) => Ok(super::traits::expand(declaration, &context)),
        DeclarationIr::Impl(declaration) => Ok(super::impls::expand_impl(declaration, &context)),
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;

    use super::dispatch;
    use crate::ir::DeclarationIr;
    use crate::ir::MacroKind;
    use crate::ir::TypeDeclarationIr;
    use crate::ir::TypeDeclarationKindIr;
    use crate::parse::parse_and_validate_declaration;

    fn parse_type(item: TokenStream) -> TypeDeclarationIr {
        let input = quote!(#[reflect(crate = qubit_reflect)] #item);
        let parsed = parse_and_validate_declaration(MacroKind::Derive, TokenStream::new(), input)
            .expect("the test declaration should parse and validate");
        let DeclarationIr::Type(declaration) = parsed.declaration else {
            panic!("expected type declaration");
        };
        declaration
    }

    fn assert_diagnostic(declaration: TypeDeclarationIr, expected: &str, detail: &str) {
        let error = dispatch(DeclarationIr::Type(declaration)).expect_err("malformed IR must be diagnosed");
        assert!(error.to_string().contains(expected), "{error}");
        assert!(error.to_string().contains(detail), "{error}");
        assert!(error.into_compile_error().to_string().contains("compile_error"));
    }

    #[test]
    fn malformed_struct_generics_report_compile_diagnostic() {
        let mut declaration = parse_type(quote!(
            struct Broken<T> {
                value: T,
            }
        ));
        declaration.generics.declaration = quote!(not_a_generic_list);

        assert_diagnostic(declaration, "cannot expand Reflect for struct", "invalid generics");
    }

    #[test]
    fn malformed_enum_generics_report_compile_diagnostic() {
        let mut declaration = parse_type(quote!(
            enum Broken<T> {
                Value(T),
            }
        ));
        declaration.generics.declaration = quote!(not_a_generic_list);

        assert_diagnostic(declaration, "cannot expand Reflect for enum", "invalid generics");
    }

    #[test]
    fn malformed_struct_where_clause_reports_compile_diagnostic() {
        let mut declaration = parse_type(quote!(
            struct Broken<T>
            where
                T: 'static,
            {
                value: T,
            }
        ));
        declaration.generics.where_clause = quote!(where ());

        assert_diagnostic(declaration, "cannot expand Reflect for struct", "invalid where clause");
    }

    #[test]
    fn malformed_enum_where_clause_reports_compile_diagnostic() {
        let mut declaration = parse_type(quote!(
            enum Broken<T>
            where
                T: 'static,
            {
                Value(T),
            }
        ));
        declaration.generics.where_clause = quote!(where ());

        assert_diagnostic(declaration, "cannot expand Reflect for enum", "invalid where clause");
    }

    #[test]
    fn union_dispatch_reports_compile_diagnostic() {
        let mut declaration = parse_type(quote!(
            struct Broken;
        ));
        declaration.kind = TypeDeclarationKindIr::Union;

        let error = dispatch(DeclarationIr::Type(declaration)).expect_err("union must be rejected");
        assert!(error.to_string().contains("cannot expand Reflect for union"));
        assert!(error.into_compile_error().to_string().contains("compile_error"));
    }
}
