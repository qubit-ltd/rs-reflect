// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Unit tests for parser and validator contracts.

use proc_macro2::TokenStream;
use quote::quote;

use crate::ir::DeclarationIr;
use crate::ir::FieldShapeIr;
use crate::ir::GenericDefaultIr;
use crate::ir::HelperName;
use crate::ir::HelperTarget;
use crate::ir::MacroKind;
use crate::ir::ParameterPatternKindIr;
use crate::ir::PathArgumentsIr;
use crate::ir::ReceiverKindIr;
use crate::ir::TypeKindIr;
use crate::parse::parse_and_validate_declaration;
use crate::parse::parse_declaration;
use crate::validate::validate_declaration;

/// Parses and validates a declaration, failing the test with its diagnostic.
fn parse_valid(kind: MacroKind, args: TokenStream, input: TokenStream) -> crate::ir::ValidatedDeclaration {
    let parsed = parse_declaration(kind, args, input).expect("the declaration should parse");
    validate_declaration(parsed).expect("the declaration should validate")
}

/// Renders all combined diagnostics emitted for a declaration.
fn parse_invalid(kind: MacroKind, args: TokenStream, input: TokenStream) -> String {
    let result = parse_and_validate_declaration(kind, args, input);
    result
        .expect_err("the declaration should be rejected")
        .into_compile_error()
        .to_string()
}

#[test]
fn test_parse_derive_struct_and_enum_with_all_member_helpers() {
    let structure = parse_valid(
        MacroKind::Derive,
        TokenStream::new(),
        quote! {
            #[reflect(capabilities(Clone, Default))]
            struct Packet<T> {
                #[reflect(opaque)]
                value: T,
                #[reflect(skip)]
                count: usize,
            }
        },
    );
    let DeclarationIr::Type(structure) = &structure.declaration else {
        panic!("expected a type declaration");
    };
    assert_eq!(structure.helper_count(HelperName::Rename), 0);
    assert_eq!(structure.helper_count(HelperName::Capabilities), 1);
    assert_eq!(structure.fields.len(), 1);
    assert_eq!(structure.fields[0].ty.source, "T");
    assert!(matches!(structure.fields[0].ty.kind, TypeKindIr::Path(_)));
    assert_eq!(structure.fields[0].attributes.len(), 1);

    let enumeration = parse_valid(
        MacroKind::Derive,
        TokenStream::new(),
        quote! {
            enum Event {
                Ready,
                #[reflect(skip)]
                Data(#[reflect(opaque)] Vec<u8>),
            }
        },
    );
    let DeclarationIr::Type(enumeration) = &enumeration.declaration else {
        panic!("expected a type declaration");
    };
    assert_eq!(enumeration.variants.len(), 1);
    assert_eq!(enumeration.variants[0].name, "Ready");
}

#[test]
fn test_parse_trait_and_impl_with_external_identity_and_specializations() {
    let reflected_trait = parse_valid(
        MacroKind::Trait,
        quote!(external_trait(Send, id = "core.marker.Send")),
        quote! {
            pub trait Service: Send {
                type Output;
                const LIMIT: usize = 8;

                #[reflect(
                    catch_unwind,
                )]
                fn run<T, const N: usize>(&self, value: T) -> Self::Output;

                #[reflect(no_invoke)]
                fn disabled(&self);
            }
        },
    );
    let DeclarationIr::Trait(reflected_trait) = &reflected_trait.declaration else {
        panic!("expected a trait declaration");
    };
    assert_eq!(reflected_trait.external_traits.len(), 1);
    assert_eq!(reflected_trait.methods.len(), 2);
    assert_eq!(reflected_trait.associated_types.len(), 1);
    assert_eq!(reflected_trait.associated_consts.len(), 1);
    assert!(!reflected_trait.retained_tokens.to_string().contains("reflect"));

    let reflected_impl = parse_valid(
        MacroKind::Impl,
        quote!(external_trait_id = "example.service.Service",),
        quote! {
            impl<T, const N: usize> Service for Packet<T> {
                #[reflect(skip)]
                fn run(&self) {}
            }
        },
    );
    let DeclarationIr::Impl(reflected_impl) = &reflected_impl.declaration else {
        panic!("expected an impl declaration");
    };
    assert_eq!(reflected_impl.methods.len(), 0);
    assert_eq!(reflected_impl.target_type.source, "Packet < T >");
    assert!(reflected_impl.trait_path.is_some());
    assert!(!reflected_impl.retained_tokens.to_string().contains("reflect"));
}

