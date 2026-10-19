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
/// Returns the generated Rust tokens.
///
/// # Errors
///
/// Returns a syntax diagnostic when the runtime facade cannot be resolved or
/// the declaration contains malformed or unsupported expansion data.
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
    dispatch_with_context(declaration, &context)
}

/// Expands a declaration after resolving its runtime path.
fn dispatch_with_context(
    declaration: DeclarationIr,
    context: &ExpansionContext,
) -> Result<TokenStream> {
    match declaration {
        DeclarationIr::Type(declaration) => match declaration.kind {
            TypeDeclarationKindIr::Struct => super::structs::expand(declaration, context),
            TypeDeclarationKindIr::Enum => super::enums::expand(declaration, context),
            TypeDeclarationKindIr::Union => Err(syn::Error::new(
                declaration.span,
                "cannot expand Reflect for union",
            )),
        },
        DeclarationIr::Trait(declaration) => Ok(super::traits::expand(declaration, context)),
        DeclarationIr::Impl(declaration) => Ok(super::impls::expand_impl(declaration, context)),
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;

    use super::dispatch_with_context;
    use crate::expand::ExpansionContext;
    use crate::ir::DeclarationIr;
    use crate::ir::ImplDeclarationIr;
    use crate::ir::MacroKind;
    use crate::ir::TraitDeclarationIr;
    use crate::ir::TypeDeclarationIr;
    use crate::ir::TypeDeclarationKindIr;
    use crate::parse::parse_and_validate_declaration;

    /// Parses and validates one type fixture through the derive pipeline.
    ///
    /// # Parameters
    ///
    /// - `item`: Type item tokens to pass through parsing and validation.
    ///
    /// # Returns
    ///
    /// Returns the validated type declaration used by dispatch tests.
    fn parse_type(item: TokenStream) -> TypeDeclarationIr {
        let input = quote!(#item);
        let parsed = parse_and_validate_declaration(MacroKind::Derive, TokenStream::new(), input)
            .expect("the test declaration should parse and validate");
        let DeclarationIr::Type(declaration) = parsed.declaration else {
            panic!("the derive parser must produce a type declaration");
        };
        declaration
    }

    /// Parses one trait declaration through the production validation pipeline.
    fn parse_trait(item: TokenStream) -> TraitDeclarationIr {
        let parsed = parse_and_validate_declaration(MacroKind::Trait, TokenStream::new(), item)
            .expect("the test trait should parse and validate");
        let DeclarationIr::Trait(declaration) = parsed.declaration else {
            panic!("the trait parser must produce a trait declaration");
        };
        declaration
    }

    /// Parses one impl declaration through the production validation pipeline.
    fn parse_impl(item: TokenStream) -> ImplDeclarationIr {
        let parsed = parse_and_validate_declaration(MacroKind::Impl, TokenStream::new(), item)
            .expect("the test impl should parse and validate");
        let DeclarationIr::Impl(declaration) = parsed.declaration else {
            panic!("the impl parser must produce an impl declaration");
        };
        declaration
    }

    /// Asserts emitted executable tokens use only the v4 protocol.
    fn assert_codegen_v4(tokens: &TokenStream) {
        let source = tokens.to_string();
        assert!(
            source.contains("codegen_v4"),
            "expected v4 token path in:\n{source}"
        );
        assert!(
            !source.contains("codegen_v3"),
            "found legacy v3 token path in:\n{source}"
        );
    }

    /// Confirms malformed type IR becomes an actionable compile error.
    ///
    /// # Parameters
    ///
    /// - `declaration`: Deliberately malformed type declaration.
    /// - `expected`: Expected diagnostic prefix.
    /// - `detail`: Expected diagnostic detail.
    fn assert_diagnostic(declaration: TypeDeclarationIr, expected: &str, detail: &str) {
        let context = ExpansionContext::for_tests();
        let error = dispatch_with_context(DeclarationIr::Type(declaration), &context)
            .expect_err("malformed IR must be diagnosed");
        assert!(
            error.to_string().contains(expected),
            "diagnostic should include the expected prefix: {error}",
        );
        assert!(
            error.to_string().contains(detail),
            "diagnostic should include the expected detail: {error}",
        );
        assert!(
            error
                .into_compile_error()
                .to_string()
                .contains("compile_error"),
            "diagnostic should convert to a compile_error token",
        );
    }

    #[test]
    fn test_malformed_struct_generics_report_compile_diagnostic() {
        let mut declaration = parse_type(quote!(
            struct Broken<T> {
                value: T,
            }
        ));
        declaration.generics.declaration = quote!(not_a_generic_list);

        assert_diagnostic(
            declaration,
            "cannot expand Reflect for struct",
            "invalid generics",
        );
    }

    #[test]
    fn test_malformed_enum_generics_report_compile_diagnostic() {
        let mut declaration = parse_type(quote!(
            enum Broken<T> {
                Value(T),
            }
        ));
        declaration.generics.declaration = quote!(not_a_generic_list);

        assert_diagnostic(
            declaration,
            "cannot expand Reflect for enum",
            "invalid generics",
        );
    }

    #[test]
    fn test_malformed_struct_where_clause_reports_compile_diagnostic() {
        let mut declaration = parse_type(quote!(
            struct Broken<T>
            where
                T: 'static,
            {
                value: T,
            }
        ));
        declaration.generics.where_clause = quote!(where ());

        assert_diagnostic(
            declaration,
            "cannot expand Reflect for struct",
            "invalid where clause",
        );
    }

    #[test]
    fn test_malformed_enum_where_clause_reports_compile_diagnostic() {
        let mut declaration = parse_type(quote!(
            enum Broken<T>
            where
                T: 'static,
            {
                Value(T),
            }
        ));
        declaration.generics.where_clause = quote!(where ());

        assert_diagnostic(
            declaration,
            "cannot expand Reflect for enum",
            "invalid where clause",
        );
    }

    #[test]
    fn test_union_dispatch_reports_compile_diagnostic() {
        let mut declaration = parse_type(quote!(
            struct Broken;
        ));
        declaration.kind = TypeDeclarationKindIr::Union;

        let context = ExpansionContext::for_tests();
        let error = dispatch_with_context(DeclarationIr::Type(declaration), &context)
            .expect_err("union must be rejected");
        assert!(
            error
                .to_string()
                .contains("cannot expand Reflect for union"),
            "union declarations should report the unsupported type",
        );
        assert!(
            error
                .into_compile_error()
                .to_string()
                .contains("compile_error"),
            "diagnostic should convert to a compile_error token",
        );
    }

    #[test]
    fn test_trait_method_tokens_use_codegen_v4() {
        let declaration = parse_trait(quote! {
            trait Service<T> {
                fn execute(&self, value: T) -> usize;
                fn defaulted(&self) -> usize { 1 }
            }
        });
        let context = ExpansionContext::for_tests();
        let tokens = dispatch_with_context(DeclarationIr::Trait(declaration), &context)
            .expect("trait tokens should be emitted");
        assert_codegen_v4(&tokens);
        assert!(
            !tokens.to_string().contains("catch_unwind"),
            "ordinary invocation tokens must not emit panic capture"
        );
    }

    #[test]
    fn test_concrete_impl_invocation_tokens_use_codegen_v4() {
        let declaration = parse_impl(quote! {
            impl Counter {
                fn add(&mut self, value: u32) -> u32 { value }
            }
        });
        let context = ExpansionContext::for_tests();
        let tokens = dispatch_with_context(DeclarationIr::Impl(declaration), &context)
            .expect("concrete impl tokens should be emitted");
        assert_codegen_v4(&tokens);
        assert!(
            tokens.to_string().contains("InvocationAdapter"),
            "concrete executable methods should emit an invocation adapter"
        );
    }

    #[test]
    fn test_generic_impl_definition_tokens_use_codegen_v4_without_application() {
        let declaration = parse_impl(quote! {
            impl<T> Service<T> for Store<T> {
                fn execute(&self, value: T) -> usize { 1 }
            }
        });
        let context = ExpansionContext::for_tests();
        let tokens = dispatch_with_context(DeclarationIr::Impl(declaration), &context)
            .expect("generic impl definition tokens should be emitted");
        assert_codegen_v4(&tokens);
        let source = tokens.to_string();
        assert!(
            source.contains("ImplDefinition"),
            "generic impl should emit a definition: {source}"
        );
        assert!(
            !source.contains("ImplApplication"),
            "generic impl definition must not fabricate a concrete application: {source}"
        );
    }

    #[test]
    fn test_method_binding_tokens_use_codegen_v4() {
        let context = ExpansionContext::for_tests();
        let tokens = crate::expand::bind_method::expand_with_context(
            quote!(demo::Counter::add as fn(&mut Counter, u32) -> u32),
            &context,
        )
        .expect("a method binding should parse");
        assert_codegen_v4(&tokens);
    }
}
