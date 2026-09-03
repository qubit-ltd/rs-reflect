// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Expansion of reflected trait declarations and their registration fragments.

// qubit-style: allow multiple-public-types
// qubit-style: allow explicit-imports

mod default_invocation;
mod default_method_expansion;
mod dyn_trait_generics;
mod metadata;
mod trait_metadata;
mod token_rewrite;

use proc_macro2::Group;
use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::ToTokens;
use quote::format_ident;
use quote::quote;
use syn::Error;
use syn::FnArg;
use syn::GenericArgument;
use syn::GenericParam;
use syn::ItemFn;
use syn::ItemTrait;
use syn::Lifetime;
use syn::LitStr;
use syn::Path;
use syn::PathArguments as SynPathArguments;
use syn::Receiver;
use syn::TraitBoundModifier;
use syn::TraitItem;
use syn::TraitItemFn;
use syn::TraitItemType;
use syn::Type;
use syn::TypeParamBound;
use syn::WhereClause;
use syn::WherePredicate as SynWherePredicate;
use syn::parse_quote;
use syn::parse_quote_spanned;
use syn::parse2;
use token_rewrite::replace_self_with_owner;

use super::expression_codegen::external_supertrait_arguments;
pub(crate) use super::expression_codegen::generic_definition;
pub(crate) use super::expression_codegen::type_expression;
use crate::expand::ExpansionContext;
use crate::ir::GenericBoundIr;
use crate::ir::GenericKindIr;
use crate::ir::HelperName;
use crate::ir::HelperValueIr;
use crate::ir::PathArgumentIr;
use crate::ir::PathArgumentsIr;
use crate::ir::ReturnTypeIr;
use crate::ir::TraitDeclarationIr;
use crate::ir::TypeIr;
use crate::ir::TypeKindIr;
use crate::ir::WherePredicateIr;
use self::dyn_trait_generics::DynTraitGenerics;
use self::default_method_expansion::DefaultMethodExpansion;

/// Conservatively rejects unresolved lifetime shapes before the semantic
/// `Sized` probe. Rust method selection does not treat an unmet region
/// obligation as an autoref fallback candidate, so lifetime uncertainty must
/// not reach that probe.
fn associated_const_type_has_proven_static_shape(ty: &TypeIr) -> bool {
    associated_const_type_has_proven_static_shape_in(ty, &std::collections::HashSet::new(), false)
}

/// Recursively checks lifetime provenance while tracking higher-ranked
/// lifetimes and callable elision scopes.
fn associated_const_type_has_proven_static_shape_in(
    ty: &TypeIr,
    bound_lifetimes: &std::collections::HashSet<String>,
    callable_elision: bool,
) -> bool {
    match &ty.kind {
        TypeKindIr::Path(path) => {
            path.qualified_self.is_none()
                && !path.segments.iter().any(|segment| segment.name == "Self")
                && path
                    .segments
                    .iter()
                    .all(|segment| match &segment.arguments {
                        PathArgumentsIr::None => true,
                        PathArgumentsIr::AngleBracketed(arguments) => {
                            arguments.iter().all(|argument| match argument {
                                PathArgumentIr::Lifetime(lifetime) => {
                                    lifetime == "'static" || bound_lifetimes.contains(lifetime)
                                }
                                PathArgumentIr::Type(ty)
                                | PathArgumentIr::AssociatedType { ty, .. } => {
                                    associated_const_type_has_proven_static_shape_in(
                                        ty,
                                        bound_lifetimes,
                                        callable_elision,
                                    )
                                }
                                PathArgumentIr::Const(_) => true,
                                PathArgumentIr::AssociatedConst { .. }
                                | PathArgumentIr::Constraint { .. }
                                | PathArgumentIr::Other(_) => false,
                            })
                        }
                        PathArgumentsIr::Parenthesized { inputs, output } => {
                            inputs.iter().all(|input| {
                                associated_const_type_has_proven_static_shape_in(
                                    input,
                                    bound_lifetimes,
                                    true,
                                )
                            }) && output.as_deref().is_none_or(|output| {
                                associated_const_type_has_proven_static_shape_in(
                                    output,
                                    bound_lifetimes,
                                    true,
                                )
                            })
                        }
                    })
        }
        TypeKindIr::Reference {
            lifetime, element, ..
        } => {
            (lifetime.as_deref().is_some_and(|lifetime| {
                lifetime == "'static" || bound_lifetimes.contains(lifetime)
            }) || (lifetime.is_none() && callable_elision))
                && associated_const_type_has_proven_static_shape_in(
                    element,
                    bound_lifetimes,
                    callable_elision,
                )
        }
        TypeKindIr::Pointer { element, .. }
        | TypeKindIr::Slice(element)
        | TypeKindIr::Array { element, .. } => associated_const_type_has_proven_static_shape_in(
            element,
            bound_lifetimes,
            callable_elision,
        ),
        TypeKindIr::Tuple(elements) => elements.iter().all(|element| {
            associated_const_type_has_proven_static_shape_in(
                element,
                bound_lifetimes,
                callable_elision,
            )
        }),
        TypeKindIr::BareFunction {
            lifetimes,
            inputs,
            output,
            ..
        } => {
            let mut function_lifetimes = bound_lifetimes.clone();
            function_lifetimes.extend(lifetimes.iter().cloned());
            inputs.iter().all(|input| {
                associated_const_type_has_proven_static_shape_in(input, &function_lifetimes, true)
            }) && output.as_deref().is_none_or(|output| {
                associated_const_type_has_proven_static_shape_in(output, &function_lifetimes, true)
            })
        }
        TypeKindIr::TraitObject { bounds, .. } | TypeKindIr::ImplTrait { bounds } => {
            bounds.iter().all(|bound| match bound {
                GenericBoundIr::Lifetime(lifetime) => {
                    lifetime == "'static" || bound_lifetimes.contains(lifetime)
                }
                GenericBoundIr::Trait {
                    path, lifetimes, ..
                } => {
                    let mut trait_lifetimes = bound_lifetimes.clone();
                    trait_lifetimes.extend(lifetimes.iter().cloned());
                    path.segments
                        .iter()
                        .all(|segment| match &segment.arguments {
                            PathArgumentsIr::None => true,
                            PathArgumentsIr::AngleBracketed(arguments) => {
                                arguments.iter().all(|argument| match argument {
                                    PathArgumentIr::Lifetime(lifetime) => {
                                        lifetime == "'static" || trait_lifetimes.contains(lifetime)
                                    }
                                    PathArgumentIr::Type(ty)
                                    | PathArgumentIr::AssociatedType { ty, .. } => {
                                        associated_const_type_has_proven_static_shape_in(
                                            ty,
                                            &trait_lifetimes,
                                            callable_elision,
                                        )
                                    }
                                    PathArgumentIr::Const(_) => true,
                                    PathArgumentIr::AssociatedConst { .. }
                                    | PathArgumentIr::Constraint { .. }
                                    | PathArgumentIr::Other(_) => false,
                                })
                            }
                            PathArgumentsIr::Parenthesized { inputs, output } => {
                                inputs.iter().all(|input| {
                                    associated_const_type_has_proven_static_shape_in(
                                        input,
                                        &trait_lifetimes,
                                        true,
                                    )
                                }) && output.as_deref().is_none_or(|output| {
                                    associated_const_type_has_proven_static_shape_in(
                                        output,
                                        &trait_lifetimes,
                                        true,
                                    )
                                })
                            }
                        })
                }
                GenericBoundIr::Other(_) => false,
            })
        }
        TypeKindIr::Never => true,
        TypeKindIr::Infer | TypeKindIr::Macro | TypeKindIr::Other => false,
    }
}