#[test]
fn test_parse_trait_skip_omits_method_metadata() {
    let parsed = parse_valid(
        MacroKind::Trait,
        TokenStream::new(),
        quote! {
            trait VisibilityContract {
                #[reflect(skip)]
                fn hidden(&self);
                fn visible(&self);
            }
        },
    );
    let DeclarationIr::Trait(reflected_trait) = &parsed.declaration else {
        panic!("expected a trait declaration");
    };
    assert_eq!(reflected_trait.methods.len(), 1);
    assert_eq!(reflected_trait.methods[0].name, "visible");
}

#[test]
fn test_validate_rejects_wrong_targets_with_combined_diagnostics() {
    let diagnostics = parse_invalid(
        MacroKind::Derive,
        TokenStream::new(),
        quote! {
            #[reflect(skip)]
            struct Invalid {
                #[reflect(thread_safe)]
                value: String,
            }
        },
    );
    assert!(diagnostics.contains("`skip` is not valid on a type"));
    assert!(diagnostics.contains("`thread_safe` was removed"));
}

#[test]
fn test_validate_rejects_duplicate_and_mutually_exclusive_helpers() {
    let diagnostics = parse_invalid(
        MacroKind::Impl,
        TokenStream::new(),
        quote! {
            impl Value {
                #[reflect(skip, no_invoke, catch_unwind)]
                fn value(&self) {}
            }
        },
    );
    assert!(diagnostics.contains("`skip` cannot be combined with `no_invoke`"));
    assert!(diagnostics.contains("`skip` cannot be combined with `catch_unwind`"));

    let no_invoke = parse_invalid(
        MacroKind::Impl,
        TokenStream::new(),
        quote! {
            impl Value {
                #[reflect(no_invoke, catch_unwind)]
                fn disabled(&self) {}
            }
        },
    );
    assert!(no_invoke.contains("`no_invoke` cannot be combined with `catch_unwind`"));
}

#[test]
fn test_validate_rejects_catch_unwind_on_async_method() {
    let diagnostics = parse_invalid(
        MacroKind::Impl,
        TokenStream::new(),
        quote! {
            impl Value {
                #[reflect(catch_unwind)]
                async fn run(&self) {}
            }
        },
    );
    assert!(diagnostics.contains("`catch_unwind` cannot be used on an async method"));
}

#[test]
fn test_parse_rejects_unknown_helpers() {
    let diagnostics = parse_invalid(
        MacroKind::Derive,
        TokenStream::new(),
        quote! {
            #[reflect(unknown_policy)]
            struct Invalid;
        },
    );
    assert!(diagnostics.contains("unknown reflection helper `unknown_policy`"));

    let qualified = parse_invalid(
        MacroKind::Derive,
        TokenStream::new(),
        quote! {
            #[reflect(policy::unknown)]
            struct Invalid;
        },
    );
    assert!(qualified.contains("unknown reflection helper `policy :: unknown`"));
}

#[test]
fn test_parse_rejects_bare_helpers_and_aggregates_validation_errors() {
    let diagnostics = parse_invalid(
        MacroKind::Derive,
        TokenStream::new(),
        quote! {
            #[reflect(unknown_policy)]
            #[reflect(skip)]
            struct Invalid {
                #[reflect]
                value: String,
            }
        },
    );
    assert!(diagnostics.contains("unknown reflection helper `unknown_policy`"));
    assert!(diagnostics.contains("`skip` is not valid on a type"));
    assert!(diagnostics.contains("bare `#[reflect]` is not a valid helper attribute"));
    assert_eq!(
        diagnostics.matches("`skip` is not valid on a type").count(),
        1,
        "semantic validation should run exactly once"
    );
}

