//! Unit tests for parser and validator contracts.

use proc_macro2::TokenStream;
use quote::quote;

use crate::ir::{DeclarationIr, HelperName, MacroKind, TypeKindIr};
use crate::parse::parse_declaration;
use crate::validate::validate_declaration;

/// Parses and validates a declaration, failing the test with its diagnostic.
fn parse_valid(
    kind: MacroKind,
    args: TokenStream,
    input: TokenStream,
) -> crate::ir::ValidatedDeclaration {
    let parsed = parse_declaration(kind, args, input).expect("the declaration should parse");
    validate_declaration(parsed).expect("the declaration should validate")
}

/// Renders all combined diagnostics emitted for a declaration.
fn parse_invalid(kind: MacroKind, args: TokenStream, input: TokenStream) -> String {
    let result = parse_declaration(kind, args, input).and_then(validate_declaration);
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
            #[reflect(rename = "Packet", capabilities(Clone, Default))]
            struct Packet<T> {
                #[reflect(rename = "payload", opaque, read_only, no_construct, default)]
                value: T,
                #[reflect(skip, default = make_count)]
                count: usize,
            }
        },
    );
    let DeclarationIr::Type(structure) = &structure.declaration else {
        panic!("expected a type declaration");
    };
    assert_eq!(structure.helper_count(HelperName::Rename), 1);
    assert_eq!(structure.helper_count(HelperName::Capabilities), 1);
    assert_eq!(structure.fields.len(), 2);
    assert_eq!(structure.fields[0].ty.source, "T");
    assert!(matches!(structure.fields[0].ty.kind, TypeKindIr::Path(_)));
    assert_eq!(structure.fields[0].attributes.len(), 5);
    assert_eq!(structure.fields[1].attributes.len(), 2);

    let enumeration = parse_valid(
        MacroKind::Derive,
        TokenStream::new(),
        quote! {
            enum Event {
                #[reflect(rename = "ready", no_construct)]
                Ready,
                #[reflect(skip)]
                Data(#[reflect(opaque)] Vec<u8>),
            }
        },
    );
    let DeclarationIr::Type(enumeration) = &enumeration.declaration else {
        panic!("expected a type declaration");
    };
    assert_eq!(enumeration.variants.len(), 2);
    assert_eq!(enumeration.variants[1].fields[0].ty.source, "Vec < u8 >");
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
                    rename = "execute",
                    catch_unwind,
                    thread_safe,
                    specialize(T = String, N = 4)
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
    assert!(
        !reflected_trait
            .retained_tokens
            .to_string()
            .contains("reflect")
    );

    let reflected_impl = parse_valid(
        MacroKind::Impl,
        quote!(
            external_trait_id = "example.service.Service",
            specialize(T = String, N = 4)
        ),
        quote! {
            impl<T, const N: usize> Service for Packet<T> {
                #[reflect(rename = "execute", skip)]
                fn run(&self) {}
            }
        },
    );
    let DeclarationIr::Impl(reflected_impl) = &reflected_impl.declaration else {
        panic!("expected an impl declaration");
    };
    assert_eq!(reflected_impl.methods.len(), 1);
    assert_eq!(reflected_impl.target_type.source, "Packet < T >");
    assert!(reflected_impl.trait_path.is_some());
    assert!(
        !reflected_impl
            .retained_tokens
            .to_string()
            .contains("reflect")
    );
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
    assert!(diagnostics.contains("`thread_safe` is not valid on a field"));
}

#[test]
fn test_validate_rejects_duplicate_and_mutually_exclusive_helpers() {
    let diagnostics = parse_invalid(
        MacroKind::Impl,
        TokenStream::new(),
        quote! {
            impl Value {
                #[reflect(rename = "one", rename = "two", skip, no_invoke, catch_unwind)]
                fn value(&self) {}
            }
        },
    );
    assert!(diagnostics.contains("duplicate `rename`"));
    assert!(diagnostics.contains("`skip` cannot be combined with `no_invoke`"));
    assert!(diagnostics.contains("`skip` cannot be combined with `catch_unwind`"));

    let no_invoke = parse_invalid(
        MacroKind::Impl,
        TokenStream::new(),
        quote! {
            impl Value {
                #[reflect(no_invoke, catch_unwind, thread_safe)]
                fn disabled(&self) {}
            }
        },
    );
    assert!(no_invoke.contains("`no_invoke` cannot be combined with `catch_unwind`"));
    assert!(no_invoke.contains("`no_invoke` cannot be combined with `thread_safe`"));
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
}

#[test]
fn test_validate_rejects_union_and_empty_or_conflicting_query_names() {
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
    assert!(names.contains("rename cannot be empty"));
    assert!(names.contains("duplicate field query name `same`"));
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
    assert!(
        mapping_diagnostics.contains("external trait ID `example.Send` is mapped more than once")
    );
}

#[test]
fn test_validate_specialization_parameter_completeness() {
    let diagnostics = parse_invalid(
        MacroKind::Impl,
        quote!(specialize(T = String, T = Vec<u8>, Extra = u32)),
        quote!(
            impl<T, const N: usize> Container<T, N> {}
        ),
    );
    assert!(diagnostics.contains("duplicate specialization argument `T`"));
    assert!(diagnostics.contains("unknown specialization parameter `Extra`"));
    assert!(diagnostics.contains("missing specialization argument `N`"));

    let valid = parse_valid(
        MacroKind::Impl,
        quote!(specialize(T = Result<String, std::io::Error>, N = 16)),
        quote!(
            impl<T, const N: usize> Container<T, N> {}
        ),
    );
    let DeclarationIr::Impl(valid) = &valid.declaration else {
        panic!("expected an impl declaration");
    };
    assert_eq!(valid.specializations.len(), 1);
    assert_eq!(valid.specializations[0].bindings.len(), 2);
}

#[test]
fn test_validate_accepts_repeatable_external_mappings_and_specializations() {
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

    let reflected_impl = parse_valid(
        MacroKind::Impl,
        quote!(specialize(T = String), specialize(T = Vec<u8>)),
        quote! {
            impl<T> Container<T> {}
        },
    );
    let DeclarationIr::Impl(reflected_impl) = &reflected_impl.declaration else {
        panic!("expected an impl declaration");
    };
    assert_eq!(reflected_impl.specializations.len(), 2);
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