/// Emits the normalized visibility carried by one reflected trait.
fn trait_visibility(declaration: &TraitDeclarationIr, facade: &TokenStream) -> TokenStream {
    match &declaration.visibility {
        crate::ir::VisibilityIr::Public => {
            quote!(#facade::__private::codegen_v1::identity::Visibility::Public)
        }
        crate::ir::VisibilityIr::Crate => {
            quote!(#facade::__private::codegen_v1::identity::Visibility::Crate)
        }
        crate::ir::VisibilityIr::Super => {
            quote!(#facade::__private::codegen_v1::identity::Visibility::Super)
        }
        crate::ir::VisibilityIr::SelfValue | crate::ir::VisibilityIr::Inherited => {
            quote!(#facade::__private::codegen_v1::identity::Visibility::Private)
        }
        crate::ir::VisibilityIr::Restricted(path) => {
            let path = LitStr::new(&path.source, declaration.span);
            quote!(#facade::__private::codegen_v1::identity::Visibility::Restricted(#path.into()))
        }
    }
}

/// Emits direct reflected and external supertrait applications.
fn direct_supertraits(declaration: &TraitDeclarationIr, facade: &TokenStream) -> Vec<TokenStream> {
    declaration
        .supertraits
        .iter()
        .filter_map(|bound| match bound {
            GenericBoundIr::Trait { path, .. } => {
                if let Some(external) = declaration
                    .external_traits
                    .iter()
                    .find(|external| external.path.source == path.source)
                {
                    let id = LitStr::new(&external.id, declaration.span);
                    let diagnostic_path = LitStr::new(&path.source, declaration.span);
                    let arguments = path
                        .segments
                        .last()
                        .map(|segment| {
                            external_supertrait_arguments(&segment.arguments, declaration, facade)
                        })
                        .unwrap_or_default();
                    Some(
                        quote!(#facade::__private::codegen_v1::descriptor::external_supertrait::<Self>(
                            #id,
                            #diagnostic_path,
                            vec![#(#arguments),*],
                        )),
                    )
                } else {
                    let path = &path.tokens;
                    Some(quote!(<Self as #path>::__qubit_reflect_trait_payload().applied()))
                }
            }
            _ => None,
        })
        .collect()
}

/// Emits concrete arguments applied to the reflected trait payload.
fn applied_arguments(declaration: &TraitDeclarationIr, facade: &TokenStream) -> Vec<TokenStream> {
    declaration
        .generics
        .params
        .iter()
        .filter_map(|parameter| {
            if parameter.kind == GenericKindIr::Const {
                let identifier = Ident::new(&parameter.name, declaration.span);
                let type_source = parameter.const_type.as_ref()?.source.as_str();
                let expression = match type_source {
                    "bool" => quote!(#facade::__private::codegen_v1::expression::ConstExpression::Boolean(#identifier)),
                    "char" => quote!(#facade::__private::codegen_v1::expression::ConstExpression::Character(#identifier)),
                    "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => {
                        quote!(#facade::__private::codegen_v1::expression::ConstExpression::SignedInteger(#identifier as i128))
                    }
                    "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => {
                        quote!(#facade::__private::codegen_v1::expression::ConstExpression::UnsignedInteger(#identifier as u128))
                    }
                    _ => return None,
                };
                let type_source = LitStr::new(type_source, declaration.span);
                return Some(quote!(#facade::__private::codegen_v1::expression::GenericArgument::Const(
                    #facade::__private::codegen_v1::expression::ConstGenericArgument::new(
                        #facade::__private::codegen_v1::expression::TypeExpression::Concrete(
                            #facade::__private::codegen_v1::expression::concrete(
                                vec![#type_source.into()].into_boxed_slice(),
                                vec![].into_boxed_slice(),
                                #facade::__private::codegen_v1::expression::DiagnosticText::from(#type_source),
                            ),
                        ),
                        #expression,
                        stringify!(#identifier),
                    ),
                )));
            }
            if parameter.kind != GenericKindIr::Type {
                return None;
            }
            let identifier = Ident::new(&parameter.name, declaration.span);
            Some(quote! {
                #facade::__private::codegen_v1::expression::GenericArgument::Type(
                    #facade::__private::codegen_v1::expression::TypeExpression::Concrete(
                        #facade::__private::codegen_v1::expression::concrete(
                            vec![std::any::type_name::<#identifier>().into()].into_boxed_slice(),
                            vec![].into_boxed_slice(),
                            #facade::__private::codegen_v1::expression::DiagnosticText::from(std::any::type_name::<#identifier>()),
                        ),
                    ),
                )
            })
        })
        .collect()
}

/// Emits the `'static` bounds required by generated trait payload hooks.
fn hook_type_bounds(declaration: &TraitDeclarationIr) -> Vec<TokenStream> {
    declaration
        .generics
        .params
        .iter()
        .filter(|parameter| parameter.kind == GenericKindIr::Type)
        .map(|parameter| {
            let identifier = Ident::new(&parameter.name, declaration.span);
            quote!(#identifier: 'static)
        })
        .collect()
}

/// Analyzes and emits all default-method adapter protocol entries.
fn default_method_expansion(
    declaration: &TraitDeclarationIr,
    suffix: &str,
    trait_name: &LitStr,
    hook_type_bounds: &[TokenStream],
    facade: &TokenStream,
    context: &ExpansionContext,
) -> DefaultMethodExpansion {
    let adapters: Vec<_> = declaration
        .methods
        .iter()
        .enumerate()
        .map(|(index, method)| {
            default_invocation::default_method_invocation_adapter(
                method,
                index,
                suffix,
                trait_name,
                hook_type_bounds,
                facade,
                context,
            )
        })
        .collect();
    let adapter_items = adapters
        .iter()
        .filter_map(|adapter| adapter.as_ref().map(|(item, _)| item.clone()))
        .collect();
    let adapter_entries = adapters
        .iter()
        .map(|adapter| match adapter {
            Some((_, entry)) => entry.clone(),
            None => quote!(None),
        })
        .collect();
    let unavailable_reason_entries = declaration
        .methods
        .iter()
        .map(|method| {
            let has_unproven_associated_type = method
                .parameters
                .iter()
                .any(|parameter| default_invocation::type_contains_associated_type(&parameter.ty))
                || matches!(&method.return_type, ReturnTypeIr::Type(ty) if default_invocation::type_contains_associated_type(ty));
            let plan = super::invocation::analysis::analyze_method(
                method,
                super::invocation::analysis::MethodContext::trait_default(
                    &quote!(Self),
                    has_unproven_associated_type,
                ),
            )
            .expect("validated method analysis is infallible");
            super::invocation::emit::emit_unavailable_reasons(&plan, context)
        })
        .collect();
    DefaultMethodExpansion {
        adapter_items,
        adapter_entries,
        unavailable_reason_entries,
    }
}

/// Emits optional concrete resolvers for nongeneric associated types.
fn associated_type_resolver_entries(
    declaration: &TraitDeclarationIr,
    facade: &TokenStream,
) -> Vec<TokenStream> {
    let codegen = quote!(#facade::__private::codegen_v1);
    declaration
        .associated_types
        .iter()
        .map(|item| {
            if item.generics.params.is_empty() {
                let name = &item.name;
                quote!({
                    use #codegen::descriptor::ResolveReflectTypeDescriptor as _;
                    let probe = #facade::__private::codegen_v1::descriptor::ReflectArgumentProbe::<Self::#name>::new();
                    (&probe).resolve_reflect_type_descriptor()
                })
            } else {
                quote!(None)
            }
        })
        .collect()
}

/// Expands a validated reflected trait without changing its ordinary Rust
/// semantics.
pub(crate) fn expand(declaration: TraitDeclarationIr, context: &ExpansionContext) -> TokenStream {
    let facade = context.facade().clone();
    let codegen = quote!(#facade::__private::codegen_v1);
    let fingerprint = context.fingerprint(&declaration.retained_tokens.to_string());
    let suffix = format!("{fingerprint:016x}");
    let marker = Ident::new(
        &format!("__QubitReflectTraitMarker_{suffix}"),
        declaration.span,
    );
    let hook = Ident::new("__qubit_reflect_trait_payload", declaration.span);
    let generic_factory = Ident::new(
        &format!("__qubit_reflect_trait_generics_{suffix}"),
        declaration.span,
    );
    let definition_factory = Ident::new(
        &format!("__qubit_reflect_trait_definition_{suffix}"),
        declaration.span,
    );
    let identity_factory = Ident::new(
        &format!("__qubit_reflect_trait_identity_{suffix}"),
        declaration.span,
    );
    let payload_factory = Ident::new(
        &format!("__qubit_reflect_trait_fragment_payload_{suffix}"),
        declaration.span,
    );
    let dyn_descriptor_factory = Ident::new(
        &format!("__qubit_reflect_dyn_trait_descriptor_{suffix}"),
        declaration.span,
    );
    let support = Ident::new(
        &format!("__qubit_reflect_trait_support_{suffix}"),
        declaration.span,
    );
    let rust_path = Ident::new(
        &format!("__QUBIT_REFLECT_TRAIT_PATH_{}", suffix.to_ascii_uppercase()),
        declaration.span,
    );
    let trait_name = declaration.name.to_string();
    let trait_name_literal = LitStr::new(&trait_name, declaration.span);
    let query_name = declaration
        .attributes
        .iter()
        .find_map(|attribute| attribute.rename())
        .unwrap_or(&trait_name)
        .to_owned();
    let query_name_literal = LitStr::new(&query_name, declaration.span);
    let visibility = trait_visibility(&declaration, &facade);
    let direct_supertraits = direct_supertraits(&declaration, &facade);
    let applied_arguments = applied_arguments(&declaration, &facade);
    let hook_type_bounds = hook_type_bounds(&declaration);
    let DefaultMethodExpansion {
        adapter_items: default_method_adapter_items,
        adapter_entries: default_method_adapter_entries,
        unavailable_reason_entries: default_method_unavailable_reason_entries,
    } = default_method_expansion(
        &declaration,
        &suffix,
        &trait_name_literal,
        &hook_type_bounds,
        &facade,
        context,
    );
    let associated_type_resolver_entries =
        associated_type_resolver_entries(&declaration, &facade);
    let associated_const_providers: Vec<_> = declaration
        .associated_consts
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let provider = format_ident!("__QubitReflectAssociatedConstProvider_{suffix}_{index}");
            let owner = format_ident!("__QubitReflectAssociatedConstOwner");
            let generic_declaration = replace_self_with_owner(declaration.generics.impl_declaration.clone(), &owner);
            let dummy: ItemFn = parse2(quote!(fn __qubit_reflect_dummy #generic_declaration() {}))
                .expect("validated trait generics can be reused by an associated-const provider");
            let mut provider_generics = dummy.sig.generics;
            provider_generics
                .params
                .push(parse_quote_spanned!(declaration.span=> #owner: ?Sized));
            let provider_declaration = quote!(#provider_generics);
            let (provider_impl_generics, provider_type_generics, _) = provider_generics.split_for_impl();
            let provider_impl_generics = quote!(#provider_impl_generics);
            let provider_type_generics = quote!(#provider_type_generics);
            let generic_arguments: Vec<_> = declaration
                .generics
                .params
                .iter()
                .map(|parameter| {
                    let name = Ident::new(&parameter.name, declaration.span);
                    if parameter.kind == GenericKindIr::Lifetime {
                        let lifetime = Lifetime::new(&format!("'{}", parameter.name), declaration.span);
                        quote!(#lifetime)
                    } else {
                        quote!(#name)
                    }
                })
                .collect();
            let marker_types: Vec<_> = declaration
                .generics
                .params
                .iter()
                .filter_map(|parameter| {
                    let name = Ident::new(&parameter.name, declaration.span);
                    match parameter.kind {
                        GenericKindIr::Lifetime => {
                            let lifetime = Lifetime::new(&format!("'{}", parameter.name), declaration.span);
                            Some(quote!(&#lifetime ()))
                        }
                        GenericKindIr::Type => Some(quote!(*const #name)),
                        GenericKindIr::Const => None,
                    }
                })
                .collect();
            let where_predicates = declaration.generics.where_predicates.iter().map(|predicate| {
                let tokens = match predicate {
                    WherePredicateIr::Lifetime { declaration, .. } | WherePredicateIr::Type { declaration, .. } => {
                        declaration.clone()
                    }
                    WherePredicateIr::Other(tokens) => tokens.clone(),
                };
                replace_self_with_owner(tokens, &owner)
            });
            let trait_name = &declaration.name;
            let trait_arguments = &declaration.generics.arguments;
            let const_name = &item.name;
            let value_type = replace_self_with_owner(item.ty.tokens.clone(), &owner);
            let has_proven_static_shape = associated_const_type_has_proven_static_shape(&item.ty);
            let provider_item = quote! {
                #[allow(non_camel_case_types)]
                pub(super) struct #provider #provider_declaration {
                    marker: ::core::marker::PhantomData<fn(
                        #(#marker_types,)*
                        *const #owner,
                    )>,
                }

                impl #provider_impl_generics __qubit_reflect_codegen::descriptor::AssociatedConstProvider
                    for #provider #provider_type_generics
                where
                    #(#where_predicates,)*
                    #owner: super::#trait_name #trait_arguments,
                {
                    type Value = #value_type;

                    fn get() -> Self::Value
                    where
                        Self::Value: Sized,
                    {
                        <#owner as super::#trait_name #trait_arguments>::#const_name
                    }
                }
            };
            let reader_entry = if has_proven_static_shape {
                quote!({
                    use #codegen::descriptor::ResolveAssociatedConstReader as _;
                    let probe = #facade::__private::codegen_v1::descriptor::AssociatedConstProbe::<
                        #support::#provider<#(#generic_arguments,)* Self>
                    >::new();
                    (&probe).resolve_associated_const_reader()
                })
            } else {
                quote!(None)
            };
            (provider_item, reader_entry)
        })
        .collect();
    let associated_const_provider_items = associated_const_providers
        .iter()
        .map(|(provider, _)| provider);
    let associated_const_scope_import = (!associated_const_providers.is_empty()).then(|| {
        quote! {
            use super::*;
        }
    });
    let associated_const_reader_entries =
        associated_const_providers.iter().map(|(_, reader)| reader);
    let trait_metadata::TraitMetadata {
        methods,
        associated_types,
        associated_consts,
        parameters,
        where_predicates,
    } = metadata::build(&declaration, &trait_name_literal, &facade);
    let mut trait_item: ItemTrait = match parse2(declaration.retained_tokens.clone()) {
        Ok(item) => item,
        Err(error) => return error.into_compile_error(),
    };
    let generate_dyn_descriptor = is_provably_dyn_compatible(&trait_item, &declaration);
    let trait_ident = &declaration.name;
    let dyn_generics = dyn_trait_generics(&trait_item, &declaration, &facade);
    let dyn_impl_declaration = &dyn_generics.impl_declaration;
    let dyn_application = &dyn_generics.trait_application;
    let dyn_factory_arguments = &dyn_generics.factory_arguments;
    let dyn_where_clause = &dyn_generics.where_clause;
    let dyn_type = quote!(dyn #trait_ident #dyn_application);
    let support_dyn_type = quote!(dyn super::#trait_ident #dyn_application);
    let associated_type_arguments = dyn_generics.associated_type_arguments.iter();
    let dyn_direct_supertraits: Vec<_> = declaration
        .supertraits
        .iter()
        .filter_map(|bound| match bound {
            GenericBoundIr::Trait { path, .. } => {
                if let Some(external) = declaration
                    .external_traits
                    .iter()
                    .find(|external| external.path.source == path.source)
                {
                    let id = LitStr::new(&external.id, declaration.span);
                    let diagnostic_path = LitStr::new(&path.source, declaration.span);
                    let arguments = path
                        .segments
                        .last()
                        .map(|segment| {
                            let mut arguments =
                                external_supertrait_arguments(&segment.arguments, &declaration, &facade);
                            arguments.extend(dyn_inherited_arguments_for_supertrait(path, &declaration, &facade));
                            arguments
                        })
                        .unwrap_or_default();
                    return Some(
                        quote!(#facade::__private::codegen_v1::descriptor::external_supertrait::<#support_dyn_type>(
                            #id,
                            #diagnostic_path,
                            vec![#(#arguments),*],
                        )),
                    );
                }
                let path = dyn_reflected_supertrait_path(path, &declaration);
                Some(quote!(
                    #facade::__private::codegen_v1::descriptor::TypeDescriptor::of::<dyn #path>()
                        .as_trait_object()
                        .expect("a reflected dyn-compatible supertrait has a trait-object descriptor")
                        .trait_descriptor()
                ))
            }
            _ => None,
        })
        .collect();
    let dyn_support = generate_dyn_descriptor.then(|| {
        quote! {
            #[doc(hidden)]
            pub(super) fn #dyn_descriptor_factory #dyn_impl_declaration ()
                -> &'static __qubit_reflect_codegen::descriptor::TraitDescriptor
                #dyn_where_clause
            {
            __qubit_reflect_codegen::descriptor::cached_trait_object_descriptor::<#support_dyn_type>(|| {
                        let definition = #definition_factory();
                        __qubit_reflect_codegen::descriptor::TraitDescriptor::builder(definition)
                            .arguments(vec![#(#applied_arguments),*])
                            .direct_supertraits([#(#dyn_direct_supertraits),*])
                            .methods(Box::leak(vec![#(#methods),*].into_boxed_slice()))
                            .associated_types(vec![#(#associated_types),*])
                            .associated_consts(vec![#(#associated_consts),*])
                            .associated_type_arguments(vec![#(#associated_type_arguments),*])
                            .build()
                            .expect("a generated dyn-compatible trait descriptor must be valid")
                })
            }
        }
    });
    let dyn_reflect_impl = generate_dyn_descriptor.then(|| {
        quote! {
            impl #dyn_impl_declaration #facade::__private::codegen_v1::descriptor::Reflect for #dyn_type
                #dyn_where_clause
            {
                fn type_descriptor() -> &'static #facade::__private::codegen_v1::descriptor::TypeDescriptor {
                    #facade::__private::codegen_v1::descriptor::intern_type::<Self>(|| {
                        #facade::__private::codegen_v1::descriptor::trait_object::<Self>(
                            #query_name_literal,
                            #support::#dyn_descriptor_factory::<#(#dyn_factory_arguments),*>,
                        )
                    })
                }
            }
        }
    });
    if trait_item
        .items
        .iter()
        .any(|item| matches!(item, TraitItem::Fn(function) if function.sig.ident == hook))
    {
        return Error::new(
            declaration.span,
            "`__qubit_reflect_trait_payload` is reserved by #[reflect]",
        )
        .into_compile_error();
    }
    let hook_item = match parse2(quote! {
        #[doc(hidden)]
        fn #hook() -> #facade::__private::codegen_v1::descriptor::TraitImplPayload
        where
            Self: Sized + 'static,
            #(#hook_type_bounds,)*
        {
            let definition = #support::#definition_factory();
            let arguments = vec![#(#applied_arguments),*];
            #facade::__private::codegen_v1::descriptor::TraitImplPayload::cached_with_arguments::<Self>(
                definition,
                arguments,
                |arguments| {
                    #facade::__private::codegen_v1::descriptor::TraitDescriptor::builder(definition)
                        .arguments(arguments)
                        .direct_supertraits([#(#direct_supertraits),*])
                        .methods(Box::leak(vec![#(#methods),*].into_boxed_slice()))
                        .associated_types(vec![#(#associated_types),*])
                        .associated_consts(vec![#(#associated_consts),*])
                        .build()
                },
                || vec![#(#default_method_adapter_entries),*],
                || vec![#(#default_method_unavailable_reason_entries),*],
                || vec![#(#associated_type_resolver_entries),*],
                || vec![#(#associated_const_reader_entries),*],
            )
        }
    }) {
        Ok(item) => item,
        Err(error) => return error.into_compile_error(),
    };
    for item in default_method_adapter_items {
        match parse2(item.clone()) {
            Ok(item) => trait_item.items.push(item),
            Err(error) => return error.into_compile_error(),
        }
    }
    trait_item.items.push(hook_item);
    quote! {
        #trait_item

        #dyn_reflect_impl

        #[doc(hidden)]
        const #rust_path: &str = concat!(module_path!(), "::", #trait_name_literal);

        #[doc(hidden)]
        mod #support {
            use #codegen as __qubit_reflect_codegen;
            #associated_const_scope_import

            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            struct #marker;

            #(#associated_const_provider_items)*

            #[doc(hidden)]
            fn #generic_factory() -> &'static __qubit_reflect_codegen::expression::GenericDefinitionDescriptor {
                static VALUE: std::sync::LazyLock<__qubit_reflect_codegen::expression::GenericDefinitionDescriptor> =
                    std::sync::LazyLock::new(|| __qubit_reflect_codegen::expression::GenericDefinitionDescriptor::new(
                        ::std::vec::Vec::from([#(#parameters),*]).into_boxed_slice(),
                        ::std::vec::Vec::from([#(#where_predicates),*]).into_boxed_slice(),
                    ));
                &VALUE
            }

            #[doc(hidden)]
            pub(super) fn #definition_factory() -> &'static __qubit_reflect_codegen::descriptor::TraitDefinitionDescriptor {
                static VALUE: std::sync::LazyLock<__qubit_reflect_codegen::descriptor::TraitDefinitionDescriptor> =
                    std::sync::LazyLock::new(|| __qubit_reflect_codegen::descriptor::TraitDefinitionDescriptor::new_with_visibility(
                        __qubit_reflect_codegen::descriptor::TraitId::Reflected(std::any::TypeId::of::<#marker>()),
                        #trait_name_literal,
                        super::#rust_path,
                        #query_name_literal,
                        __qubit_reflect_codegen::descriptor::TraitCompleteness::Complete,
                        #generic_factory(),
                        #visibility,
                    ));
                let definition = &*VALUE;
                definition.initialize_members(|definition| (
                    vec![#(#methods),*].into_boxed_slice(),
                    vec![#(#associated_types),*].into_boxed_slice(),
                    vec![#(#associated_consts),*].into_boxed_slice(),
                ));
                definition
            }

            #dyn_support

            #[doc(hidden)]
            fn #identity_factory() -> __qubit_reflect_codegen::registration::RuntimeIdentity {
                __qubit_reflect_codegen::registration::RuntimeIdentity::Trait(
                    __qubit_reflect_codegen::descriptor::TraitId::Reflected(std::any::TypeId::of::<#marker>()),
                )
            }

            #[doc(hidden)]
            fn #payload_factory() -> __qubit_reflect_codegen::registration::FragmentPayload {
                __qubit_reflect_codegen::registration::FragmentPayload::Trait(#definition_factory())
            }

            __qubit_reflect_codegen::inventory::submit! {
                __qubit_reflect_codegen::registration::RegistrationFragment::new(
                    __qubit_reflect_codegen::registration::FragmentKind::Trait,
                    __qubit_reflect_codegen::registration::StaticFragmentIdentity::new(
                        env!("CARGO_PKG_NAME"),
                        module_path!(),
                        line!(),
                        column!(),
                        "trait",
                        #fingerprint,
                    ),
                    #identity_factory,
                    #payload_factory,
                )
            }
        }
    }
}

/// Builds a `'static` concrete dyn application from the trait declaration.
fn dyn_trait_generics(
    item: &ItemTrait,
    declaration: &TraitDeclarationIr,
    facade: &TokenStream,
) -> DynTraitGenerics {
    let mut impl_parameters = Vec::new();
    let mut impl_predicates = Vec::new();
    let mut application_arguments = Vec::new();
    let mut factory_arguments = Vec::new();
    for parameter in &item.generics.params {
        match parameter {
            GenericParam::Lifetime(_) => application_arguments.push(quote!('static)),
            GenericParam::Type(parameter) => {
                let name = &parameter.ident;
                let bounds = &parameter.bounds;
                impl_parameters.push(quote!(#name));
                if bounds.is_empty() {
                    impl_predicates.push(quote!(#name: 'static));
                } else {
                    impl_predicates.push(quote!(#name: #bounds + 'static));
                }
                application_arguments.push(quote!(#name));
                factory_arguments.push(quote!(#name));
            }
            GenericParam::Const(parameter) => {
                let name = &parameter.ident;
                let ty = &parameter.ty;
                impl_parameters.push(quote!(const #name: #ty));
                application_arguments.push(quote!(#name));
                factory_arguments.push(quote!(#name));
            }
        }
    }
    let mut associated_type_arguments = Vec::new();
    for associated in item.items.iter().filter_map(|item| match item {
        TraitItem::Type(associated) if associated_type_requires_dyn_binding(associated) => {
            Some(associated)
        }
        _ => None,
    }) {
        let name = &associated.ident;
        let parameter = format_ident!("__QubitReflectAssociated{}", name);
        let bounds = &associated.bounds;
        impl_parameters.push(quote!(#parameter));
        if bounds.is_empty() {
            impl_predicates.push(quote!(#parameter: 'static));
        } else {
            impl_predicates.push(quote!(#parameter: #bounds + 'static));
        }
        application_arguments.push(quote!(#name = #parameter));
        factory_arguments.push(quote!(#parameter));
        let name_literal = LitStr::new(&name.to_string(), name.span());
        associated_type_arguments.push(quote!(
            #facade::__private::codegen_v1::expression::associated_type(
                #name_literal,
                #facade::__private::codegen_v1::expression::TypeExpression::Concrete(
                    #facade::__private::codegen_v1::expression::concrete(
                        vec![std::any::type_name::<#parameter>().into()].into_boxed_slice(),
                        vec![].into_boxed_slice(),
                        #facade::__private::codegen_v1::expression::DiagnosticText::from(std::any::type_name::<#parameter>()),
                    ),
                ),
            )
        ));
    }
    let direct_associated_names: std::collections::HashSet<_> = item
        .items
        .iter()
        .filter_map(|item| match item {
            TraitItem::Type(associated) => Some(associated.ident.to_string()),
            _ => None,
        })
        .collect();
    for inherited in dyn_inherited_associated_types(declaration) {
        let Some(segment) = inherited.segments.last() else {
            continue;
        };
        if direct_associated_names.contains(&segment.name) {
            continue;
        }
        let name = Ident::new(&segment.name, declaration.span);
        let parameter = format_ident!("__QubitReflectAssociated{}", name);
        impl_parameters.push(quote!(#parameter));
        impl_predicates.push(quote!(#parameter: 'static));
        application_arguments.push(quote!(#name = #parameter));
        factory_arguments.push(quote!(#parameter));
        let name_literal = LitStr::new(&segment.name, declaration.span);
        associated_type_arguments.push(quote!(
            #facade::__private::codegen_v1::expression::associated_type(
                #name_literal,
                #facade::__private::codegen_v1::expression::TypeExpression::Concrete(
                    #facade::__private::codegen_v1::expression::concrete(
                        vec![std::any::type_name::<#parameter>().into()].into_boxed_slice(),
                        vec![].into_boxed_slice(),
                        #facade::__private::codegen_v1::expression::DiagnosticText::from(std::any::type_name::<#parameter>()),
                    ),
                ),
            )
        ));
    }
    let impl_declaration = if impl_parameters.is_empty() {
        TokenStream::new()
    } else {
        quote!(<#(#impl_parameters),*>)
    };
    let trait_application = if application_arguments.is_empty() {
        TokenStream::new()
    } else {
        quote!(<#(#application_arguments),*>)
    };
    let declared_predicates: Vec<_> = item
        .generics
        .where_clause
        .iter()
        .flat_map(|clause| &clause.predicates)
        .map(|predicate| {
            let predicate =
                replace_declared_lifetimes_with_static(predicate.to_token_stream(), declaration);
            replace_self_associated_types(predicate, item, declaration)
        })
        .collect();
    let where_clause = if declared_predicates.is_empty() && impl_predicates.is_empty() {
        TokenStream::new()
    } else {
        quote!(where #(#declared_predicates,)* #(#impl_predicates),*)
    };
    DynTraitGenerics {
        impl_declaration,
        trait_application,
        factory_arguments,
        where_clause,
        associated_type_arguments,
    }
}

/// Replaces declared trait lifetime arguments with `'static` in generated dyn
/// applications and their where predicates.
fn replace_declared_lifetimes_with_static(
    tokens: TokenStream,
    declaration: &TraitDeclarationIr,
) -> TokenStream {
    let lifetime_names: std::collections::HashSet<_> = declaration
        .generics
        .params
        .iter()
        .filter(|parameter| parameter.kind == GenericKindIr::Lifetime)
        .map(|parameter| parameter.name.as_str())
        .collect();
    let mut input = tokens.into_iter().peekable();
    let mut output = TokenStream::new();
    while let Some(token) = input.next() {
        if matches!(&token, TokenTree::Punct(punctuation) if punctuation.as_char() == '\'')
            && input.peek().is_some_and(
                |next| matches!(next, TokenTree::Ident(identifier) if lifetime_names.contains(identifier.to_string().as_str())),
            )
        {
            output.extend([token]);
            let _ = input.next();
            output.extend([TokenTree::Ident(Ident::new("static", Span::call_site()))]);
            continue;
        }
        output.extend([match token {
            TokenTree::Group(group) => {
                let mut replaced = Group::new(
                    group.delimiter(),
                    replace_declared_lifetimes_with_static(group.stream(), declaration),
                );
                replaced.set_span(group.span());
                TokenTree::Group(replaced)
            }
            token => token,
        }]);
    }
    output
}

/// Rewrites `Self::Assoc` predicates to the generated dyn binding parameter.
fn replace_self_associated_types(
    tokens: TokenStream,
    item: &ItemTrait,
    declaration: &TraitDeclarationIr,
) -> TokenStream {
    let mut associated: std::collections::HashSet<_> = item
        .items
        .iter()
        .filter_map(|item| match item {
            TraitItem::Type(associated) if associated_type_requires_dyn_binding(associated) => {
                Some(associated.ident.to_string())
            }
            _ => None,
        })
        .collect();
    associated.extend(
        dyn_inherited_associated_types(declaration)
            .filter_map(|path| path.segments.last().map(|segment| segment.name.clone())),
    );
    let input: Vec<_> = tokens.into_iter().collect();
    let mut output = TokenStream::new();
    let mut index = 0;
    while index < input.len() {
        if let Some(
            [
                TokenTree::Ident(self_type),
                TokenTree::Punct(first),
                TokenTree::Punct(second),
                TokenTree::Ident(name),
            ],
        ) = input.get(index..index + 4)
            && self_type == "Self"
            && first.as_char() == ':'
            && second.as_char() == ':'
            && associated.contains(&name.to_string())
        {
            output.extend([TokenTree::Ident(Ident::new(
                &format!("__QubitReflectAssociated{name}"),
                name.span(),
            ))]);
            index += 4;
            continue;
        }
        output.extend([match input[index].clone() {
            TokenTree::Group(group) => {
                let mut replaced = Group::new(
                    group.delimiter(),
                    replace_self_associated_types(group.stream(), item, declaration),
                );
                replaced.set_span(group.span());
                TokenTree::Group(replaced)
            }
            token => token,
        }]);
        index += 1;
    }
    output
}

/// Returns inherited associated types explicitly proven for a dyn root.
fn dyn_inherited_associated_types(
    declaration: &TraitDeclarationIr,
) -> impl Iterator<Item = &crate::ir::PathIr> {
    declaration
        .attributes
        .iter()
        .filter_map(|attribute| match &attribute.value {
            HelperValueIr::DynCompatible(paths) => Some(paths.iter()),
            _ => None,
        })
        .flatten()
}

/// Adds explicit inherited bindings to one reflected dyn supertrait path.
fn dyn_reflected_supertrait_path(
    path: &crate::ir::PathIr,
    declaration: &TraitDeclarationIr,
) -> TokenStream {
    let mut syntax: Path = parse2(path.tokens.clone())
        .expect("validated reflected supertrait paths must parse as Rust paths");
    for inherited in dyn_inherited_associated_types(declaration)
        .filter(|inherited| inherited_belongs_to_supertrait(inherited, path))
    {
        let name = Ident::new(
            &inherited
                .segments
                .last()
                .expect("validated inherited associated path has an item")
                .name,
            declaration.span,
        );
        let parameter = format_ident!("__QubitReflectAssociated{}", name);
        let segment = syntax
            .segments
            .last_mut()
            .expect("validated supertrait path has a segment");
        match &mut segment.arguments {
            SynPathArguments::None => {
                segment.arguments = SynPathArguments::AngleBracketed(parse_quote!(
                    <#name = #parameter>
                ));
            }
            SynPathArguments::AngleBracketed(arguments) => {
                arguments.args.push(parse_quote!(#name = #parameter));
            }
            SynPathArguments::Parenthesized(_) => {}
        }
    }
    let syntax = if syntax.leading_colon.is_some() {
        quote!(#syntax)
    } else {
        quote!(super::#syntax)
    };
    replace_declared_lifetimes_with_static(syntax, declaration)
}

/// Builds inherited associated bindings for an external supertrait identity.
fn dyn_inherited_arguments_for_supertrait(
    path: &crate::ir::PathIr,
    declaration: &TraitDeclarationIr,
    facade: &TokenStream,
) -> Vec<TokenStream> {
    dyn_inherited_associated_types(declaration)
        .filter(|inherited| inherited_belongs_to_supertrait(inherited, path))
        .map(|inherited| {
            let name = inherited
                .segments
                .last()
                .expect("validated inherited associated path has an item")
                .name
                .clone();
            let name_literal = LitStr::new(&name, declaration.span);
            let parameter = format_ident!("__QubitReflectAssociated{name}");
            quote!(#facade::__private::codegen_v1::expression::associated_type(
                #name_literal,
                #facade::__private::codegen_v1::expression::TypeExpression::Concrete(
                    #facade::__private::codegen_v1::expression::concrete(
                        vec![std::any::type_name::<#parameter>().into()].into_boxed_slice(),
                        vec![].into_boxed_slice(),
                        #facade::__private::codegen_v1::expression::DiagnosticText::from(std::any::type_name::<#parameter>()),
                    ),
                ),
            ))
        })
        .collect()
}

/// Returns whether `Supertrait::Item` names an item on this direct bound.
fn inherited_belongs_to_supertrait(
    inherited: &crate::ir::PathIr,
    supertrait: &crate::ir::PathIr,
) -> bool {
    inherited.segments.len() == supertrait.segments.len() + 1
        && inherited
            .segments
            .iter()
            .zip(&supertrait.segments)
            .all(|(left, right)| left.name == right.name)
}

/// Returns whether the declaration is syntactically proven to admit a bare
/// `'static` trait object.
///
/// Generic traits are deliberately excluded because the requirements do not
/// define which concrete application a declaration-level macro should choose.
/// Supertraits are limited to standard traits whose dyn compatibility is known
/// without inspecting another macro expansion.
fn is_provably_dyn_compatible(item: &ItemTrait, declaration: &TraitDeclarationIr) -> bool {
    if declaration
        .attributes
        .iter()
        .any(|attribute| attribute.name == HelperName::DynCompatible)
    {
        return true;
    }
    if where_clause_requires_sized_self(item.generics.where_clause.as_ref())
        || item
            .generics
            .where_clause
            .as_ref()
            .is_some_and(|clause| tokens_contain_unprojected_self(clause.to_token_stream()))
        || !item.supertraits.iter().all(is_known_dyn_compatible_bound)
    {
        return false;
    }
    item.items.iter().all(|trait_item| match trait_item {
        TraitItem::Fn(method) => method_is_dyn_dispatchable(method),
        TraitItem::Type(associated) => {
            associated.generics.params.is_empty()
                || where_clause_requires_sized_self(associated.generics.where_clause.as_ref())
        }
        TraitItem::Const(_) => false,
        _ => false,
    })
}

/// Returns whether one supertrait bound is known locally to preserve dyn
/// compatibility.
fn is_known_dyn_compatible_bound(bound: &TypeParamBound) -> bool {
    match bound {
        TypeParamBound::Lifetime(_) => true,
        TypeParamBound::Trait(bound) => {
            if !matches!(bound.modifier, TraitBoundModifier::None)
                || tokens_contain_self(bound.to_token_stream())
            {
                return false;
            }
            let path = bound.path.to_token_stream().to_string().replace(' ', "");
            if matches!(
                path.as_str(),
                "Sized"
                    | "std::marker::Sized"
                    | "core::marker::Sized"
                    | "::std::marker::Sized"
                    | "::core::marker::Sized"
            ) {
                return false;
            }
            matches!(
                path.as_str(),
                "::std::fmt::Debug"
                    | "::core::fmt::Debug"
                    | "std::fmt::Debug"
                    | "core::fmt::Debug"
                    | "::std::fmt::Display"
                    | "::core::fmt::Display"
                    | "std::fmt::Display"
                    | "core::fmt::Display"
                    | "::std::marker::Send"
                    | "::core::marker::Send"
                    | "std::marker::Send"
                    | "core::marker::Send"
                    | "::std::marker::Sync"
                    | "::core::marker::Sync"
                    | "std::marker::Sync"
                    | "core::marker::Sync"
                    | "::std::marker::Unpin"
                    | "::core::marker::Unpin"
                    | "std::marker::Unpin"
                    | "core::marker::Unpin"
            )
        }
        _ => false,
    }
}

/// Returns whether a dyn application must name a concrete binding for this
/// associated type.
fn associated_type_requires_dyn_binding(associated: &TraitItemType) -> bool {
    !where_clause_requires_sized_self(associated.generics.where_clause.as_ref())
}

/// Returns whether one method is dispatchable through a trait object or is
/// explicitly excluded from the vtable by `Self: Sized`.
fn method_is_dyn_dispatchable(method: &TraitItemFn) -> bool {
    if where_clause_requires_sized_self(method.sig.generics.where_clause.as_ref()) {
        return true;
    }
    if method.sig.asyncness.is_some()
        || method
            .sig
            .generics
            .params
            .iter()
            .any(|parameter| !matches!(parameter, GenericParam::Lifetime(_)))
    {
        return false;
    }
    if method
        .sig
        .generics
        .where_clause
        .as_ref()
        .is_some_and(|clause| tokens_contain_unprojected_self(clause.to_token_stream()))
    {
        return false;
    }
    let Some(FnArg::Receiver(receiver)) = method.sig.inputs.first() else {
        return false;
    };
    if !receiver_is_dyn_dispatchable(receiver) {
        return false;
    }
    method.sig.inputs.iter().skip(1).all(|input| {
        let tokens = input.to_token_stream();
        !tokens_contain_unprojected_self(tokens.clone()) && !tokens_contain_ident(tokens, "impl")
    }) && {
        let output = method.sig.output.to_token_stream();
        !tokens_contain_unprojected_self(output.clone()) && !tokens_contain_ident(output, "impl")
    }
}

/// Returns whether a method receiver is one of Rust's dyn-dispatchable forms.
fn receiver_is_dyn_dispatchable(receiver: &Receiver) -> bool {
    if receiver.colon_token.is_none() {
        return true;
    }
    receiver_type_is_dyn_dispatchable(&receiver.ty)
}

/// Checks explicit `Self`, reference, smart-pointer, and pinned receiver types.
fn receiver_type_is_dyn_dispatchable(ty: &Type) -> bool {
    match ty {
        Type::Path(path) if path.qself.is_none() && path.path.is_ident("Self") => true,
        Type::Reference(reference) => receiver_type_is_dyn_dispatchable(&reference.elem),
        Type::Path(path) if path.qself.is_none() => {
            let Some(segment) = path.path.segments.last() else {
                return false;
            };
            if !matches!(
                segment.ident.to_string().as_str(),
                "Box" | "Rc" | "Arc" | "Pin"
            ) {
                return false;
            }
            let SynPathArguments::AngleBracketed(arguments) = &segment.arguments else {
                return false;
            };
            let mut types = arguments.args.iter().filter_map(|argument| match argument {
                GenericArgument::Type(ty) => Some(ty),
                _ => None,
            });
            let Some(inner) = types.next() else {
                return false;
            };
            types.next().is_none() && receiver_type_is_dyn_dispatchable(inner)
        }
        _ => false,
    }
}

/// Returns whether `Self` occurs outside an associated-type projection.
fn tokens_contain_unprojected_self(tokens: TokenStream) -> bool {
    let tokens: Vec<_> = tokens.into_iter().collect();
    tokens.iter().enumerate().any(|(index, token)| match token {
        TokenTree::Group(group) => tokens_contain_unprojected_self(group.stream()),
        TokenTree::Ident(identifier) if identifier == "Self" => !matches!(
            tokens.get(index + 1..index + 4),
            Some([
                TokenTree::Punct(first),
                TokenTree::Punct(second),
                TokenTree::Ident(_),
            ]) if first.as_char() == ':' && second.as_char() == ':'
        ),
        _ => false,
    })
}

/// Returns whether a where clause contains a direct `Self: Sized` predicate.
fn where_clause_requires_sized_self(where_clause: Option<&WhereClause>) -> bool {
    where_clause.is_some_and(|where_clause| {
        where_clause.predicates.iter().any(|predicate| {
            let SynWherePredicate::Type(predicate) = predicate else {
                return false;
            };
            matches!(predicate.bounded_ty, Type::Path(ref path) if path.qself.is_none() && path.path.is_ident("Self"))
                && predicate.bounds.iter().any(|bound| {
                    matches!(bound, TypeParamBound::Trait(bound)
                        if matches!(bound.modifier, TraitBoundModifier::None)
                            && bound.path.is_ident("Sized"))
                })
        })
    })
}

/// Returns whether a token stream contains the standalone `Self` type name.
fn tokens_contain_self(tokens: TokenStream) -> bool {
    tokens_contain_ident(tokens, "Self")
}

/// Returns whether a token stream contains one standalone identifier.
fn tokens_contain_ident(tokens: TokenStream, expected: &str) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Ident(identifier) => identifier == expected,
        TokenTree::Group(group) => tokens_contain_ident(group.stream(), expected),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

// Expression generation lives in `expand::expression_codegen`.