#[test]
fn test_validate_rejects_union_and_removed_rename_helpers() {
    let union_diagnostic = parse_invalid(
        MacroKind::Derive,
        TokenStream::new(),
        quote!(union Unsupported { value: u64 }),
    );
    assert!(union_diagnostic.contains("Reflect cannot be derived for unions"));

    let names = parse_invalid(
        MacroKind::Derive,
        TokenStream::new(),
        quote! {
            struct DuplicateNames {
                #[reflect(rename = "")]
                first: u8,
                #[reflect(rename = "same")]
                second: u8,
                #[reflect(rename = "same")]
                third: u8,
            }
        },
    );
    assert!(names.contains("`rename` was removed"));
}

#[test]
fn test_validate_external_trait_ids_and_mapping_conflicts() {
    let impl_diagnostics = parse_invalid(
        MacroKind::Impl,
        quote!(external_trait_id = "qubit.reflect.Display"),
        quote!(impl Display for Value {}),
    );
    assert!(impl_diagnostics.contains("reserved `qubit.reflect` namespace"));

    let mapping_diagnostics = parse_invalid(
        MacroKind::Trait,
        quote!(
            external_trait(Send, id = "example.Send"),
            external_trait(Send, id = "example.Other"),
            external_trait(Sync, id = "example.Send")
        ),
        quote!(
            trait Invalid: Send + Sync {}
        ),
    );
    assert!(mapping_diagnostics.contains("external trait path `Send` is mapped more than once"));
    assert!(mapping_diagnostics.contains("external trait ID `example.Send` is mapped more than once"));

    let unused_mapping = parse_invalid(
        MacroKind::Trait,
        quote!(external_trait(NotABound, id = "example.NotABound")),
        quote! {
            trait Invalid: Send {}
        },
    );
    assert!(unused_mapping.contains("external trait mapping `NotABound` does not match a direct supertrait"));
}

#[test]
fn test_external_mappings_reject_non_supertrait_bounds() {
    let diagnostics = parse_invalid(
        MacroKind::Trait,
        quote!(
            external_trait(Send, id = "core.marker.Send"),
            external_trait(Sync, id = "core.marker.Sync"),
            external_trait(core::fmt::Debug, id = "core.fmt.Debug")
        ),
        quote! {
            trait Service {
                type Item<T>: Send
                where
                    T: Sync;

                fn run<T: core::fmt::Debug>(&self, value: T);
            }
        },
    );
    assert!(diagnostics.contains("external trait mapping `Send` does not match a direct supertrait"));
    assert!(diagnostics.contains("external trait mapping `Sync` does not match a direct supertrait"));
    assert!(diagnostics.contains("external trait mapping `core :: fmt :: Debug` does not match a direct supertrait"));
}

