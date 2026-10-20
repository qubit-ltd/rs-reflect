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
use quote::quote;
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
fn dispatch_with_context(declaration: DeclarationIr, context: &ExpansionContext) -> Result<TokenStream> {
    let expanded = match declaration {
        DeclarationIr::Type(declaration) => match declaration.kind {
            TypeDeclarationKindIr::Struct => super::structs::expand(declaration, context),
            TypeDeclarationKindIr::Enum => super::enums::expand(declaration, context),
            TypeDeclarationKindIr::Union => Err(syn::Error::new(declaration.span, "cannot expand Reflect for union")),
        },
        DeclarationIr::Trait(declaration) => Ok(super::traits::expand(declaration, context)),
        DeclarationIr::Impl(declaration) => Ok(super::impls::expand_impl(declaration, context)),
    }?;
    let facade = context.facade();
    Ok(quote! {
        const _: () = #facade::__private::codegen_v4::protocol::assert_protocol_version(4);
        #expanded
    })
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

    #[test]
    fn test_registered_method_binding_emits_v4_provider_registration() {
        let context = ExpansionContext::for_tests();
        let tokens = super::super::bind_method::expand_registered_with_context(
            quote!(Fixture::convert::<u64> as fn(&Fixture, String) -> u64),
            &context,
        )
        .expect("concrete method binding should expand");
        let source = tokens.to_string();

        assert!(source.contains("codegen_v4"), "expected v4 path in {source}");
        assert!(
            source.contains("submit_method_catalog_provider"),
            "expected linked method catalog provider in {source}",
        );
        assert!(
            !source.contains(concat!("codegen", "_v3")),
            "must not emit the legacy protocol in {source}",
        );
    }

    #[test]
    fn test_reflection_emits_protocol_assertion_through_resolved_facade() {
        use proc_macro_crate::FoundCrate;

        use crate::ir::MacroKind;

        let context = ExpansionContext::from_found_crate(Some(FoundCrate::Name("renamed_reflect".to_owned())))
            .expect("renamed runtime dependency should resolve");
        let declarations = [
            (
                MacroKind::Derive,
                quote!(
                    struct Fixture;
                ),
            ),
            (
                MacroKind::Trait,
                quote!(
                    trait Fixture {}
                ),
            ),
            (MacroKind::Impl, quote!(impl Fixture {})),
        ];

        for (kind, item) in declarations {
            let parsed = parse_and_validate_declaration(kind, TokenStream::new(), item)
                .expect("fixture declaration should parse and validate");
            let expanded =
                dispatch_with_context(parsed.declaration, &context).expect("fixture declaration should expand");
            let source = expanded.to_string();

            assert!(
                source
                    .contains("renamed_reflect :: __private :: codegen_v4 :: protocol :: assert_protocol_version (4)"),
                "protocol assertion should use the resolved runtime facade: {source}",
            );
        }
    }

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
        assert!(source.contains("codegen_v4"), "expected v4 token path in:\n{source}");
        assert!(
            !source.contains(concat!("codegen", "_v3")),
            "found legacy v3 token path in:\n{source}"
        );
        assert!(
            !source.contains("catch_unwind"),
            "generated thunks must leave panic capture to the runtime:\n{source}"
        );
        for forbidden in [
            "MemberId",
            "FragmentIdentity",
            "RuntimeIdentity",
            "StaticFragmentIdentity",
            "FragmentPayload",
            concat!("Reflect", "Registry"),
            "CatchingInvocationAdapter",
            "catching_invocation_adapter",
        ] {
            assert!(
                !source.contains(forbidden),
                "found forbidden v4 token `{forbidden}` in:\n{source}"
            );
        }
        let compact = source
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        assert!(
            !compact.contains("inventory::submit"),
            "found raw inventory submission in:\n{source}"
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
            error.into_compile_error().to_string().contains("compile_error"),
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

        assert_diagnostic(declaration, "cannot expand Reflect for struct", "invalid generics");
    }

    #[test]
    fn test_malformed_enum_generics_report_compile_diagnostic() {
        let mut declaration = parse_type(quote!(
            enum Broken<T> {
                Value(T),
            }
        ));
        declaration.generics.declaration = quote!(not_a_generic_list);

        assert_diagnostic(declaration, "cannot expand Reflect for enum", "invalid generics");
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

        assert_diagnostic(declaration, "cannot expand Reflect for struct", "invalid where clause");
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

        assert_diagnostic(declaration, "cannot expand Reflect for enum", "invalid where clause");
    }

    #[test]
    fn test_union_dispatch_reports_compile_diagnostic() {
        let mut declaration = parse_type(quote!(
            struct Broken;
        ));
        declaration.kind = TypeDeclarationKindIr::Union;

        let context = ExpansionContext::for_tests();
        let error =
            dispatch_with_context(DeclarationIr::Type(declaration), &context).expect_err("union must be rejected");
        assert!(
            error.to_string().contains("cannot expand Reflect for union"),
            "union declarations should report the unsupported type",
        );
        assert!(
            error.into_compile_error().to_string().contains("compile_error"),
            "diagnostic should convert to a compile_error token",
        );
    }

    #[test]
    fn test_trait_method_tokens_use_codegen_v4() {
        let declaration = parse_trait(quote! {
            trait Service<T> {
                fn execute(&self, value: T) -> usize;
                fn defaulted(&self) -> usize { 1 }
                fn generic_default<U>(&self, value: U) -> U { value }
            }
        });
        let context = ExpansionContext::for_tests();
        let tokens =
            dispatch_with_context(DeclarationIr::Trait(declaration), &context).expect("trait tokens should be emitted");
        assert_codegen_v4(&tokens);
        assert!(
            tokens.to_string().contains("MethodMetadataSpec"),
            "trait declarations should retain the executable v4 method metadata"
        );
        assert!(
            tokens.to_string().contains("DEFAULT_METHODS")
                && tokens.to_string().contains("MethodImplementationSource :: Defaulted"),
            "default methods should expose a typed target-bound invocation provider"
        );
        assert!(
            tokens.to_string().contains("GenericDefinitionMetadata"),
            "trait generic definitions should remain in generated metadata"
        );
        assert!(
            tokens.to_string().contains("method_at (2usize)"),
            "generic default methods should remain in the typed default-method catalog: {tokens}"
        );
        assert!(
            tokens
                .to_string()
                .contains("InvocationUnavailableReason :: UnspecializedGeneric"),
            "generic default methods without a binding need structured specialization state: {tokens}"
        );
        assert!(
            !tokens.to_string().contains("catch_unwind"),
            "panic capture belongs to the runtime, not generated thunks"
        );
        assert!(
            tokens.to_string().contains("catching_local"),
            "synchronous default methods should register the same typed thunk for catching calls"
        );
    }

    #[test]
    fn test_reflect_structural_emitter_uses_codegen_v4_only() {
        let parsed = parse_and_validate_declaration(
            MacroKind::Derive,
            TokenStream::new(),
            quote! {
                struct StructuralFixture {
                    value: u32,
                }
            },
        )
        .expect("the Reflect derive input should parse and validate");
        let context = ExpansionContext::for_tests();
        let tokens =
            dispatch_with_context(parsed.declaration, &context).expect("the Reflect structural emitter should expand");

        assert_codegen_v4(&tokens);
        assert!(
            tokens.to_string().contains("TypeRegistration"),
            "Reflect should emit the structural type registration: {tokens}"
        );
    }

    #[test]
    fn test_pinned_default_trait_method_emits_typed_adapter() {
        let declaration = parse_trait(quote! {
            trait PinnedService {
                fn pinned(self: ::core::pin::Pin<&mut Self>) {}
            }
        });
        let context = ExpansionContext::for_tests();
        let tokens =
            dispatch_with_context(DeclarationIr::Trait(declaration), &context).expect("trait tokens should be emitted");
        assert_codegen_v4(&tokens);
        let source = tokens.to_string();
        assert!(
            source.contains("PinnedFunctionInvocationAdapter")
                && source.contains("pinned_local :: < __QubitReflectTarget >"),
            "pinned default methods should emit the typed pinned adapter: {source}"
        );
        assert!(
            !source.contains("pinned_thread_safe")
                && !source.contains("catching_local")
                && !source.contains("catching_thread_safe")
                && !source.contains("InvocationUnavailableReason :: ValueModeUnavailable"),
            "pinned defaults should omit unsupported ThreadSafe and catching slots: {source}"
        );
    }

    #[test]
    fn test_unsafe_method_emits_unchecked_adapter_thunk() {
        let declaration = parse_impl(quote! {
            impl Counter {
                unsafe fn update_unchecked(&mut self, value: u32) { let _ = value; }
            }
        });
        let context = ExpansionContext::for_tests();
        let tokens =
            dispatch_with_context(DeclarationIr::Impl(declaration), &context).expect("impl tokens should be emitted");
        assert_codegen_v4(&tokens);
        let source = tokens.to_string();
        assert!(
            source.contains("unsafe { < Counter > :: update_unchecked"),
            "unsafe methods should emit a thunk usable only through invoke_unchecked: {source}"
        );
        assert!(
            !source.contains("catching_local") && !source.contains("catching_thread_safe"),
            "unsafe methods must not expose a safe catching invocation slot: {source}"
        );
    }

    #[test]
    fn test_unavailable_invocation_reasons_are_attached_to_method() {
        let declaration = parse_impl(quote! {
            impl Counter {
                extern "C" fn abi_only(&self) {}
                fn opaque(&self) -> impl ::core::fmt::Debug { 1u32 }
            }
        });
        let context = ExpansionContext::for_tests();
        let tokens =
            dispatch_with_context(DeclarationIr::Impl(declaration), &context).expect("impl tokens should be emitted");
        assert_codegen_v4(&tokens);
        let source = tokens.to_string();
        assert!(
            source.contains("with_unavailable")
                && source.contains("InvocationUnavailableReason :: UnsupportedAbi")
                && source.contains("InvocationUnavailableReason :: OpaqueReturn"),
            "non-callable methods should carry structured runtime reasons: {source}"
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
        assert!(
            tokens.to_string().contains("ImplApplicationSpec"),
            "concrete impl methods should be attached to a typed application"
        );
        assert!(
            tokens.to_string().contains("catching_local"),
            "synchronous impl methods should expose the typed thunk to invoke_catching"
        );
    }

    #[test]
    fn test_concrete_pin_receiver_emits_matching_adapter_and_slot() {
        let context = ExpansionContext::for_tests();
        let declaration = parse_impl(quote! {
            impl Worker {
                fn first(&self) {}
                fn second(&self) {}
                fn third(&self) {}
                fn fourth(&self) {}
                fn fifth(&self) {}
                fn sixth(&self) {}
                fn pinned_value(self: ::std::pin::Pin<&mut Worker>) -> u32 {
                    1
                }
            }
        });
        let receiver = declaration.methods[6]
            .receiver
            .as_ref()
            .expect("the explicit self receiver should be retained by parsing");
        assert_eq!(
            crate::expand::invocation::analysis::typed_pinned_receiver_mutable(receiver),
            Some(true),
            "Pin<&mut ConcreteSelf> should be classified as pinned: {receiver:#?}"
        );

        let tokens =
            dispatch_with_context(DeclarationIr::Impl(declaration), &context).expect("the concrete impl should expand");
        let source = tokens.to_string();
        assert!(
            source.contains("__QUBIT_REFLECT_INVOCATION_ADAPTER_6"),
            "the pinned adapter definition and slot should use index 6: {source}"
        );
        assert!(
            source.contains("PinnedFunctionInvocationAdapter") && source.contains("pinned_local"),
            "a concrete pinned receiver should emit the typed pinned adapter slot: {source}"
        );
        assert!(
            source.contains("ReceiverDescriptor :: for_type :: < Worker >")
                && !source.contains("for_type :: < :: std :: pin"),
            "receiver dispatch should bind to the reflected target, not Pin<&mut Worker>: {source}"
        );
    }

    #[test]
    fn test_async_impl_does_not_emit_catching_adapter_slot() {
        let declaration = parse_impl(quote! {
            impl Counter {
                async fn execute(&self) -> u32 { 1 }
            }
        });
        let context = ExpansionContext::for_tests();
        let tokens = dispatch_with_context(DeclarationIr::Impl(declaration), &context)
            .expect("async impl tokens should be emitted");
        assert_codegen_v4(&tokens);
        let source = tokens.to_string();
        assert!(
            source.contains(" local ("),
            "async calls retain ordinary invocation: {source}"
        );
        assert!(
            !source.contains("catching_local") && !source.contains("catching_thread_safe"),
            "async methods must reject catching mode without a catcher slot: {source}"
        );
    }

    #[test]
    fn test_trait_impl_tokens_use_typed_trait_application() {
        let parsed = parse_and_validate_declaration(
            MacroKind::Impl,
            quote!(reflected_trait = crate::Service),
            quote! {
                impl Service<u32> for Counter {
                    fn execute(&self, value: u32) -> usize { value as usize }
                }
            },
        )
        .expect("the reflected trait impl input should parse and validate");
        let DeclarationIr::Impl(declaration) = parsed.declaration else {
            panic!("the ReflectImpl parser must produce an impl declaration");
        };
        let context = ExpansionContext::for_tests();
        let tokens = dispatch_with_context(DeclarationIr::Impl(declaration), &context)
            .expect("concrete trait impl tokens should be emitted");
        assert_codegen_v4(&tokens);
        let source = tokens.to_string();
        assert!(
            source.contains("TraitApplicationSpec"),
            "trait impls should emit a typed trait application: {source}"
        );
        assert!(
            source.contains("__qbr_trait_application_metadata") && source.contains("as Service < u32 >"),
            "local reflected trait applications should query resolved generic metadata through the source trait path: {source}"
        );
        assert!(
            source.contains("effective_methods") && source.contains("MethodImplementationSource :: Overridden"),
            "trait applications should retain overridden effective method metadata: {source}"
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

    #[test]
    fn test_registered_method_binding_emits_provider_macro() {
        let context = ExpansionContext::for_tests();
        let tokens = crate::expand::bind_method::expand_registered_with_context(
            quote!(demo::Counter::convert::<u64> as fn(&Counter, String) -> u64),
            &context,
        )
        .expect("a registered generic method binding should parse");
        assert_codegen_v4(&tokens);
        let source = tokens.to_string();
        assert!(
            source.contains("submit_method_catalog_provider !")
                && source.contains("OnceLock")
                && source.contains("MethodSpecializationSpec"),
            "registered bindings should lazily provide typed catalog entries: {source}"
        );
        assert!(
            !source.contains("inventory :: submit") && !source.contains(concat!("codegen", "_v3")),
            "registered binding tokens must delegate inventory and use only v4: {source}"
        );
    }

    #[test]
    fn test_generic_method_binding_preserves_typed_specialization_arguments() {
        let context = ExpansionContext::for_tests();
        let tokens = crate::expand::bind_method::expand_with_context(
            quote!(demo::Counter::map::<u32, 'static, 3usize> as fn(&Counter, u32) -> u32),
            &context,
        )
        .expect("a typed generic method binding should parse");
        assert_codegen_v4(&tokens);
        let source = tokens.to_string();
        assert!(source.contains("MethodSpecializationSpec"), "{source}");
        assert!(source.contains("type_argument :: < u32 >"), "{source}");
        assert!(source.contains("lifetime (\"'static\")"), "{source}");
        assert!(source.contains("usize (3usize)"), "{source}");
        assert!(source.contains("with_specializations"), "{source}");
    }

    #[test]
    fn test_thread_safe_method_binding_uses_thread_safe_adapter_slot() {
        let context = ExpansionContext::for_tests();
        let tokens = crate::expand::bind_method::expand_with_context(
            quote!(demo::Counter::map::<u32> as thread_safe fn(&Counter, u32) -> u32),
            &context,
        )
        .expect("a ThreadSafe generic method binding should parse");
        assert_codegen_v4(&tokens);
        let source = tokens.to_string();
        assert!(source.contains("value :: ThreadSafe"), "{source}");
        assert!(
            source.contains("thread_safe (& __QUBIT_BIND_METHOD_ADAPTER)"),
            "{source}"
        );
    }

    #[test]
    fn test_method_binding_preserves_untyped_integer_literal_for_typed_normalization() {
        let context = ExpansionContext::for_tests();
        let tokens = crate::expand::bind_method::expand_with_context(
            quote!(demo::Counter::map::<3, -4> as fn(&Counter, u32) -> u32),
            &context,
        )
        .expect("untyped integer specializations should retain their literal value");
        assert_codegen_v4(&tokens);
        let source = tokens.to_string();
        assert!(
            source.contains("const_integer_literal (false , 3u128)")
                && source.contains("const_integer_literal (true , 4u128)"),
            "{source}"
        );
    }
}
