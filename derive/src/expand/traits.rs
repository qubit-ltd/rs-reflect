// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Expansion of reflected trait declarations to typed codegen v4 metadata.

mod default_invocation;
mod metadata;
mod trait_metadata;

use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::ItemTrait;
use syn::LitStr;
use syn::parse_quote;
use syn::parse2;

use super::ExpansionContext;
pub(crate) use super::expression_codegen::generic_definition;
pub(crate) use super::expression_codegen::type_expression;
use crate::ir::GenericBoundIr;
use crate::ir::PathArgumentIr;
use crate::ir::PathArgumentsIr;
use crate::ir::PathIr;
use crate::ir::TraitDeclarationIr;
use crate::ir::VisibilityIr;

pub(crate) fn expand(declaration: TraitDeclarationIr, context: &ExpansionContext) -> TokenStream {
    let facade = context.facade().clone();
    let codegen = quote!(#facade::__private::codegen_v4);
    let marker = format_ident!("__QubitReflectTraitDef_{}", declaration.name);
    let spec_provider = format_ident!("__QubitReflectTraitDef_{}_SPEC", declaration.name);
    let supertrait_applications_provider =
        format_ident!("__QubitReflectTraitDef_{}_APPLICATION_SUPERTRAITS", declaration.name);
    let supertrait_applications_for_provider = format_ident!(
        "__QubitReflectTraitDef_{}_APPLICATION_SUPERTRAITS_FOR",
        declaration.name
    );
    let all_supertrait_applications_provider = format_ident!(
        "__QubitReflectTraitDef_{}_APPLICATION_ALL_SUPERTRAITS",
        declaration.name
    );
    let default_methods_provider = format_ident!("__QubitReflectTraitDef_{}_DEFAULT_METHODS", declaration.name);
    let trait_name = declaration.name.to_string();
    let trait_name_literal = LitStr::new(&trait_name, declaration.span);
    let visibility = match &declaration.visibility {
        VisibilityIr::Public => quote!(#codegen::traits::TraitVisibility::Public),
        VisibilityIr::Restricted(path) => {
            let path = LitStr::new(&path.source, declaration.span);
            quote!(#codegen::traits::TraitVisibility::Restricted(#path))
        }
        VisibilityIr::Crate | VisibilityIr::Super | VisibilityIr::SelfValue | VisibilityIr::Inherited => {
            quote!(#codegen::traits::TraitVisibility::Private)
        }
    };
    let generic_definition = super::traits::generic_definition(&declaration.generics, declaration.span, &facade);
    let trait_metadata = metadata::build(&declaration, &trait_name_literal, &facade);
    let methods = trait_metadata.methods;
    let associated_types = trait_metadata.associated_types;
    let associated_consts = trait_metadata.associated_consts;
    let supertraits = declaration
        .supertraits
        .iter()
        .enumerate()
        .filter_map(|(index, bound)| match bound {
            GenericBoundIr::Trait { path, lifetimes, .. } => {
                let name = LitStr::new(&path.source, path.span);
                let environment =
                    crate::expand::generic_environment::GenericEnvironment::from_generics(&declaration.generics);
                let arguments = path.segments.iter().flat_map(|segment| {
                    crate::expand::expression_codegen::path_arguments(
                        &segment.arguments,
                        &environment,
                        &facade,
                        path.span,
                    )
                });
                let lifetimes = lifetimes.iter().map(|lifetime| {
                    crate::expand::expression_codegen::lifetime_expression(lifetime, path.span, &facade)
                });
                let expression = quote!(::std::boxed::Box::leak(::std::boxed::Box::new(
                    #codegen::expression::TraitExpressionData::new(
                        #name,
                        ::std::boxed::Box::leak(::std::vec![#(#arguments),*].into_boxed_slice()),
                        ::std::boxed::Box::leak(::std::vec![#(#lifetimes),*].into_boxed_slice()),
                    ),
                )));
                if let Some(binding) = declaration
                    .supertrait_bindings
                    .iter()
                    .find(|binding| binding.index == index)
                {
                    let provider = trait_spec_provider_path(&binding.provider);
                    Some(quote!(#codegen::traits::trait_expression_for(#expression, #provider())))
                } else {
                    Some(quote!(#codegen::traits::unresolved_trait_expression(#expression)))
                }
            }
            _ => None,
        });
    let has_generics = !declaration.generics.params.is_empty();
    let has_type_or_const_generics = declaration
        .generics
        .params
        .iter()
        .any(|parameter| parameter.kind != crate::ir::GenericKindIr::Lifetime);
    let supertrait_applications = declaration
        .supertraits
        .iter()
        .enumerate()
        .filter_map(|(index, bound)| {
            let GenericBoundIr::Trait { path, lifetimes, .. } = bound else {
                return None;
            };
            let references_child_lifetime = path_references_child_lifetime(path, &declaration.generics);
            let binding = declaration
                .supertrait_bindings
                .iter()
                .find(|binding| binding.index == index);
            // An explicit binding establishes trait identity even when the
            // application contains a higher-ranked lifetime. In that case
            // retain the source arguments, but do not claim concrete identity.
            // Child lifetime parameters that do not occur in the edge do not
            // affect its application.
            let provider = binding.map(|binding| &binding.provider);
            let concrete = lifetimes.is_empty()
                && !references_child_lifetime
                && !supertrait_path_contains_projection(path, &declaration.generics);
            supertrait_application(path, lifetimes, &declaration.generics, &facade, concrete, provider)
        })
        .collect::<Vec<_>>();
    let mut retained = match parse2::<ItemTrait>(declaration.retained_tokens.clone()) {
        Ok(item) => item,
        Err(error) => return error.into_compile_error(),
    };
    let application_hook = format_ident!("__qbr_trait_application_metadata");
    if let Some(conflicting_method) = retained.items.iter().find_map(|item| match item {
        syn::TraitItem::Fn(method) if method.sig.ident == application_hook => Some(method),
        _ => None,
    }) {
        return syn::Error::new_spanned(
            conflicting_method,
            "`__qbr_trait_application_metadata` is reserved by `#[Reflect]`",
        )
        .into_compile_error();
    }
    let application_hook_tokens = trait_application_hook(&declaration, &spec_provider, &application_hook, &facade);
    retained
        .items
        .push(parse2(application_hook_tokens).expect("the generated trait application hook parses"));
    let default_methods = super::traits::default_invocation::default_methods_provider(
        &declaration,
        &retained,
        &declaration.name,
        &spec_provider,
        &default_methods_provider,
        &facade,
        context,
    );
    let supertrait_binding_checks = declaration
        .supertrait_bindings
        .iter()
        .filter(|binding| {
            matches!(
                declaration.supertraits.get(binding.index),
                Some(GenericBoundIr::Trait { path, lifetimes, .. })
                    if lifetimes.is_empty() && !has_supertrait_arguments(path)
            )
        })
        .map(|binding| {
            let provider = syn::parse_str::<syn::Path>(&binding.provider.source)
                .expect("validated supertrait provider path parses");
            let check = format_ident!(
                "__QubitReflectTraitDef_{}_CHECK_SUPERTRAIT_{}",
                declaration.name,
                binding.index
            );
            let trait_name = &declaration.name;
            let mut check_generics = syn::parse2::<syn::Generics>(declaration.generics.impl_declaration.clone())
                .expect("validated trait generics parse");
            check_generics
                .params
                .insert(0, syn::parse_quote!(__QubitReflectTarget: ?Sized));
            if !declaration.generics.where_clause.is_empty() {
                check_generics.where_clause = Some(
                    syn::parse2(declaration.generics.where_clause.clone())
                        .expect("validated trait where clause parses"),
                );
            }
            let trait_arguments = &declaration.generics.arguments;
            let child_bound = syn::parse2::<syn::TypeParamBound>(quote!(#trait_name #trait_arguments))
                .expect("trait declaration arguments form a trait bound");
            check_generics
                .make_where_clause()
                .predicates
                .push(syn::parse_quote!(__QubitReflectTarget: #child_bound));
            let (check_impl_generics, _, check_where_clause) = check_generics.split_for_impl();
            quote! {
                #[allow(non_snake_case, dead_code)]
                fn #check #check_impl_generics () #check_where_clause {
                    fn require_provider<__QubitReflectBoundTarget: ?Sized + #provider>() {}
                    require_provider::<__QubitReflectTarget>();
                }
            }
        });
    let helper_parameters = declaration
        .generics
        .params
        .iter()
        .filter_map(|parameter| match parameter.kind {
            crate::ir::GenericKindIr::Lifetime => None,
            crate::ir::GenericKindIr::Type => {
                let Ok(syn::GenericParam::Type(mut generic)) = parse2(parameter.declaration.clone()) else {
                    return Some(parameter.declaration.clone());
                };
                generic.default = None;
                generic.bounds.push(parse_quote!('static));
                Some(quote!(#generic))
            }
            crate::ir::GenericKindIr::Const => {
                let Ok(syn::GenericParam::Const(mut generic)) = parse2(parameter.declaration.clone()) else {
                    return Some(parameter.declaration.clone());
                };
                generic.default = None;
                Some(quote!(#generic))
            }
        })
        .collect::<Vec<_>>();
    let helper_generics = if helper_parameters.is_empty() {
        quote!()
    } else {
        quote!(<#(#helper_parameters),*>)
    };
    let no_argument_supertrait_applications = declaration
        .supertraits
        .iter()
        .enumerate()
        .filter_map(|(index, bound)| {
            let GenericBoundIr::Trait { path, lifetimes, .. } = bound else {
                return None;
            };
            let binding = declaration
                .supertrait_bindings
                .iter()
                .find(|binding| binding.index == index);
            let can_use_source_application = (has_generics && has_supertrait_arguments(path)) || !lifetimes.is_empty();
            if can_use_source_application && binding.is_none() {
                return supertrait_application(path, lifetimes, &declaration.generics, &facade, false, None);
            }
            supertrait_application(
                path,
                lifetimes,
                &declaration.generics,
                &facade,
                !can_use_source_application,
                binding.map(|binding| &binding.provider),
            )
        })
        .collect::<Vec<_>>();
    let no_argument_supertraits = quote!(::std::boxed::Box::leak(
        ::std::vec![#(#no_argument_supertrait_applications),*].into_boxed_slice()
    ));
    let generic_supertrait_provider = if has_type_or_const_generics {
        let applications = &supertrait_applications;
        quote! {
            #[allow(non_snake_case)]
            #[doc(hidden)]
            pub fn #supertrait_applications_for_provider #helper_generics() -> &'static [&'static #codegen::traits::TraitApplication] {
                ::std::boxed::Box::leak(::std::vec![#(#applications),*].into_boxed_slice())
            }
        }
    } else {
        quote!()
    };
    let all_supertrait_applications_provider_tokens = quote! {
        #[allow(non_snake_case)]
        #[doc(hidden)]
        pub fn #all_supertrait_applications_provider() -> &'static [&'static #codegen::traits::TraitApplication] {
            static VALUE: ::std::sync::OnceLock<&'static [&'static #codegen::traits::TraitApplication]> = ::std::sync::OnceLock::new();
            VALUE.get_or_init(|| #codegen::traits::all_supertrait_applications(#supertrait_applications_provider()))
        }
    };
    let tokens = quote! {
        #retained

        #(#supertrait_binding_checks)*

        #[allow(non_camel_case_types)]
        #[doc(hidden)]
        pub struct #marker;

        #[allow(non_snake_case)]
        #[doc(hidden)]
        pub fn #spec_provider() -> &'static #codegen::traits::TraitDefinitionSpec<#marker> {
            static VALUE: ::std::sync::OnceLock<#codegen::traits::TraitDefinitionSpec<#marker>> =
                ::std::sync::OnceLock::new();
            VALUE.get_or_init(|| {
                let generics = #generic_definition;
                let supertraits = ::std::boxed::Box::leak(::std::vec![#(#supertraits),*].into_boxed_slice());
                let methods = ::std::boxed::Box::leak(::std::vec![#(#methods),*].into_boxed_slice());
                let associated_types = ::std::boxed::Box::leak(::std::vec![#(#associated_types),*].into_boxed_slice());
                let associated_consts = ::std::boxed::Box::leak(::std::vec![#(#associated_consts),*].into_boxed_slice());
                #codegen::traits::TraitDefinitionSpec::new(
                    #trait_name_literal,
                    concat!(module_path!(), "::", #trait_name_literal),
                    #visibility,
                    #codegen::traits::Completeness::Complete,
                    ::std::boxed::Box::leak(::std::boxed::Box::new(generics)),
                    supertraits,
                    methods,
                    associated_types,
                    associated_consts,
                )
            })
        }

        #default_methods

        #[allow(non_snake_case)]
        #[doc(hidden)]
        pub fn #supertrait_applications_provider() -> &'static [&'static #codegen::traits::TraitApplication] {
            #no_argument_supertraits
        }

        #generic_supertrait_provider

        #all_supertrait_applications_provider_tokens

        #codegen::traits::submit_trait_definition!(#marker, #spec_provider);
    };
    tokens
}

/// Builds the hidden trait method that exposes compiler-resolved generic
/// arguments for one local reflected trait application.
fn trait_application_hook(
    declaration: &TraitDeclarationIr,
    spec_provider: &syn::Ident,
    hook: &syn::Ident,
    facade: &TokenStream,
) -> TokenStream {
    let codegen = quote!(#facade::__private::codegen_v4);
    let environment = crate::expand::generic_environment::GenericEnvironment::from_generics(&declaration.generics);
    let mut arguments = Vec::new();
    let mut identities = Vec::new();
    let mut type_bounds = Vec::new();
    let has_application_generics = declaration
        .generics
        .params
        .iter()
        .any(|parameter| parameter.kind != crate::ir::GenericKindIr::Lifetime);
    let supertrait_generic_arguments = declaration
        .generics
        .params
        .iter()
        .filter(|parameter| parameter.kind != crate::ir::GenericKindIr::Lifetime)
        .map(|parameter| {
            let name = syn::Ident::new(&parameter.name, parameter.span);
            quote!(#name)
        })
        .collect::<Vec<_>>();
    let supertrait_provider = if has_application_generics {
        format_ident!(
            "__QubitReflectTraitDef_{}_APPLICATION_SUPERTRAITS_FOR",
            declaration.name
        )
    } else {
        format_ident!("__QubitReflectTraitDef_{}_APPLICATION_SUPERTRAITS", declaration.name)
    };
    let direct_supertraits = if has_application_generics {
        quote!(#supertrait_provider::<#(#supertrait_generic_arguments),*>() )
    } else {
        quote!(#supertrait_provider())
    };

    for (index, parameter) in declaration.generics.params.iter().enumerate() {
        let name = syn::Ident::new(&parameter.name, parameter.span);
        match parameter.kind {
            crate::ir::GenericKindIr::Lifetime => {}
            crate::ir::GenericKindIr::Type => {
                let type_parameter: syn::Type = syn::parse_quote!(#name);
                let type_ir = crate::parse::convert_type(&type_parameter);
                let expression = super::traits::type_expression(&type_ir, &environment, facade);
                let parameter_metadata = quote!(#spec_provider().generic_parameter_at(#index));
                arguments.push(
                    quote!(#codegen::expression::GenericArgumentMetadata::type_argument::<#name>(
                        #parameter_metadata,
                        #expression,
                    )),
                );
                identities.push(quote!(#codegen::traits::ArgumentIdentity::type_argument::<#name>()));
                type_bounds.push(quote!(#name: 'static));
            }
            crate::ir::GenericKindIr::Const => {
                let parameter_metadata = quote!(#spec_provider().generic_parameter_at(#index));
                let value = quote!(#codegen::expression::ConstExpression::from_const(#name));
                arguments.push(quote!(#codegen::expression::GenericArgumentMetadata::const_argument(
                    #parameter_metadata,
                    #value,
                )));
                identities.push(quote!(#codegen::traits::ArgumentIdentity::const_argument(#value)));
            }
        }
    }

    let argument_box = quote!(::std::vec![#(#arguments),*].into_boxed_slice());
    let identity_box = quote!(::std::vec![#(#identities),*].into_boxed_slice());
    let where_bounds = if type_bounds.is_empty() {
        quote!(where Self: Sized)
    } else {
        quote!(where Self: Sized, #(#type_bounds),*)
    };

    quote! {
        /// Returns typed generic arguments after Rust applies this trait's defaults.
        #[doc(hidden)]
        #[allow(non_snake_case)]
        fn #hook() -> (
            ::std::boxed::Box<[#codegen::expression::GenericArgumentMetadata]>,
            ::std::boxed::Box<[#codegen::traits::ArgumentIdentity]>,
            &'static [&'static #codegen::traits::TraitApplication],
            &'static [&'static #codegen::traits::TraitApplication],
        )
        #where_bounds
        {
            let direct_supertraits = #direct_supertraits;
            let all_supertraits = #codegen::traits::all_supertrait_applications(direct_supertraits);
            (#argument_box, #identity_box, direct_supertraits, all_supertraits)
        }
    }
}

fn has_supertrait_arguments(path: &PathIr) -> bool {
    path.segments.iter().any(
        |segment| matches!(&segment.arguments, PathArgumentsIr::AngleBracketed(arguments) if !arguments.is_empty()),
    )
}

/// Returns whether a supertrait argument contains an associated-type
/// projection whose resolved type cannot be proven `'static` from source
/// syntax alone.
fn supertrait_path_contains_projection(path: &PathIr, generics: &crate::ir::GenericsIr) -> bool {
    path.segments.iter().any(|segment| match &segment.arguments {
        PathArgumentsIr::None => false,
        PathArgumentsIr::AngleBracketed(arguments) => arguments.iter().any(|argument| match argument {
            PathArgumentIr::Type(ty) | PathArgumentIr::AssociatedType { ty, .. } => {
                type_contains_projection(ty, generics)
            }
            PathArgumentIr::Constraint { .. }
            | PathArgumentIr::Lifetime(_)
            | PathArgumentIr::Const(_)
            | PathArgumentIr::AssociatedConst { .. }
            | PathArgumentIr::Other(_) => false,
        }),
        PathArgumentsIr::Parenthesized { inputs, output } => {
            inputs.iter().any(|ty| type_contains_projection(ty, generics))
                || output.as_ref().is_some_and(|ty| type_contains_projection(ty, generics))
        }
    })
}

fn type_contains_projection(ty: &crate::ir::TypeIr, generics: &crate::ir::GenericsIr) -> bool {
    use crate::ir::TypeKindIr;

    match &ty.kind {
        TypeKindIr::Path(path) => {
            path.qualified_self.is_some()
                || path.segments.first().is_some_and(|first| {
                    (first.name == "Self"
                        || generics.params.iter().any(|parameter| {
                            parameter.kind == crate::ir::GenericKindIr::Type && parameter.name == first.name
                        }))
                        && path.segments.len() > 1
                })
                || path.segments.iter().any(|segment| match &segment.arguments {
                    PathArgumentsIr::None => false,
                    PathArgumentsIr::AngleBracketed(arguments) => arguments.iter().any(|argument| match argument {
                        PathArgumentIr::Type(ty) | PathArgumentIr::AssociatedType { ty, .. } => {
                            type_contains_projection(ty, generics)
                        }
                        PathArgumentIr::Constraint { .. }
                        | PathArgumentIr::Lifetime(_)
                        | PathArgumentIr::Const(_)
                        | PathArgumentIr::AssociatedConst { .. }
                        | PathArgumentIr::Other(_) => false,
                    }),
                    PathArgumentsIr::Parenthesized { inputs, output } => {
                        inputs.iter().any(|ty| type_contains_projection(ty, generics))
                            || output.as_ref().is_some_and(|ty| type_contains_projection(ty, generics))
                    }
                })
        }
        TypeKindIr::Reference { element, .. } | TypeKindIr::Slice(element) | TypeKindIr::Pointer { element, .. } => {
            type_contains_projection(element, generics)
        }
        TypeKindIr::Tuple(elements) => elements
            .iter()
            .any(|element| type_contains_projection(element, generics)),
        TypeKindIr::Array { element, .. } => type_contains_projection(element, generics),
        TypeKindIr::BareFunction { inputs, output, .. } => {
            inputs.iter().any(|input| type_contains_projection(input, generics))
                || output
                    .as_ref()
                    .is_some_and(|output| type_contains_projection(output, generics))
        }
        TypeKindIr::TraitObject { bounds, .. } | TypeKindIr::ImplTrait { bounds } => {
            bounds.iter().any(|bound| match bound {
                GenericBoundIr::Trait { path, .. } => supertrait_path_contains_projection(path, generics),
                GenericBoundIr::Lifetime(_) | GenericBoundIr::Other(_) => false,
            })
        }
        TypeKindIr::Never | TypeKindIr::Infer => false,
        // Opaque source syntax may expand to an associated-type projection.
        TypeKindIr::Macro | TypeKindIr::Other => true,
    }
}

fn path_references_child_lifetime(path: &PathIr, generics: &crate::ir::GenericsIr) -> bool {
    let lifetimes = generics
        .params
        .iter()
        .filter(|parameter| parameter.kind == crate::ir::GenericKindIr::Lifetime)
        .map(|parameter| parameter.name.as_str())
        .collect::<Vec<_>>();
    token_stream_references_lifetime(&path.tokens, &lifetimes)
}

fn token_stream_references_lifetime(tokens: &TokenStream, lifetimes: &[&str]) -> bool {
    let token_trees = tokens.clone().into_iter().collect::<Vec<_>>();
    for (index, token) in token_trees.iter().enumerate() {
        if let proc_macro2::TokenTree::Punct(punctuation) = token
            && punctuation.as_char() == '\''
            && token_trees.get(index + 1).is_some_and(|next| {
                if let proc_macro2::TokenTree::Ident(name) = next {
                    let name = name.to_string();
                    lifetimes.contains(&name.as_str())
                } else {
                    false
                }
            })
        {
            return true;
        }
        if let proc_macro2::TokenTree::Group(group) = token
            && token_stream_references_lifetime(&group.stream(), lifetimes)
        {
            return true;
        }
    }
    false
}

fn supertrait_application_arguments(
    path: &PathIr,
    environment: &crate::expand::generic_environment::GenericEnvironment,
    facade: &TokenStream,
    concrete: bool,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut arguments = Vec::new();
    let mut identity = Vec::new();
    let Some(segment) = path.segments.last() else {
        return (arguments, identity);
    };
    let PathArgumentsIr::AngleBracketed(source_arguments) = &segment.arguments else {
        return (arguments, identity);
    };
    for argument in source_arguments {
        match argument {
            PathArgumentIr::Type(ty) => {
                let ty_tokens = &ty.tokens;
                let expression = super::expression_codegen::type_expression(ty, environment, facade);
                if concrete {
                    arguments.push(quote!(#facade::__private::codegen_v4::expression::GenericArgumentMetadata::positional_type_argument::<#ty_tokens>(#expression)));
                    identity.push(
                        quote!(#facade::__private::codegen_v4::traits::ArgumentIdentity::type_argument::<#ty_tokens>()),
                    );
                } else {
                    arguments.push(quote!(#facade::__private::codegen_v4::expression::GenericArgumentMetadata::positional_type_expression(#expression)));
                    identity.push(quote!(#facade::__private::codegen_v4::traits::ArgumentIdentity::expression(stringify!(#ty_tokens))));
                }
            }
            PathArgumentIr::Const(value) => {
                let const_value = if concrete {
                    quote!(#facade::__private::codegen_v4::expression::ConstExpression::from_const(#value))
                } else {
                    quote!(#facade::__private::codegen_v4::expression::ConstExpression::Path(stringify!(#value)))
                };
                arguments.push(quote!(#facade::__private::codegen_v4::expression::GenericArgumentMetadata::positional_const_argument(#const_value)));
                identity.push(
                    quote!(#facade::__private::codegen_v4::traits::ArgumentIdentity::const_argument(#const_value)),
                );
            }
            PathArgumentIr::Lifetime(_) => {}
            PathArgumentIr::AssociatedType { .. }
            | PathArgumentIr::AssociatedConst { .. }
            | PathArgumentIr::Constraint { .. }
            | PathArgumentIr::Other(_) => {}
        }
    }
    (arguments, identity)
}

fn supertrait_application(
    path: &PathIr,
    lifetimes: &[String],
    generics: &crate::ir::GenericsIr,
    facade: &TokenStream,
    concrete: bool,
    binding_provider: Option<&PathIr>,
) -> Option<TokenStream> {
    let codegen = quote!(#facade::__private::codegen_v4);
    let environment = crate::expand::generic_environment::GenericEnvironment::from_generics(generics);
    let expression_arguments = path.segments.iter().flat_map(|segment| {
        crate::expand::expression_codegen::path_arguments(&segment.arguments, &environment, facade, path.span)
    });
    let mut source_path = path
        .segments
        .iter()
        .map(|segment| segment.name.as_str())
        .collect::<Vec<_>>()
        .join("::");
    if path.leading_colon {
        source_path.insert_str(0, "::");
    }
    let path_literal = LitStr::new(&source_path, path.span);
    let lifetime_metadata = lifetimes
        .iter()
        .map(|lifetime| crate::expand::expression_codegen::lifetime_expression(lifetime, path.span, facade));
    let expression = quote!(::std::boxed::Box::leak(::std::boxed::Box::new(
        #codegen::expression::TraitExpressionData::new(
            #path_literal,
            ::std::boxed::Box::leak(::std::vec![#(#expression_arguments),*].into_boxed_slice()),
            ::std::boxed::Box::leak(::std::vec![#(#lifetime_metadata),*].into_boxed_slice()),
        ),
    )));
    let (arguments, identity) = supertrait_application_arguments(path, &environment, facade, concrete);
    if let Some(provider_path) = binding_provider {
        let definition_provider = trait_spec_provider_path(provider_path);
        let direct_provider = trait_applications_provider_call(provider_path, false);
        let all_provider = trait_applications_provider_call(provider_path, true);
        let completeness = if has_supertrait_arguments(path) {
            quote!(#codegen::traits::SupertraitStatus::Incomplete)
        } else {
            quote!(#codegen::traits::supertrait_applications_complete(direct))
        };
        Some(quote!({
            let expression = #expression;
            let arguments = ::std::boxed::Box::leak(::std::vec![#(#arguments),*].into_boxed_slice());
            let identity = ::std::boxed::Box::leak(::std::vec![#(#identity),*].into_boxed_slice());
            let direct = #direct_provider;
            let all = #all_provider;
            ::std::boxed::Box::leak(::std::boxed::Box::new(
                #codegen::traits::TraitApplication::linked(
                    #definition_provider().metadata(), expression, arguments, identity,
                    direct, all,
                    #completeness,
                ),
            )) as &'static #codegen::traits::TraitApplication
        }))
    } else {
        Some(quote!({
        let expression = #expression;
        let arguments = ::std::boxed::Box::leak(::std::vec![#(#arguments),*].into_boxed_slice());
        let identity = ::std::boxed::Box::leak(::std::vec![#(#identity),*].into_boxed_slice());
        ::std::boxed::Box::leak(::std::boxed::Box::new(
            #codegen::traits::unresolved_application(expression, arguments, identity),
        )) as &'static #codegen::traits::TraitApplication
        }))
    }
}

fn trait_spec_provider_path(path: &PathIr) -> syn::Path {
    generated_trait_path(path, "SPEC")
}

fn trait_applications_provider_call(path: &PathIr, all: bool) -> TokenStream {
    let suffix = if all {
        "APPLICATION_ALL_SUPERTRAITS"
    } else {
        "APPLICATION_SUPERTRAITS"
    };
    let provider_path = generated_trait_path(path, suffix);
    quote!(#provider_path())
}

fn generated_trait_path(path: &PathIr, suffix: &str) -> syn::Path {
    let mut provider: syn::Path =
        syn::parse_str(&path.source).expect("validated trait binding path must parse as a Rust path");
    let segment = provider
        .segments
        .last_mut()
        .expect("validated trait binding path must have a final segment");
    let identifier = format_ident!("__QubitReflectTraitDef_{}_{}", segment.ident, suffix);
    segment.ident = identifier;
    segment.arguments = syn::PathArguments::None;
    provider
}