#[test]
fn test_removed_options_are_explicitly_rejected_and_supported_helpers_remain() {
    for (tokens, expected) in [
        (quote!(rename = "old"), "`rename` was removed"),
        (quote!(readonly), "`readonly` was removed"),
        (quote!(read_only), "`readonly` was removed"),
        (quote!(no_construct), "`no_construct` was removed"),
        (quote!(default), "`default` was removed"),
        (quote!(thread_safe), "`thread_safe` was removed"),
        (quote!(specialize(T = String)), "`specialize` was removed"),
    ] {
        let diagnostic = parse_invalid(
            MacroKind::Derive,
            TokenStream::new(),
            quote! { struct OldApi { #[reflect(#tokens)] value: String } },
        );
        assert!(diagnostic.contains(expected), "{tokens}: {diagnostic}");
    }

    let impl_diagnostic = parse_invalid(
        MacroKind::Impl,
        quote!(specialize(T = String)),
        quote!(
            impl<T> Container<T> {}
        ),
    );
    assert!(impl_diagnostic.contains("`specialize` was removed"));

    let supported = parse_valid(
        MacroKind::Derive,
        TokenStream::new(),
        quote! {
            #[reflect(capabilities(Clone), definition_provider_v2 = provider)]
            struct NewApi<T> { #[reflect(skip)] value: T }
        },
    );
    let DeclarationIr::Type(supported) = &supported.declaration else {
        panic!("expected a type declaration");
    };
    assert_eq!(supported.helper_count(HelperName::Capabilities), 1);
    assert_eq!(supported.helper_count(HelperName::DefinitionProviderV2), 1);
}

#[test]
fn test_helper_target_matrix_matches_the_shared_contract() {
    let all_targets = [
        HelperTarget::Type,
        HelperTarget::Field,
        HelperTarget::Variant,
        HelperTarget::Method,
        HelperTarget::Impl,
        HelperTarget::Trait,
        HelperTarget::AssociatedItem,
    ];
    let expectations = [
        (HelperName::Rename, &[][..]),
        (HelperName::Opaque, &[0, 1][..]),
        (HelperName::Capabilities, &[0][..]),
        (HelperName::Skip, &[1, 2, 3][..]),
        (HelperName::ReadOnly, &[][..]),
        (HelperName::NoConstruct, &[][..]),
        (HelperName::Default, &[][..]),
        (HelperName::NoInvoke, &[3][..]),
        (HelperName::CatchUnwind, &[3][..]),
        (HelperName::ThreadSafe, &[][..]),
        (HelperName::Specialize, &[][..]),
        (HelperName::ExternalTraitId, &[4][..]),
        (HelperName::ExternalTrait, &[5][..]),
    ];
    for (helper, valid_indexes) in expectations {
        for (index, target) in all_targets.into_iter().enumerate() {
            assert_eq!(
                helper.supports(target),
                valid_indexes.contains(&index),
                "unexpected target matrix entry for {} on {}",
                helper.as_str(),
                target.as_str()
            );
        }
    }
}

#[test]
fn test_validate_accepts_repeatable_external_mappings() {
    let reflected_trait = parse_valid(
        MacroKind::Trait,
        quote!(
            external_trait(Send, id = "core.marker.Send"),
            external_trait(Sync, id = "core.marker.Sync")
        ),
        quote! {
            trait Service: Send + Sync {}
        },
    );
    let DeclarationIr::Trait(reflected_trait) = &reflected_trait.declaration else {
        panic!("expected a trait declaration");
    };
    assert_eq!(reflected_trait.external_traits.len(), 2);
}

#[test]
fn test_parse_rejects_attribute_macros_on_wrong_item_kinds() {
    let trait_diagnostic = parse_invalid(
        MacroKind::Trait,
        TokenStream::new(),
        quote!(
            struct NotATrait;
        ),
    );
    assert!(trait_diagnostic.contains("`#[reflect]` can only be applied to a trait"));

    let impl_diagnostic = parse_invalid(
        MacroKind::Impl,
        TokenStream::new(),
        quote!(
            fn not_an_impl() {}
        ),
    );
    assert!(impl_diagnostic.contains("`#[reflect_impl]` can only be applied to an impl block"));
}

#[test]
fn test_parse_preserves_structured_generics_receivers_patterns_and_type_bounds() {
    let declaration = parse_valid(
        MacroKind::Derive,
        TokenStream::new(),
        quote! {
            struct Wrapper<'a: 'static, T: Clone = String, const N: usize = 4> {
                value: &'a dyn core::fmt::Display,
                marker: [T; N],
            }
        },
    );
    let DeclarationIr::Type(declaration) = &declaration.declaration else {
        panic!("expected a type declaration");
    };
    assert_eq!(declaration.generics.params[0].bounds.len(), 1);
    assert!(matches!(
        declaration.generics.params[1].default,
        Some(GenericDefaultIr::Type(_))
    ));
    assert!(matches!(
        declaration.generics.params[2].default,
        Some(GenericDefaultIr::Const(_))
    ));
    assert!(!declaration.generics.impl_declaration.to_string().contains('='));
    assert_eq!(declaration.generics.arguments.to_string(), "< 'a , T , N >");
    assert!(declaration.generics.where_clause.is_empty());
    let TypeKindIr::Reference { lifetime, element, .. } = &declaration.fields[0].ty.kind else {
        panic!("expected a reference type");
    };
    assert_eq!(lifetime.as_deref(), Some("'a"));
    assert!(matches!(element.kind, TypeKindIr::TraitObject { .. }));

    let reflected_impl = parse_valid(
        MacroKind::Impl,
        TokenStream::new(),
        quote! {
            impl Value {
                async unsafe extern "C" fn execute<'a, T: Clone>(
                    &'a mut self,
                    _: T,
                    (left, right): (u8, u8),
                ) -> ! {
                    loop {}
                }
            }
        },
    );
    let DeclarationIr::Impl(reflected_impl) = &reflected_impl.declaration else {
        panic!("expected an impl declaration");
    };
    let method = &reflected_impl.methods[0];
    assert_eq!(
        method.receiver.as_ref().map(|value| value.kind),
        Some(ReceiverKindIr::MutableReference)
    );
    assert!(method.qualifiers.is_async);
    assert!(method.qualifiers.is_unsafe);
    assert_eq!(method.qualifiers.abi.as_deref(), Some("C"));
    assert_eq!(method.parameters[0].pattern.kind, ParameterPatternKindIr::Wildcard);
    assert_eq!(method.parameters[1].pattern.kind, ParameterPatternKindIr::Destructure);
    assert!(
        matches!(method.return_type, crate::ir::ReturnTypeIr::Type(ref ty) if matches!(ty.kind, TypeKindIr::Never))
    );
}

#[test]
fn test_parse_preserves_parenthesized_path_and_bare_function_binders() {
    let declaration = parse_valid(
        MacroKind::Derive,
        TokenStream::new(),
        quote! {
            struct Functions<T> {
                callback: for<'a> unsafe extern "C" fn(&'a T) -> &'a T,
                callable: Box<dyn Fn(T) -> T>,
            }
        },
    );
    let DeclarationIr::Type(declaration) = &declaration.declaration else {
        panic!("expected a type declaration");
    };
    let TypeKindIr::BareFunction { lifetimes, .. } = &declaration.fields[0].ty.kind else {
        panic!("expected a bare function type");
    };
    assert_eq!(lifetimes, &["'a"]);

    let TypeKindIr::Path(box_path) = &declaration.fields[1].ty.kind else {
        panic!("expected Box path");
    };
    let PathArgumentsIr::AngleBracketed(arguments) = &box_path.segments[0].arguments else {
        panic!("expected Box angle-bracketed arguments");
    };
    let crate::ir::PathArgumentIr::Type(callable) = &arguments[0] else {
        panic!("expected Box type argument");
    };
    let TypeKindIr::TraitObject { bounds, .. } = &callable.kind else {
        panic!("expected trait object");
    };
    let crate::ir::GenericBoundIr::Trait { path, .. } = &bounds[0] else {
        panic!("expected Fn trait bound");
    };
    let PathArgumentsIr::Parenthesized { inputs, output } = &path.segments[0].arguments else {
        panic!("expected parenthesized Fn arguments");
    };
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].source, "T");
    assert_eq!(output.as_deref().map(|ty| ty.source.as_str()), Some("T"));
}

