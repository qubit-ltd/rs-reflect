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
use syn::parse2;

use super::ExpansionContext;
pub(crate) use super::expression_codegen::generic_definition;
pub(crate) use super::expression_codegen::type_expression;
use crate::ir::GenericBoundIr;
use crate::ir::TraitDeclarationIr;
use crate::ir::VisibilityIr;

pub(crate) fn expand(declaration: TraitDeclarationIr, context: &ExpansionContext) -> TokenStream {
    let facade = context.facade().clone();
    let codegen = quote!(#facade::__private::codegen_v4);
    let marker = format_ident!("__QubitReflectTraitDef_{}", declaration.name);
    let spec_provider = format_ident!("__QubitReflectTraitDef_{}_SPEC", declaration.name);
    let default_methods_provider = format_ident!(
        "__QubitReflectTraitDef_{}_DEFAULT_METHODS",
        declaration.name
    );
    let trait_name = declaration.name.to_string();
    let trait_name_literal = LitStr::new(&trait_name, declaration.span);
    let visibility = match &declaration.visibility {
        VisibilityIr::Public => quote!(#codegen::traits::TraitVisibility::Public),
        VisibilityIr::Restricted(path) => {
            let path = LitStr::new(&path.source, declaration.span);
            quote!(#codegen::traits::TraitVisibility::Restricted(#path))
        }
        VisibilityIr::Crate
        | VisibilityIr::Super
        | VisibilityIr::SelfValue
        | VisibilityIr::Inherited => {
            quote!(#codegen::traits::TraitVisibility::Private)
        }
    };
    let generic_definition =
        super::traits::generic_definition(&declaration.generics, declaration.span, &facade);
    let trait_metadata = metadata::build(&declaration, &trait_name_literal, &facade);
    let methods = trait_metadata.methods;
    let associated_types = trait_metadata.associated_types;
    let associated_consts = trait_metadata.associated_consts;
    let supertraits = declaration.supertraits.iter().filter_map(|bound| match bound {
        GenericBoundIr::Trait { path, lifetimes, .. } => {
            let name = LitStr::new(&path.source, path.span);
            let environment = crate::expand::generic_environment::GenericEnvironment::from_generics(&declaration.generics);
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
            Some(quote!(#codegen::traits::unresolved_trait_expression(
                ::std::boxed::Box::leak(::std::boxed::Box::new(
                    #codegen::expression::TraitExpressionData::new(
                        #name,
                        ::std::boxed::Box::leak(::std::vec![#(#arguments),*].into_boxed_slice()),
                        ::std::boxed::Box::leak(::std::vec![#(#lifetimes),*].into_boxed_slice()),
                    ),
                )),
            )))
        }
        _ => None,
    });
    let retained = match parse2::<ItemTrait>(declaration.retained_tokens.clone()) {
        Ok(item) => item,
        Err(error) => return error.into_compile_error(),
    };
    let default_methods = super::traits::default_invocation::default_methods_provider(
        &declaration,
        &retained,
        &declaration.name,
        &spec_provider,
        &default_methods_provider,
        &facade,
        context,
    );
    let tokens = quote! {
        #retained

        #[doc(hidden)]
        pub struct #marker;

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

        #codegen::traits::submit_trait_definition!(#marker, #spec_provider);
    };
    super::invocation::emit::rewrite_codegen_paths(tokens)
}