#[test]
fn test_parse_treats_identifier_at_subpattern_as_destructure() {
    let declaration = parse_valid(
        MacroKind::Impl,
        TokenStream::new(),
        quote! {
            impl Value {
                fn bind(whole @ (left, right): (u8, u8), simple: u8) {
                    let _ = (whole, left, right, simple);
                }
            }
        },
    );
    let DeclarationIr::Impl(declaration) = &declaration.declaration else {
        panic!("expected an impl declaration")
    };
    let method = &declaration.methods[0];

    assert_eq!(method.parameters[0].name, None);
    assert_eq!(method.parameters[0].pattern.kind, ParameterPatternKindIr::Destructure);
    assert_eq!(method.parameters[1].name.as_deref(), Some("simple"));
    assert_eq!(method.parameters[1].pattern.kind, ParameterPatternKindIr::Identifier);
}

#[test]
fn test_reflect_trait_requires_each_direct_supertrait_to_be_classified() {
    let diagnostic = parse_invalid(
        MacroKind::Trait,
        TokenStream::new(),
        quote! { trait Broken: std::fmt::Debug {} },
    );
    assert!(diagnostic.contains("direct supertrait `std :: fmt :: Debug` needs"));

    parse_valid(
        MacroKind::Trait,
        quote!(
            supertrait(Parent),
            external_trait(std::fmt::Debug, id = "std.fmt.Debug")
        ),
        quote! { trait Child: Parent + std::fmt::Debug {} },
    );
}

#[test]
fn test_explicit_runtime_crate_helper_is_rejected() {
    let derive_error = parse_invalid(
        MacroKind::Derive,
        TokenStream::new(),
        quote!(
            #[reflect(crate = qubit_reflect)]
            struct Record {}
        ),
    );
    assert!(
        derive_error.contains("unknown"),
        "unexpected derive error: {derive_error}"
    );

    let trait_error = parse_invalid(
        MacroKind::Trait,
        quote!(crate = qubit_reflect),
        quote!(
            trait Service {}
        ),
    );
    assert!(trait_error.contains("unknown"), "unexpected trait error: {trait_error}");

    let impl_error = parse_invalid(MacroKind::Impl, quote!(crate = qubit_reflect), quote!(impl Service {}));
    assert!(impl_error.contains("unknown"), "unexpected impl error: {impl_error}");
}

/// The versioned provider hook accepts only one caller-owned identifier on a
/// generic type.
#[test]
fn test_definition_provider_v2_validates_target_and_identifier() {
    parse_valid(
        MacroKind::Derive,
        TokenStream::new(),
        quote! {
            #[reflect(definition_provider_v2 = caller_provider)] struct Valid<T> { value: T }
        },
    );
    for (declaration, expected) in [
        (
            quote! { #[reflect(definition_provider_v2 = provider)] struct Plain; },
            "requires a generic type",
        ),
        (
            quote! { #[reflect(definition_provider_v2)] struct Missing<T>(T); },
            "requires one function identifier",
        ),
        (
            quote! { #[reflect(definition_provider_v2 = module::provider)] struct Path<T>(T); },
            "unexpected token",
        ),
        (
            quote! {
                #[reflect(definition_provider_v2 = first, definition_provider_v2 = second)]
                struct Duplicate<T>(T);
            },
            "duplicate",
        ),
        (
            quote! { struct Field<T> { #[reflect(definition_provider_v2 = provider)] value: T } },
            "not valid on a field",
        ),
    ] {
        assert!(
            parse_invalid(MacroKind::Derive, TokenStream::new(), declaration).contains(expected),
            "expected diagnostic: {expected}"
        );
    }
}

/// Empty fields must retain their source container before any code generation.
#[test]
fn test_empty_struct_ir_retains_source_field_shape() {
    for (input, expected) in [
        (
            quote!(
                struct Unit;
            ),
            FieldShapeIr::Unit,
        ),
        (
            quote!(
                struct Named {}
            ),
            FieldShapeIr::Named,
        ),
        (
            quote!(
                struct Tuple();
            ),
            FieldShapeIr::Unnamed,
        ),
    ] {
        let parsed = parse_valid(MacroKind::Derive, TokenStream::new(), input);
        let DeclarationIr::Type(declaration) = parsed.declaration else {
            panic!("type declaration")
        };
        assert_eq!(declaration.field_shape, expected);
        assert!(declaration.fields.is_empty());
    }
}

#[test]
fn test_skip_removes_members_from_all_reflection_ir_collections() {
    let structure = parse_valid(
        MacroKind::Derive,
        TokenStream::new(),
        quote! {
            struct Sample {
                visible: u8,
                #[reflect(skip)]
                hidden: String,
            }
        },
    );
    let DeclarationIr::Type(structure) = structure.declaration else {
        panic!("expected a type declaration");
    };
    assert_eq!(structure.fields.len(), 1);
    assert_eq!(structure.fields[0].name.as_ref().unwrap(), "visible");

    let enumeration = parse_valid(
        MacroKind::Derive,
        TokenStream::new(),
        quote! {
            enum SampleEnum {
                Visible,
                #[reflect(skip)]
                Hidden(String),
            }
        },
    );
    let DeclarationIr::Type(enumeration) = enumeration.declaration else {
        panic!("expected a type declaration");
    };
    assert_eq!(enumeration.variants.len(), 1);
    assert_eq!(enumeration.variants[0].name, "Visible");

    let implementation = parse_valid(
        MacroKind::Impl,
        TokenStream::new(),
        quote! {
            impl Sample {
                fn visible(&self) {}
                #[reflect(skip)]
                fn hidden(&self) {}
            }
        },
    );
    let DeclarationIr::Impl(implementation) = implementation.declaration else {
        panic!("expected an impl declaration");
    };
    assert_eq!(implementation.methods.len(), 1);
    assert_eq!(implementation.methods[0].name, "visible");
}
