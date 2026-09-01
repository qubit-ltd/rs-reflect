// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Expansion of reflected trait declarations and their registration fragments.

mod token_rewrite;

use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::ToTokens;
use quote::format_ident;
use quote::quote;

use crate::ir::GenericBoundIr;
use crate::ir::GenericKindIr;
use crate::ir::GenericsIr;
use crate::ir::HelperName;
use crate::ir::HelperValueIr;
use crate::ir::MethodIr;
use crate::ir::ParameterPatternKindIr;
use crate::ir::PathArgumentIr;
use crate::ir::PathArgumentsIr;
use crate::ir::ReceiverKindIr;
use crate::ir::ReturnTypeIr;
use crate::ir::TraitDeclarationIr;
use crate::ir::TypeIr;
use crate::ir::TypeKindIr;
use crate::ir::WherePredicateIr;

use token_rewrite::replace_self_with_owner;

/// Conservatively rejects unresolved lifetime shapes before the semantic
/// `Sized` probe. Rust method selection does not treat an unmet region
/// obligation as an autoref fallback candidate, so lifetime uncertainty must
/// not reach that probe.
fn associated_const_type_has_proven_static_shape(ty: &TypeIr) -> bool {
    associated_const_type_has_proven_static_shape_in(
        ty,
        &std::collections::HashSet::new(),
        false,
    )
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

/// Expands a validated reflected trait without changing its ordinary Rust
/// semantics.
pub(crate) fn expand(declaration: TraitDeclarationIr) -> TokenStream {
    let facade = match facade_path() {
        Some(path) => path,
        None => return declaration.retained_tokens,
    };
    let fingerprint = fingerprint(&declaration.retained_tokens.to_string());
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
    let trait_name_literal = syn::LitStr::new(&trait_name, declaration.span);
    let query_name = declaration
        .attributes
        .iter()
        .find_map(|attribute| attribute.rename())
        .unwrap_or(&trait_name)
        .to_owned();
    let query_name_literal = syn::LitStr::new(&query_name, declaration.span);
    let visibility = match &declaration.visibility {
        crate::ir::VisibilityIr::Public => {
            quote!(#facade::identity::Visibility::Public)
        }
        crate::ir::VisibilityIr::Crate => {
            quote!(#facade::identity::Visibility::Crate)
        }
        crate::ir::VisibilityIr::Super => {
            quote!(#facade::identity::Visibility::Super)
        }
        crate::ir::VisibilityIr::SelfValue
        | crate::ir::VisibilityIr::Inherited => {
            quote!(#facade::identity::Visibility::Private)
        }
        crate::ir::VisibilityIr::Restricted(path) => {
            let path = syn::LitStr::new(&path.source, declaration.span);
            quote!(#facade::identity::Visibility::Restricted(#path.into()))
        }
    };
    let direct_supertraits: Vec<_> = declaration
        .supertraits
        .iter()
        .filter_map(|bound| match bound {
            GenericBoundIr::Trait { path, .. } => {
                if let Some(external) = declaration
                    .external_traits
                    .iter()
                    .find(|external| external.path.source == path.source)
                {
                    let id = syn::LitStr::new(&external.id, declaration.span);
                    let diagnostic_path = syn::LitStr::new(&path.source, declaration.span);
                    let arguments = path
                        .segments
                        .last()
                        .map(|segment| {
                            external_supertrait_arguments(&segment.arguments, &declaration, &facade)
                        })
                        .unwrap_or_default();
                    Some(quote!(#facade::__private::external_supertrait::<Self>(
                        #id,
                        #diagnostic_path,
                        vec![#(#arguments),*],
                    )))
                } else {
                    let path = &path.tokens;
                    Some(quote!(<Self as #path>::__qubit_reflect_trait_payload().applied()))
                }
            }
            _ => None,
        })
        .collect();
    let applied_arguments: Vec<_> = declaration.generics.params.iter().filter_map(|parameter| {
        if parameter.kind == GenericKindIr::Const {
            let identifier = Ident::new(&parameter.name, declaration.span);
            let type_source = parameter.const_type.as_ref()?.source.as_str();
            let expression = match type_source {
                "bool" => quote!(#facade::expression::ConstExpression::Boolean(#identifier)),
                "char" => quote!(#facade::expression::ConstExpression::Character(#identifier)),
                "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => quote!(#facade::expression::ConstExpression::SignedInteger(#identifier as i128)),
                "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => quote!(#facade::expression::ConstExpression::UnsignedInteger(#identifier as u128)),
                _ => return None,
            };
            let type_source = syn::LitStr::new(type_source, declaration.span);
            return Some(quote!(#facade::expression::GenericArgument::Const(
                #facade::expression::ConstGenericArgument {
                    declared_type: Box::new(#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression {
                        path: Box::new([#type_source.into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::from(#type_source),
                    })),
                    value: #expression,
                    normalized_diagnostic: stringify!(#identifier).into(),
                },
            )));
        }
        if parameter.kind != GenericKindIr::Type { return None; }
        let identifier = Ident::new(&parameter.name, declaration.span);
        Some(quote! {
            #facade::expression::GenericArgument::Type(
                #facade::expression::TypeExpression::Concrete(
                    #facade::expression::ConcreteTypeExpression {
                        path: Box::new([std::any::type_name::<#identifier>().into()]),
                        arguments: Box::new([]),
                        diagnostic: #facade::expression::DiagnosticText::from(
                            std::any::type_name::<#identifier>(),
                        ),
                    },
                ),
            )
        })
    }).collect();
    let hook_type_bounds: Vec<_> = declaration
        .generics
        .params
        .iter()
        .filter_map(|parameter| {
            if parameter.kind != GenericKindIr::Type {
                return None;
            }
            let identifier = Ident::new(&parameter.name, declaration.span);
            Some(quote!(#identifier: 'static))
        })
        .collect();
    let default_method_adapters: Vec<_> = declaration
        .methods
        .iter()
        .enumerate()
        .map(|(index, method)| {
            default_method_invocation_adapter(
                method,
                index,
                &suffix,
                &trait_name_literal,
                &hook_type_bounds,
                &facade,
            )
        })
        .collect();
    let default_method_adapter_items = default_method_adapters
        .iter()
        .filter_map(|adapter| adapter.as_ref().map(|(item, _)| item));
    let default_method_adapter_entries =
        default_method_adapters.iter().map(|adapter| match adapter {
            Some((_, entry)) => entry.clone(),
            None => quote!(None),
        });
    let default_method_unavailable_reason_entries =
        declaration.methods.iter().map(|method| {
            super::impls::invocation_unavailable_reason_entry(
                method,
                &quote!(Self),
                &facade,
            )
        });
    let associated_type_resolver_entries = declaration.associated_types.iter().map(|item| {
        if item.generics.params.is_empty() {
            let name = &item.name;
            quote!({
                use #facade::__private::descriptor::ResolveReflectTypeDescriptor as _;
                let probe = #facade::__private::descriptor::ReflectArgumentProbe::<Self::#name>::new();
                (&probe).resolve_reflect_type_descriptor()
            })
        } else {
            quote!(None)
        }
    });
    let associated_const_providers: Vec<_> = declaration
        .associated_consts
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let provider = format_ident!("__QubitReflectAssociatedConstProvider_{suffix}_{index}");
            let owner = format_ident!("__QubitReflectAssociatedConstOwner");
            let generic_declaration = replace_self_with_owner(
                declaration.generics.impl_declaration.clone(),
                &owner,
            );
            let dummy: syn::ItemFn = syn::parse2(quote!(fn __qubit_reflect_dummy #generic_declaration() {}))
                .expect("validated trait generics can be reused by an associated-const provider");
            let mut provider_generics = dummy.sig.generics;
            provider_generics
                .params
                .push(syn::parse_quote_spanned!(declaration.span=> #owner: ?Sized));
            let provider_declaration = quote!(#provider_generics);
            let (provider_impl_generics, provider_type_generics, _) =
                provider_generics.split_for_impl();
            let provider_impl_generics = quote!(#provider_impl_generics);
            let provider_type_generics = quote!(#provider_type_generics);
            let generic_arguments: Vec<_> = declaration
                .generics
                .params
                .iter()
                .map(|parameter| {
                    let name = Ident::new(&parameter.name, declaration.span);
                    if parameter.kind == GenericKindIr::Lifetime {
                        let lifetime = syn::Lifetime::new(
                            &format!("'{}", parameter.name),
                            declaration.span,
                        );
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
                            let lifetime = syn::Lifetime::new(
                                &format!("'{}", parameter.name),
                                declaration.span,
                            );
                            Some(quote!(&#lifetime ()))
                        }
                        GenericKindIr::Type => Some(quote!(*const #name)),
                        GenericKindIr::Const => None,
                    }
                })
                .collect();
            let where_predicates = declaration.generics.where_predicates.iter().map(|predicate| {
                let tokens = match predicate {
                    WherePredicateIr::Lifetime { declaration, .. }
                    | WherePredicateIr::Type { declaration, .. } => declaration.clone(),
                    WherePredicateIr::Other(tokens) => tokens.clone(),
                };
                replace_self_with_owner(tokens, &owner)
            });
            let trait_name = &declaration.name;
            let trait_arguments = &declaration.generics.arguments;
            let const_name = &item.name;
            let value_type = replace_self_with_owner(item.ty.tokens.clone(), &owner);
            let has_proven_static_shape =
                associated_const_type_has_proven_static_shape(&item.ty);
            let provider_item = quote! {
                #[allow(non_camel_case_types)]
                pub(super) struct #provider #provider_declaration {
                    marker: ::core::marker::PhantomData<fn(
                        #(#marker_types,)*
                        *const #owner,
                    )>,
                }

                impl #provider_impl_generics __qubit_reflect::__private::descriptor::AssociatedConstProvider
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
                    use #facade::__private::descriptor::ResolveAssociatedConstReader as _;
                    let probe = #facade::__private::descriptor::AssociatedConstProbe::<
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
    let associated_const_scope_import =
        (!associated_const_providers.is_empty()).then(|| {
            quote! {
                #[allow(unused_imports)]
                use super::*;
            }
        });
    let associated_const_reader_entries =
        associated_const_providers.iter().map(|(_, reader)| reader);
    let methods: Vec<_> = declaration.methods.iter().enumerate().map(|(method_index, method)| {
        let name = syn::LitStr::new(&method.name.to_string(), method.span);
        let query = method
            .attributes
            .iter()
            .find_map(|attribute| attribute.rename())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| method.name.to_string());
        let query = syn::LitStr::new(&query, method.span);
        let index = method_index;
        let receiver = match &method.receiver {
            Some(receiver) => match receiver.kind {
                ReceiverKindIr::Value => quote!(Some(#facade::descriptor::ReceiverDescriptor::Owned)),
                ReceiverKindIr::SharedReference => quote!(Some(#facade::descriptor::ReceiverDescriptor::Shared)),
                ReceiverKindIr::MutableReference => quote!(Some(#facade::descriptor::ReceiverDescriptor::Mutable)),
                ReceiverKindIr::Typed => {
                    let declaration = syn::LitStr::new(&receiver.declaration.to_string(), receiver.span);
                    quote!(Some(#facade::descriptor::ReceiverDescriptor::Explicit(#declaration)))
                }
            },
            None => quote!(None),
        };
        let parameters = method.parameters.iter().map(|parameter| {
            let name = parameter
                .name
                .as_deref()
                .map(|name| syn::LitStr::new(name, parameter.span));
            let name = match name {
                Some(name) => quote!(Some(#name)),
                None => quote!(None),
            };
            let pattern = match parameter.pattern.kind {
                ParameterPatternKindIr::Identifier => {
                    quote!(#facade::descriptor::ParameterPatternDescriptor::Identifier)
                }
                ParameterPatternKindIr::Wildcard => quote!(#facade::descriptor::ParameterPatternDescriptor::Wildcard),
                ParameterPatternKindIr::Destructure => {
                    let source = syn::LitStr::new(&parameter.pattern.source, parameter.span);
                    quote!(#facade::descriptor::ParameterPatternDescriptor::Destructure(#source.into()))
                }
            };
            let passing = match &parameter.ty.kind {
                TypeKindIr::Reference { mutable: true, .. } => {
                    quote!(#facade::descriptor::ParameterPassingMode::MutableBorrow)
                }
                TypeKindIr::Reference { .. } => quote!(#facade::descriptor::ParameterPassingMode::SharedBorrow),
                _ => quote!(#facade::descriptor::ParameterPassingMode::Owned),
            };
            let ty = type_expression(&parameter.ty, &facade);
            let index = parameter.index;
            quote!(#facade::descriptor::ParameterDescriptor::new(#index, #name, #pattern, #passing, #ty, None))
        });
        let return_value = match &method.return_type {
            ReturnTypeIr::Unit => quote!(#facade::descriptor::ReturnDescriptor::unit()),
            ReturnTypeIr::Type(ty) => {
                let expression = type_expression(ty, &facade);
                let kind = match ty.kind {
                    TypeKindIr::Never => quote!(#facade::descriptor::ReturnKind::Never),
                    TypeKindIr::Reference { .. } => quote!(#facade::descriptor::ReturnKind::Reference),
                    TypeKindIr::ImplTrait { .. } => quote!(#facade::descriptor::ReturnKind::Opaque),
                    _ => quote!(#facade::descriptor::ReturnKind::Concrete),
                };
                quote!(#facade::descriptor::ReturnDescriptor::new(#kind, Some(#expression), None))
            }
        };
        let qualifiers = &method.qualifiers;
        let method_generic_definition = generic_definition(&method.generics, method.span, &facade);
        let is_async = qualifiers.is_async;
        let is_unsafe = qualifiers.is_unsafe;
        let is_const = qualifiers.is_const;
        let is_variadic = qualifiers.is_variadic;
        let has_default = method.has_default;
        let abi = qualifiers.abi.as_deref().map(|abi| syn::LitStr::new(abi, method.span));
        let abi = match abi {
            Some(abi) => quote!(Some(#facade::expression::FunctionAbi::Other(#abi.into()))),
            None => quote!(None),
        };
        quote! {
            #facade::descriptor::MethodDescriptor::builder(
                #facade::identity::MemberId::new(
                    #trait_name_literal,
                    "method",
                    #index,
                    #facade::identity::FragmentIdentity::new(
                        env!("CARGO_PKG_NAME"), module_path!(), line!(), column!(), "method", #index as u64,
                    ),
                ),
                #name,
                #query,
                #facade::descriptor::MethodDeclarationOwner::Trait(definition),
            )
            .visibility(#facade::descriptor::MethodVisibility::InheritedFromTrait)
            .receiver(#receiver)
            .parameters(vec![#(#parameters),*])
            .return_value(#return_value)
            .qualifiers(#facade::descriptor::MethodQualifiers {
                is_async: #is_async,
                is_unsafe: #is_unsafe,
                is_const: #is_const,
                abi: #abi,
                is_variadic: #is_variadic,
            })
            .generic_definition(&#method_generic_definition)
            .has_default(#has_default)
            .build()
        }
    }).collect();
    let associated_types: Vec<_> = declaration.associated_types.iter().enumerate().map(|(index, item)| {
        let name = syn::LitStr::new(&item.name.to_string(), item.span);
        let bounds = item.bounds.iter().filter_map(|bound| match bound {
            GenericBoundIr::Trait { path, lifetimes, modifier } => {
                let path = syn::LitStr::new(&path.source, item.span);
                let lifetimes = lifetimes
                    .iter()
                    .map(|lifetime| lifetime_expression(lifetime, item.span, &facade));
                let modifier = match modifier { crate::ir::TraitBoundModifierIr::None => quote!(#facade::expression::TraitBoundModifier::None), crate::ir::TraitBoundModifierIr::Maybe => quote!(#facade::expression::TraitBoundModifier::Maybe) };
                Some(quote!(#facade::expression::PredicateDescriptor::TypeBound {
                    subject: #facade::expression::TypeExpression::Parameter(#name.into()),
                    bounds: Box::new([#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression {
                        path: Box::new([#path.into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::from(#path),
                    })]), bound_modifiers: Box::new([#modifier]), higher_ranked_lifetimes: Box::new([#(#lifetimes),*]), diagnostic: #facade::expression::DiagnosticText::default(),
                }))
            }
            GenericBoundIr::Lifetime(lifetime) => {
                let lifetime = lifetime_expression(lifetime, item.span, &facade);
                Some(quote!(#facade::expression::PredicateDescriptor::TypeOutlives {
                    ty: #facade::expression::TypeExpression::Parameter(#name.into()),
                    lifetime: #lifetime,
                    diagnostic: #facade::expression::DiagnosticText::default(),
                }))
            }
            _ => None,
        });
        let default = item.value.as_ref().map(|value| type_expression(value, &facade));
        let default = match default { Some(value) => quote!(Some(#value)), None => quote!(None) };
        let generic_definition = generic_definition(&item.generics, item.span, &facade);
        quote!(#facade::descriptor::AssociatedTypeDescriptor::new_with_generic_definition(
            #index,
            #name,
            #name,
            Box::new([#(#bounds),*]),
            #default,
            #generic_definition,
        ))
    }).collect();
    let associated_consts: Vec<_> = declaration.associated_consts.iter().enumerate().map(|(index, item)| {
        let name = syn::LitStr::new(&item.name.to_string(), item.span);
        let ty = type_expression(&item.ty, &facade);
        let has_default = item.value.is_some();
        quote!(#facade::descriptor::AssociatedConstDescriptor::new(#index, #name, #name, #ty, #has_default))
    }).collect();
    let parameters = declaration.generics.params.iter().map(|parameter| {
        let name = syn::LitStr::new(&parameter.name, declaration.span);
        match parameter.kind {
            GenericKindIr::Lifetime => {
                let bounds = parameter.bounds.iter().filter_map(|bound| match bound {
                    GenericBoundIr::Lifetime(lifetime) => Some(lifetime_expression(
                        lifetime,
                        declaration.span,
                        &facade,
                    )),
                    _ => None,
                });
                quote! {
                __qubit_reflect::expression::GenericParameterDescriptor::Lifetime {
                    name: #name.into(),
                    bounds: Box::new([#(#bounds),*]),
                    diagnostic: __qubit_reflect::expression::DiagnosticText::default(),
                }
            }}
            GenericKindIr::Type => {
                let subject = syn::LitStr::new(&parameter.name, declaration.span);
                let bounds = parameter.bounds.iter().filter_map(|bound| match bound {
                    GenericBoundIr::Trait { path, lifetimes, modifier } => {
                        let path = syn::LitStr::new(&path.source, declaration.span);
                        let lifetimes = lifetimes.iter().map(|lifetime| {
                            lifetime_expression(lifetime, declaration.span, &facade)
                        });
                        let modifier = match modifier {
                            crate::ir::TraitBoundModifierIr::None => quote!(#facade::expression::TraitBoundModifier::None),
                            crate::ir::TraitBoundModifierIr::Maybe => quote!(#facade::expression::TraitBoundModifier::Maybe),
                        };
                        Some(quote!(#facade::expression::PredicateDescriptor::TypeBound {
                            subject: #facade::expression::TypeExpression::Parameter(#subject.into()),
                            bounds: Box::new([#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression {
                                path: Box::new([#path.into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::from(#path),
                            })]),
                            bound_modifiers: Box::new([#modifier]),
                            higher_ranked_lifetimes: Box::new([#(#lifetimes),*]), diagnostic: #facade::expression::DiagnosticText::default(),
                        }))
                    }
                    GenericBoundIr::Lifetime(lifetime) => {
                        let lifetime = lifetime_expression(lifetime, declaration.span, &facade);
                        Some(quote!(#facade::expression::PredicateDescriptor::TypeOutlives {
                            ty: #facade::expression::TypeExpression::Parameter(#subject.into()),
                            lifetime: #lifetime,
                            diagnostic: #facade::expression::DiagnosticText::default(),
                        }))
                    }
                    _ => None,
                });
                let default = match parameter.default.as_ref() {
                    Some(crate::ir::GenericDefaultIr::Type(value)) => {
                        let value = type_expression(value, &facade);
                        quote!(Some(#value))
                    }
                    _ => quote!(None),
                };
                quote! {
                __qubit_reflect::expression::GenericParameterDescriptor::Type {
                    name: #name.into(),
                    bounds: Box::new([#(#bounds),*]),
                    default: #default,
                    diagnostic: __qubit_reflect::expression::DiagnosticText::default(),
                }
            }}
            GenericKindIr::Const => {
                let const_type = parameter
                    .const_type
                    .as_ref()
                    .map(|value| value.source.as_str())
                    .unwrap_or("_");
                let const_type = syn::LitStr::new(const_type, declaration.span);
                let default = match parameter.default.as_ref() {
                    Some(crate::ir::GenericDefaultIr::Const(value)) => const_expression(value, &facade),
                    _ => quote!(None),
                };
                quote! {
                    __qubit_reflect::expression::GenericParameterDescriptor::Const {
                        name: #name.into(),
                        ty: Box::new(__qubit_reflect::expression::TypeExpression::Concrete(
                            __qubit_reflect::expression::ConcreteTypeExpression {
                                path: Box::new([#const_type.into()]),
                                arguments: Box::new([]),
                                diagnostic: __qubit_reflect::expression::DiagnosticText::from(#const_type),
                            },
                        )),
                        default: #default,
                        diagnostic: __qubit_reflect::expression::DiagnosticText::default(),
                    }
                }
            }
        }
    });
    let where_predicates = declaration.generics.where_predicates.iter().flat_map(|predicate| match predicate {
        crate::ir::WherePredicateIr::Lifetime { lifetime, bounds, .. } => {
            let lifetime = lifetime_expression(lifetime, declaration.span, &facade);
            let bounds: Vec<_> = bounds.iter().map(|bound| {
                lifetime_expression(bound, declaration.span, &facade)
            }).collect();
            vec![quote!(#facade::expression::PredicateDescriptor::LifetimeOutlives {
                lifetime: #lifetime,
                bounds: Box::new([#(#bounds),*]), diagnostic: #facade::expression::DiagnosticText::default(),
            })]
        }
        crate::ir::WherePredicateIr::Type {
            bounded_type,
            lifetimes,
            bounds,
            ..
        } => {
            let subject = type_expression(bounded_type, &facade);
            let higher_ranked_lifetimes: Vec<_> = lifetimes.iter().map(|lifetime| {
                lifetime_expression(lifetime, declaration.span, &facade)
            }).collect();
            let trait_bounds: Vec<_> = bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Trait { path, .. } => {
                    let path = syn::LitStr::new(&path.source, declaration.span);
                    Some(quote!(#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression {
                        path: Box::new([#path.into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::from(#path),
                    })))
                }
                _ => None,
            }).collect();
            let bound_modifiers: Vec<_> = bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Trait { modifier, .. } => Some(match modifier {
                    crate::ir::TraitBoundModifierIr::None => {
                        quote!(#facade::expression::TraitBoundModifier::None)
                    }
                    crate::ir::TraitBoundModifierIr::Maybe => {
                        quote!(#facade::expression::TraitBoundModifier::Maybe)
                    }
                }),
                _ => None,
            }).collect();
            let type_bound = (!trait_bounds.is_empty()).then(|| quote!(#facade::expression::PredicateDescriptor::TypeBound {
                subject: #subject,
                bounds: Box::new([#(#trait_bounds),*]),
                bound_modifiers: Box::new([#(#bound_modifiers),*]),
                higher_ranked_lifetimes: Box::new([#(#higher_ranked_lifetimes),*]),
                diagnostic: #facade::expression::DiagnosticText::default(),
            }));
            let lifetime_bounds = bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Lifetime(lifetime) => {
                    let lifetime = lifetime_expression(lifetime, declaration.span, &facade);
                    let subject = type_expression(bounded_type, &facade);
                    Some(quote!(#facade::expression::PredicateDescriptor::TypeOutlives {
                        ty: #subject,
                        lifetime: #lifetime,
                        diagnostic: #facade::expression::DiagnosticText::default(),
                    }))
                }
                _ => None,
            });
            type_bound.into_iter().chain(lifetime_bounds).collect()
        }
        _ => Vec::new(),
    });
    let mut trait_item: syn::ItemTrait =
        match syn::parse2(declaration.retained_tokens.clone()) {
            Ok(item) => item,
            Err(error) => return error.into_compile_error(),
        };
    let generate_dyn_descriptor =
        is_provably_dyn_compatible(&trait_item, &declaration);
    let trait_ident = &declaration.name;
    let dyn_generics = dyn_trait_generics(&trait_item, &declaration, &facade);
    let dyn_impl_declaration = &dyn_generics.impl_declaration;
    let dyn_application = &dyn_generics.trait_application;
    let dyn_factory_arguments = &dyn_generics.factory_arguments;
    let dyn_where_clause = &dyn_generics.where_clause;
    let dyn_type = quote!(dyn #trait_ident #dyn_application);
    let support_dyn_type = quote!(dyn super::#trait_ident #dyn_application);
    let associated_type_arguments =
        dyn_generics.associated_type_arguments.iter();
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
                    let id = syn::LitStr::new(&external.id, declaration.span);
                    let diagnostic_path = syn::LitStr::new(&path.source, declaration.span);
                    let arguments = path
                        .segments
                        .last()
                        .map(|segment| {
                            let mut arguments = external_supertrait_arguments(
                                &segment.arguments,
                                &declaration,
                                &facade,
                            );
                            arguments.extend(dyn_inherited_arguments_for_supertrait(
                                path,
                                &declaration,
                                &facade,
                            ));
                            arguments
                        })
                        .unwrap_or_default();
                    return Some(quote!(#facade::__private::external_supertrait::<#support_dyn_type>(
                        #id,
                        #diagnostic_path,
                        vec![#(#arguments),*],
                    )));
                }
                let path = dyn_reflected_supertrait_path(path, &declaration);
                Some(quote!(
                    #facade::descriptor::TypeDescriptor::of::<dyn #path>()
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
                -> &'static __qubit_reflect::descriptor::TraitDescriptor
                #dyn_where_clause
            {
            __qubit_reflect::__private::cached_trait_object_descriptor::<#support_dyn_type>(|| {
                        let definition = #definition_factory();
                        __qubit_reflect::descriptor::TraitDescriptor::builder(definition)
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
            impl #dyn_impl_declaration #facade::descriptor::Reflect for #dyn_type
                #dyn_where_clause
            {
                fn type_descriptor() -> &'static #facade::descriptor::TypeDescriptor {
                    #facade::__private::intern_type::<Self>(|| {
                        #facade::__private::descriptor::trait_object::<Self>(
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
        .any(|item| matches!(item, syn::TraitItem::Fn(function) if function.sig.ident == hook))
    {
        return syn::Error::new(
            declaration.span,
            "`__qubit_reflect_trait_payload` is reserved by #[reflect]",
        )
        .into_compile_error();
    }
    let hook_item = match syn::parse2(quote! {
        #[doc(hidden)]
        fn #hook() -> #facade::__private::TraitImplPayload
        where
            Self: Sized + 'static,
            #(#hook_type_bounds,)*
        {
            let definition = #support::#definition_factory();
            let arguments = vec![#(#applied_arguments),*];
            #facade::__private::TraitImplPayload::cached_with_arguments::<Self>(
                definition,
                arguments,
                |arguments| {
                    #facade::descriptor::TraitDescriptor::builder(definition)
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
        match syn::parse2(item.clone()) {
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
            use #facade as __qubit_reflect;
            #associated_const_scope_import

            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            struct #marker;

            #(#associated_const_provider_items)*

            #[doc(hidden)]
            fn #generic_factory() -> &'static __qubit_reflect::expression::GenericDefinitionDescriptor {
                static VALUE: std::sync::LazyLock<__qubit_reflect::expression::GenericDefinitionDescriptor> =
                    std::sync::LazyLock::new(|| __qubit_reflect::expression::GenericDefinitionDescriptor {
                        parameters: Box::new([#(#parameters),*]),
                        predicates: Box::new([#(#where_predicates),*]),
                        diagnostic: __qubit_reflect::expression::DiagnosticText::default(),
                    });
                &VALUE
            }

            #[doc(hidden)]
            pub(super) fn #definition_factory() -> &'static __qubit_reflect::descriptor::TraitDefinitionDescriptor {
                static VALUE: std::sync::LazyLock<__qubit_reflect::descriptor::TraitDefinitionDescriptor> =
                    std::sync::LazyLock::new(|| __qubit_reflect::descriptor::TraitDefinitionDescriptor::new_with_visibility(
                        __qubit_reflect::descriptor::TraitId::Reflected(std::any::TypeId::of::<#marker>()),
                        #trait_name_literal,
                        super::#rust_path,
                        #query_name_literal,
                        __qubit_reflect::descriptor::TraitCompleteness::Complete,
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
            fn #identity_factory() -> __qubit_reflect::__private::RuntimeIdentity {
                __qubit_reflect::__private::RuntimeIdentity::Trait(
                    __qubit_reflect::descriptor::TraitId::Reflected(std::any::TypeId::of::<#marker>()),
                )
            }

            #[doc(hidden)]
            fn #payload_factory() -> __qubit_reflect::__private::FragmentPayload {
                __qubit_reflect::__private::FragmentPayload::Trait(#definition_factory())
            }

            __qubit_reflect::__private::inventory::submit! {
                __qubit_reflect::__private::RegistrationFragment::new(
                    __qubit_reflect::__private::FragmentKind::Trait,
                    __qubit_reflect::__private::StaticFragmentIdentity::new(
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

/// Generated generic syntax and associated-type identities for one dyn root.
struct DynTraitGenerics {
    impl_declaration: TokenStream,
    trait_application: TokenStream,
    factory_arguments: Vec<TokenStream>,
    where_clause: TokenStream,
    associated_type_arguments: Vec<TokenStream>,
}

/// Builds a `'static` concrete dyn application from the trait declaration.
fn dyn_trait_generics(
    item: &syn::ItemTrait,
    declaration: &TraitDeclarationIr,
    facade: &TokenStream,
) -> DynTraitGenerics {
    let mut impl_parameters = Vec::new();
    let mut impl_predicates = Vec::new();
    let mut application_arguments = Vec::new();
    let mut factory_arguments = Vec::new();
    for parameter in &item.generics.params {
        match parameter {
            syn::GenericParam::Lifetime(_) => {
                application_arguments.push(quote!('static))
            }
            syn::GenericParam::Type(parameter) => {
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
            syn::GenericParam::Const(parameter) => {
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
        syn::TraitItem::Type(associated)
            if associated_type_requires_dyn_binding(associated) =>
        {
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
        let name_literal = syn::LitStr::new(&name.to_string(), name.span());
        associated_type_arguments.push(quote!(
            #facade::expression::GenericArgument::AssociatedType {
                name: #name_literal.into(),
                value: Box::new(#facade::expression::TypeExpression::Concrete(
                    #facade::expression::ConcreteTypeExpression {
                        path: Box::new([std::any::type_name::<#parameter>().into()]),
                        arguments: Box::new([]),
                        diagnostic: #facade::expression::DiagnosticText::from(
                            std::any::type_name::<#parameter>(),
                        ),
                    },
                )),
            }
        ));
    }
    let direct_associated_names: std::collections::HashSet<_> = item
        .items
        .iter()
        .filter_map(|item| match item {
            syn::TraitItem::Type(associated) => {
                Some(associated.ident.to_string())
            }
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
        let name_literal = syn::LitStr::new(&segment.name, declaration.span);
        associated_type_arguments.push(quote!(
            #facade::expression::GenericArgument::AssociatedType {
                name: #name_literal.into(),
                value: Box::new(#facade::expression::TypeExpression::Concrete(
                    #facade::expression::ConcreteTypeExpression {
                        path: Box::new([std::any::type_name::<#parameter>().into()]),
                        arguments: Box::new([]),
                        diagnostic: #facade::expression::DiagnosticText::from(
                            std::any::type_name::<#parameter>(),
                        ),
                    },
                )),
            }
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
            let predicate = replace_declared_lifetimes_with_static(
                predicate.to_token_stream(),
                declaration,
            );
            replace_self_associated_types(predicate, item, declaration)
        })
        .collect();
    let where_clause =
        if declared_predicates.is_empty() && impl_predicates.is_empty() {
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
                let mut replaced = proc_macro2::Group::new(
                    group.delimiter(),
                    replace_declared_lifetimes_with_static(
                        group.stream(),
                        declaration,
                    ),
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
    item: &syn::ItemTrait,
    declaration: &TraitDeclarationIr,
) -> TokenStream {
    let mut associated: std::collections::HashSet<_> = item
        .items
        .iter()
        .filter_map(|item| match item {
            syn::TraitItem::Type(associated)
                if associated_type_requires_dyn_binding(associated) =>
            {
                Some(associated.ident.to_string())
            }
            _ => None,
        })
        .collect();
    associated.extend(dyn_inherited_associated_types(declaration).filter_map(
        |path| path.segments.last().map(|segment| segment.name.clone()),
    ));
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
                let mut replaced = proc_macro2::Group::new(
                    group.delimiter(),
                    replace_self_associated_types(
                        group.stream(),
                        item,
                        declaration,
                    ),
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
    let mut syntax: syn::Path = syn::parse2(path.tokens.clone()).expect(
        "validated reflected supertrait paths must parse as Rust paths",
    );
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
            syn::PathArguments::None => {
                segment.arguments =
                    syn::PathArguments::AngleBracketed(syn::parse_quote!(
                        <#name = #parameter>
                    ));
            }
            syn::PathArguments::AngleBracketed(arguments) => {
                arguments.args.push(syn::parse_quote!(#name = #parameter));
            }
            syn::PathArguments::Parenthesized(_) => {}
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
            let name_literal = syn::LitStr::new(&name, declaration.span);
            let parameter = format_ident!("__QubitReflectAssociated{name}");
            quote!(#facade::expression::GenericArgument::AssociatedType {
                name: #name_literal.into(),
                value: Box::new(#facade::expression::TypeExpression::Concrete(
                    #facade::expression::ConcreteTypeExpression {
                        path: Box::new([std::any::type_name::<#parameter>().into()]),
                        arguments: Box::new([]),
                        diagnostic: #facade::expression::DiagnosticText::from(
                            std::any::type_name::<#parameter>(),
                        ),
                    },
                )),
            })
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
fn is_provably_dyn_compatible(
    item: &syn::ItemTrait,
    declaration: &TraitDeclarationIr,
) -> bool {
    if declaration
        .attributes
        .iter()
        .any(|attribute| attribute.name == HelperName::DynCompatible)
    {
        return true;
    }
    if where_clause_requires_sized_self(item.generics.where_clause.as_ref())
        || item.generics.where_clause.as_ref().is_some_and(|clause| {
            tokens_contain_unprojected_self(clause.to_token_stream())
        })
        || !item.supertraits.iter().all(is_known_dyn_compatible_bound)
    {
        return false;
    }
    item.items.iter().all(|trait_item| match trait_item {
        syn::TraitItem::Fn(method) => method_is_dyn_dispatchable(method),
        syn::TraitItem::Type(associated) => {
            associated.generics.params.is_empty()
                || where_clause_requires_sized_self(
                    associated.generics.where_clause.as_ref(),
                )
        }
        syn::TraitItem::Const(_) => false,
        _ => false,
    })
}

/// Returns whether one supertrait bound is known locally to preserve dyn
/// compatibility.
fn is_known_dyn_compatible_bound(bound: &syn::TypeParamBound) -> bool {
    match bound {
        syn::TypeParamBound::Lifetime(_) => true,
        syn::TypeParamBound::Trait(bound) => {
            if !matches!(bound.modifier, syn::TraitBoundModifier::None)
                || tokens_contain_self(bound.to_token_stream())
            {
                return false;
            }
            let path =
                bound.path.to_token_stream().to_string().replace(' ', "");
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
fn associated_type_requires_dyn_binding(
    associated: &syn::TraitItemType,
) -> bool {
    !where_clause_requires_sized_self(associated.generics.where_clause.as_ref())
}

/// Returns whether one method is dispatchable through a trait object or is
/// explicitly excluded from the vtable by `Self: Sized`.
fn method_is_dyn_dispatchable(method: &syn::TraitItemFn) -> bool {
    if where_clause_requires_sized_self(
        method.sig.generics.where_clause.as_ref(),
    ) {
        return true;
    }
    if method.sig.asyncness.is_some()
        || method.sig.generics.params.iter().any(|parameter| {
            !matches!(parameter, syn::GenericParam::Lifetime(_))
        })
    {
        return false;
    }
    if method
        .sig
        .generics
        .where_clause
        .as_ref()
        .is_some_and(|clause| {
            tokens_contain_unprojected_self(clause.to_token_stream())
        })
    {
        return false;
    }
    let Some(syn::FnArg::Receiver(receiver)) = method.sig.inputs.first() else {
        return false;
    };
    if !receiver_is_dyn_dispatchable(receiver) {
        return false;
    }
    method.sig.inputs.iter().skip(1).all(|input| {
        let tokens = input.to_token_stream();
        !tokens_contain_unprojected_self(tokens.clone())
            && !tokens_contain_ident(tokens, "impl")
    }) && {
        let output = method.sig.output.to_token_stream();
        !tokens_contain_unprojected_self(output.clone())
            && !tokens_contain_ident(output, "impl")
    }
}

/// Returns whether a method receiver is one of Rust's dyn-dispatchable forms.
fn receiver_is_dyn_dispatchable(receiver: &syn::Receiver) -> bool {
    if receiver.colon_token.is_none() {
        return true;
    }
    receiver_type_is_dyn_dispatchable(&receiver.ty)
}

/// Checks explicit `Self`, reference, smart-pointer, and pinned receiver types.
fn receiver_type_is_dyn_dispatchable(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(path)
            if path.qself.is_none() && path.path.is_ident("Self") =>
        {
            true
        }
        syn::Type::Reference(reference) => {
            receiver_type_is_dyn_dispatchable(&reference.elem)
        }
        syn::Type::Path(path) if path.qself.is_none() => {
            let Some(segment) = path.path.segments.last() else {
                return false;
            };
            if !matches!(
                segment.ident.to_string().as_str(),
                "Box" | "Rc" | "Arc" | "Pin"
            ) {
                return false;
            }
            let syn::PathArguments::AngleBracketed(arguments) =
                &segment.arguments
            else {
                return false;
            };
            let mut types =
                arguments.args.iter().filter_map(|argument| match argument {
                    syn::GenericArgument::Type(ty) => Some(ty),
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
        TokenTree::Group(group) => {
            tokens_contain_unprojected_self(group.stream())
        }
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
fn where_clause_requires_sized_self(
    where_clause: Option<&syn::WhereClause>,
) -> bool {
    where_clause.is_some_and(|where_clause| {
        where_clause.predicates.iter().any(|predicate| {
            let syn::WherePredicate::Type(predicate) = predicate else {
                return false;
            };
            matches!(predicate.bounded_ty, syn::Type::Path(ref path) if path.qself.is_none() && path.path.is_ident("Self"))
                && predicate.bounds.iter().any(|bound| {
                    matches!(bound, syn::TypeParamBound::Trait(bound)
                        if matches!(bound.modifier, syn::TraitBoundModifier::None)
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
        TokenTree::Group(group) => {
            tokens_contain_ident(group.stream(), expected)
        }
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

/// Generates one concrete local adapter hook for a safely erasable default
/// method and the payload entry that exposes it to reflected impl expansion.
fn default_method_invocation_adapter(
    method: &MethodIr,
    index: usize,
    suffix: &str,
    trait_name: &syn::LitStr,
    hook_type_bounds: &[TokenStream],
    facade: &TokenStream,
) -> Option<(TokenStream, TokenStream)> {
    let target = quote!(Self);
    let typed_owned_receiver = method.receiver.as_ref().and_then(|receiver| {
        super::impls::typed_owned_receiver_type(receiver, &target)
    });
    let typed_pinned_receiver = method
        .receiver
        .as_ref()
        .and_then(super::impls::typed_pinned_receiver_mutable);
    let supported_receiver = matches!(
        method.receiver.as_ref().map(|receiver| receiver.kind),
        None | Some(ReceiverKindIr::Value)
            | Some(ReceiverKindIr::SharedReference)
            | Some(ReceiverKindIr::MutableReference)
    ) || typed_owned_receiver.is_some()
        || typed_pinned_receiver.is_some();
    let supported_parameters = method.parameters.iter().all(|parameter| {
        super::impls::supports_invocation_parameter(&parameter.ty)
    });
    let has_unproven_associated_type = method
        .parameters
        .iter()
        .any(|parameter| type_contains_associated_type(&parameter.ty))
        || matches!(
            &method.return_type,
            ReturnTypeIr::Type(ty) if type_contains_associated_type(ty)
        );
    if let Some(pinned_mutable) = typed_pinned_receiver {
        let supported = method.has_default
            && supported_parameters
            && method.generics.params.is_empty()
            && method.generics.where_predicates.is_empty()
            && !has_unproven_associated_type
            && !method.qualifiers.is_async
            && !method.qualifiers.is_unsafe
            && method.qualifiers.abi.is_none()
            && !method.qualifiers.is_variadic
            && !super::impls::is_borrow_return(&method.return_type)
            && !method.attributes.iter().any(|attribute| {
                matches!(
                    attribute.name,
                    HelperName::Skip
                        | HelperName::NoInvoke
                        | HelperName::ThreadSafe
                        | HelperName::CatchUnwind
                )
            })
            && super::impls::supports_invocation_return(&method.return_type);
        return supported.then(|| {
            default_pinned_method_invocation_adapter(
                method,
                index,
                suffix,
                trait_name,
                hook_type_bounds,
                facade,
                pinned_mutable,
            )
        });
    }
    let supported = method.has_default
        && supported_receiver
        && supported_parameters
        && method.generics.params.is_empty()
        && method.generics.where_predicates.is_empty()
        && !has_unproven_associated_type
        && !method.qualifiers.is_unsafe
        && method.qualifiers.abi.is_none()
        && !method.qualifiers.is_variadic
        && (!super::impls::return_contains_non_static_lifetime(
            &method.return_type,
        ) || super::impls::is_supported_shared_borrow_return(
            &method.return_type,
        ) || super::impls::is_supported_mutable_borrow_return(method))
        && (!method.qualifiers.is_async
            || !super::impls::is_borrow_return(&method.return_type))
        && !method.attributes.iter().any(|attribute| {
            matches!(attribute.name, HelperName::Skip | HelperName::NoInvoke)
        })
        && super::impls::supports_invocation_return(&method.return_type);
    if !supported {
        return None;
    }

    let adapter_name =
        format_ident!("__qubit_reflect_invoke_default_{suffix}_{index}");
    let method_name = &method.name;
    let thread_safe = method
        .attributes
        .iter()
        .any(|attribute| attribute.name == HelperName::ThreadSafe);
    let catching_requested = method
        .attributes
        .iter()
        .any(|attribute| attribute.name == HelperName::CatchUnwind)
        && !method.qualifiers.is_async;
    let mode = if thread_safe {
        quote!(#facade::value::ThreadSafe)
    } else {
        quote!(#facade::value::Local)
    };
    let thread_safe_assertions = thread_safe.then(|| {
        super::impls::thread_safe_invocation_assertions(
            method,
            &target,
            typed_owned_receiver.as_ref(),
            None,
        )
    });
    let catching_assertions = catching_requested.then(|| {
        super::impls::catching_invocation_assertions(
            method,
            &target,
            typed_owned_receiver.as_ref(),
            None,
        )
    });
    let receiver_expectation = if matches!(
        method.receiver.as_ref().map(|receiver| receiver.kind),
        Some(ReceiverKindIr::Value)
    ) {
        quote!(#facade::invoke::ReceiverExpectation::owned::<Self>())
    } else if let Some(receiver_type) = &typed_owned_receiver {
        quote!(#facade::invoke::ReceiverExpectation::owned::<#receiver_type>())
    } else {
        match method.receiver.as_ref().map(|receiver| receiver.kind) {
            Some(ReceiverKindIr::Value) => {
                quote!(#facade::invoke::ReceiverExpectation::owned::<Self>())
            }
            Some(ReceiverKindIr::MutableReference) => {
                quote!(#facade::invoke::ReceiverExpectation::borrowed_mut::<Self>())
            }
            Some(ReceiverKindIr::SharedReference) => {
                quote!(#facade::invoke::ReceiverExpectation::borrowed::<Self>())
            }
            Some(ReceiverKindIr::Typed) => return None,
            None => quote!(#facade::invoke::ReceiverExpectation::none()),
        }
    };
    let receiver_binding = match method
        .receiver
        .as_ref()
        .map(|receiver| receiver.kind)
    {
        Some(ReceiverKindIr::Value) => quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver: Self = match receiver {
                Some(#facade::invoke::InvocationReceiver::Owned(value)) =>
                    #facade::value::DynamicOwned::<#mode>::downcast::<Self>(value)
                        .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                _ => unreachable!("validation checked receiver mode"),
            };
        },
        Some(ReceiverKindIr::Typed) if typed_owned_receiver.is_some() => {
            let receiver_type =
                typed_owned_receiver.as_ref().expect("checked above");
            quote! {
                let (receiver, arguments) = validated.into_parts();
                let receiver: #receiver_type = match receiver {
                    Some(#facade::invoke::InvocationReceiver::Owned(value)) =>
                        #facade::value::DynamicOwned::<#mode>::downcast::<#receiver_type>(value)
                            .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                    _ => unreachable!("validation checked receiver mode"),
                };
            }
        }
        Some(ReceiverKindIr::MutableReference) => quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver: &mut Self = match receiver {
                Some(#facade::invoke::InvocationReceiver::Mut(value)) =>
                    #facade::value::DynamicMut::<#mode>::downcast::<Self>(value)
                        .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                _ => unreachable!("validation checked receiver mode"),
            };
        },
        Some(ReceiverKindIr::SharedReference) => quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver: &Self = match receiver {
                Some(#facade::invoke::InvocationReceiver::Ref(value)) =>
                    #facade::value::DynamicRef::<#mode>::downcast::<Self>(value)
                        .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                Some(#facade::invoke::InvocationReceiver::Mut(value)) => {
                    let value = #facade::value::DynamicMut::<#mode>::downcast::<Self>(value)
                        .unwrap_or_else(|_| unreachable!("validation checked receiver type"));
                    &*value
                }
                _ => unreachable!("validation checked receiver mode"),
            };
        },
        Some(ReceiverKindIr::Typed) => return None,
        None => quote! {
            let (_receiver, arguments) = validated.into_parts();
        },
    };
    let parameter_expectations: Vec<_> = method
        .parameters
        .iter()
        .map(|parameter| {
            super::impls::invocation_argument_expectation(parameter, facade)
        })
        .collect();
    let argument_bindings: Vec<_> = method
        .parameters
        .iter()
        .map(|parameter| {
            super::impls::invocation_argument_binding(parameter, facade, &mode)
        })
        .collect();
    let call_arguments: Vec<_> = method
        .parameters
        .iter()
        .map(|parameter| {
            format_ident!("__qubit_reflect_argument_{}", parameter.index)
        })
        .collect();
    let call = if method.receiver.is_some() {
        quote!(Self::#method_name(receiver, #(#call_arguments),*))
    } else {
        quote!(Self::#method_name(#(#call_arguments),*))
    };
    let borrow_origins: Vec<_> = std::iter::once(
        method
            .receiver
            .is_some()
            .then(|| quote!(#facade::invoke::BorrowOrigin::Receiver)),
    )
    .flatten()
    .chain(
        method
            .parameters
            .iter()
            .filter(|parameter| {
                matches!(parameter.ty.kind, TypeKindIr::Reference { .. })
            })
            .map(|parameter| {
                let index = parameter.index;
                quote!(#facade::invoke::BorrowOrigin::Parameter(#index))
            }),
    )
    .collect();
    let output = match (method.qualifiers.is_async, &method.return_type) {
        (false, ReturnTypeIr::Unit) => quote! {
            #receiver_binding
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            #call;
            #facade::invoke::InvocationOutput::Unit
        },
        (
            false,
            ReturnTypeIr::Type(TypeIr {
                kind:
                    TypeKindIr::Reference {
                        mutable: false,
                        element,
                        ..
                    },
                ..
            }),
        ) => {
            let value = if super::impls::is_str_type(element) {
                quote!(#facade::value::DynamicRef::<#mode>::new_str(#call))
            } else {
                quote!(#facade::value::DynamicRef::<#mode>::new(#call))
            };
            quote! {
                #receiver_binding
                let mut arguments = arguments.into_vec().into_iter();
                #(#argument_bindings)*
                #facade::invoke::InvocationOutput::Ref {
                    value: #value,
                    origins: ::std::boxed::Box::new([#(#borrow_origins),*]),
                }
            }
        }
        (
            false,
            ReturnTypeIr::Type(TypeIr {
                kind:
                    TypeKindIr::Reference {
                        mutable: true,
                        element,
                        ..
                    },
                ..
            }),
        ) => {
            let value = if super::impls::is_str_type(element) {
                quote!(#facade::value::DynamicMut::<#mode>::new_str_mut(#call))
            } else {
                quote!(#facade::value::DynamicMut::<#mode>::new(#call))
            };
            quote! {
                #receiver_binding
                let mut arguments = arguments.into_vec().into_iter();
                #(#argument_bindings)*
                #facade::invoke::InvocationOutput::Mut {
                    value: #value,
                    origin: #facade::invoke::BorrowOrigin::Receiver,
                }
            }
        }
        (
            false,
            ReturnTypeIr::Type(TypeIr {
                kind: TypeKindIr::Never,
                ..
            }),
        ) => quote! {
            #receiver_binding
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            match #call {}
        },
        (false, ReturnTypeIr::Type(_)) => quote! {
            #receiver_binding
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            #facade::invoke::InvocationOutput::Owned(
                #facade::value::DynamicOwned::<#mode>::new(#call),
            )
        },
        (true, ReturnTypeIr::Unit) => quote! {
            #receiver_binding
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            #facade::invoke::InvocationOutput::Future(
                #facade::invoke::ReflectedFuture::<#mode>::new(async move {
                    #call.await;
                    #facade::invoke::InvocationOutput::Unit
                }),
            )
        },
        (
            true,
            ReturnTypeIr::Type(TypeIr {
                kind: TypeKindIr::Never,
                ..
            }),
        ) => quote! {
            #receiver_binding
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            #facade::invoke::InvocationOutput::Future(
                #facade::invoke::ReflectedFuture::<#mode>::new(async move {
                    match #call.await {}
                }),
            )
        },
        (true, ReturnTypeIr::Type(_)) => quote! {
            #receiver_binding
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            #facade::invoke::InvocationOutput::Future(
                #facade::invoke::ReflectedFuture::<#mode>::new(async move {
                    #facade::invoke::InvocationOutput::Owned(
                        #facade::value::DynamicOwned::<#mode>::new(#call.await),
                    )
                }),
            )
        },
    };
    let catching_adapter_binding = if catching_requested {
        let catching_call = match &method.return_type {
            ReturnTypeIr::Unit => quote! {
                #call;
                #facade::invoke::InvocationOutput::Unit
            },
            ReturnTypeIr::Type(TypeIr {
                kind:
                    TypeKindIr::Reference {
                        mutable: false,
                        element,
                        ..
                    },
                ..
            }) => {
                let value = if super::impls::is_str_type(element) {
                    quote!(#facade::value::DynamicRef::<#mode>::new_str(#call))
                } else {
                    quote!(#facade::value::DynamicRef::<#mode>::new(#call))
                };
                quote! {
                    #facade::invoke::InvocationOutput::Ref {
                        value: #value,
                        origins: ::std::boxed::Box::new([#(#borrow_origins),*]),
                    }
                }
            }
            ReturnTypeIr::Type(TypeIr {
                kind:
                    TypeKindIr::Reference {
                        mutable: true,
                        element,
                        ..
                    },
                ..
            }) => {
                let value = if super::impls::is_str_type(element) {
                    quote!(#facade::value::DynamicMut::<#mode>::new_str_mut(#call))
                } else {
                    quote!(#facade::value::DynamicMut::<#mode>::new(#call))
                };
                quote! {
                    #facade::invoke::InvocationOutput::Mut {
                        value: #value,
                        origin: #facade::invoke::BorrowOrigin::Receiver,
                    }
                }
            }
            ReturnTypeIr::Type(TypeIr {
                kind: TypeKindIr::Never,
                ..
            }) => {
                quote!(match #call {})
            }
            ReturnTypeIr::Type(_) => quote! {
                #facade::invoke::InvocationOutput::Owned(
                    #facade::value::DynamicOwned::<#mode>::new(#call),
                )
            },
        };
        quote! {
            #[cfg(panic = "unwind")]
            let catching_adapter: #facade::invoke::CatchingInvocationAdapter<#mode> = |invocation| {
                #catching_assertions
                let identity = #facade::identity::MemberId::new(
                    #trait_name,
                    "default-method",
                    #index,
                    #facade::identity::FragmentIdentity::new(
                        env!("CARGO_PKG_NAME"), module_path!(), line!(), column!(),
                        "default-method", #index as u64,
                    ),
                );
                let validated = invocation.validate(
                    &identity,
                    #receiver_expectation,
                    &[#(#parameter_expectations),*],
                )?;
                #receiver_binding
                let mut arguments = arguments.into_vec().into_iter();
                #(#argument_bindings)*
                match ::std::panic::catch_unwind(|| { #catching_call }) {
                    Ok(output) => Ok(Ok(output)),
                    Err(payload) => Ok(Err(#facade::invoke::InvocationPanic::new(identity, payload))),
                }
            };
        }
    } else {
        TokenStream::new()
    };
    let invocation_result = if !method.qualifiers.is_async
        && matches!(
            method.return_type,
            ReturnTypeIr::Type(TypeIr {
                kind: TypeKindIr::Never,
                ..
            })
        ) {
        quote!({ #output })
    } else {
        quote!(Ok({ #output }))
    };
    let adapter_item = quote! {
        #[doc(hidden)]
        fn #adapter_name<'call>(
            invocation: #facade::invoke::Invocation<'call, #mode>,
        ) -> ::core::result::Result<
            #facade::invoke::InvocationOutput<'call, #mode>,
            #facade::invoke::InvocationFailure<'call, #mode>,
        >
        where
            Self: Sized + 'static,
            #(#hook_type_bounds,)*
        {
            #thread_safe_assertions
            let identity = #facade::identity::MemberId::new(
                #trait_name,
                "default-method",
                #index,
                #facade::identity::FragmentIdentity::new(
                    env!("CARGO_PKG_NAME"),
                    module_path!(),
                    line!(),
                    column!(),
                    "default-method",
                    #index as u64,
                ),
            );
            let validated = invocation.validate(
                &identity,
                #receiver_expectation,
                &[#(#parameter_expectations),*],
            )?;
            #invocation_result
        }
    };
    let adapter_constructor = if catching_requested && thread_safe {
        quote!(#facade::descriptor::InvocationAdapter::thread_safe_with_catching(
            Self::#adapter_name,
            catching_adapter,
        ))
    } else if catching_requested {
        quote!(#facade::descriptor::InvocationAdapter::local_with_catching(
            Self::#adapter_name,
            catching_adapter,
        ))
    } else if thread_safe {
        quote!(#facade::descriptor::InvocationAdapter::thread_safe(Self::#adapter_name))
    } else {
        quote!(#facade::descriptor::InvocationAdapter::local(Self::#adapter_name))
    };
    let unavailable_catching_constructor = if thread_safe {
        quote!(#facade::descriptor::InvocationAdapter::thread_safe_with_unavailable_catching(
            Self::#adapter_name,
        ))
    } else {
        quote!(#facade::descriptor::InvocationAdapter::local_with_unavailable_catching(
            Self::#adapter_name,
        ))
    };
    let adapter_entry = if catching_requested {
        quote! {{
            #catching_adapter_binding
            #[cfg(panic = "unwind")]
            let adapter = #adapter_constructor;
            #[cfg(panic = "abort")]
            let adapter = #unavailable_catching_constructor;
            Some(::std::boxed::Box::leak(::std::boxed::Box::new(adapter))
                as &'static #facade::descriptor::InvocationAdapter)
        }}
    } else {
        quote! {
            Some(::std::boxed::Box::leak(::std::boxed::Box::new(
                #adapter_constructor,
            )) as &'static #facade::descriptor::InvocationAdapter)
        }
    };
    Some((adapter_item, adapter_entry))
}

/// Generates a typed local adapter for a default `Pin<&Self>` or
/// `Pin<&mut Self>` receiver without erasing its pin proof.
fn default_pinned_method_invocation_adapter(
    method: &MethodIr,
    index: usize,
    suffix: &str,
    trait_name: &syn::LitStr,
    hook_type_bounds: &[TokenStream],
    facade: &TokenStream,
    pinned_mutable: bool,
) -> (TokenStream, TokenStream) {
    let adapter_name =
        format_ident!("__qubit_reflect_invoke_default_pinned_{suffix}_{index}");
    let method_name = &method.name;
    let mode = quote!(#facade::value::Local);
    let parameter_expectations: Vec<_> = method
        .parameters
        .iter()
        .map(|parameter| {
            super::impls::invocation_argument_expectation(parameter, facade)
        })
        .collect();
    let argument_bindings: Vec<_> = method
        .parameters
        .iter()
        .map(|parameter| {
            super::impls::invocation_argument_binding(parameter, facade, &mode)
        })
        .collect();
    let call_arguments: Vec<_> = method
        .parameters
        .iter()
        .map(|parameter| {
            format_ident!("__qubit_reflect_argument_{}", parameter.index)
        })
        .collect();
    let invocation_type = if pinned_mutable {
        quote!(#facade::invoke::PinnedMutInvocation<'call, Self, #mode>)
    } else {
        quote!(#facade::invoke::PinnedRefInvocation<'call, Self, #mode>)
    };
    let failure_type = if pinned_mutable {
        quote!(#facade::invoke::PinnedMutInvocationFailure<'call, Self, #mode>)
    } else {
        quote!(#facade::invoke::PinnedRefInvocationFailure<'call, Self, #mode>)
    };
    let adapter_type = if pinned_mutable {
        quote!(#facade::invoke::PinnedMutAdapter<Self, #mode>)
    } else {
        quote!(#facade::invoke::PinnedRefAdapter<Self, #mode>)
    };
    let call = quote!(Self::#method_name(receiver, #(#call_arguments),*));
    let output = match method.return_type {
        ReturnTypeIr::Unit => quote! {
            #call;
            #facade::invoke::InvocationOutput::Unit
        },
        ReturnTypeIr::Type(TypeIr {
            kind: TypeKindIr::Never,
            ..
        }) => quote!(match #call {}),
        ReturnTypeIr::Type(_) => quote! {
            #facade::invoke::InvocationOutput::Owned(
                #facade::value::DynamicOwned::<#mode>::new(#call),
            )
        },
    };
    let invocation_result = if matches!(
        method.return_type,
        ReturnTypeIr::Type(TypeIr {
            kind: TypeKindIr::Never,
            ..
        })
    ) {
        quote!(#output)
    } else {
        quote!(Ok(#output))
    };
    let adapter_item = quote! {
        #[doc(hidden)]
        fn #adapter_name<'call>(
            invocation: #invocation_type,
        ) -> ::core::result::Result<
            #facade::invoke::InvocationOutput<'call, #mode>,
            #failure_type,
        >
        where
            Self: Sized + 'static,
            #(#hook_type_bounds,)*
        {
            let identity = #facade::identity::MemberId::new(
                #trait_name,
                "default-method",
                #index,
                #facade::identity::FragmentIdentity::new(
                    env!("CARGO_PKG_NAME"), module_path!(), line!(), column!(),
                    "default-method", #index as u64,
                ),
            );
            let validated = invocation.validate(&identity, &[#(#parameter_expectations),*])?;
            let (receiver, arguments) = validated.into_parts();
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            #invocation_result
        }
    };
    let constructor = if pinned_mutable {
        quote!(#facade::descriptor::InvocationAdapter::pinned_mut_local(adapter))
    } else {
        quote!(#facade::descriptor::InvocationAdapter::pinned_ref_local(adapter))
    };
    let adapter_entry = quote! {{
        let adapter: #adapter_type = Self::#adapter_name;
        let adapter: &'static #adapter_type =
            ::std::boxed::Box::leak(::std::boxed::Box::new(adapter));
        Some(::std::boxed::Box::leak(::std::boxed::Box::new(#constructor))
            as &'static #facade::descriptor::InvocationAdapter)
    }};
    (adapter_item, adapter_entry)
}

/// Returns whether a dynamic signature contains an associated type whose
/// concrete `'static` bound cannot be proven at the trait declaration site.
fn type_contains_associated_type(ty: &TypeIr) -> bool {
    match &ty.kind {
        TypeKindIr::Path(path) => path_contains_associated_type(path),
        TypeKindIr::Reference { element, .. }
        | TypeKindIr::Slice(element)
        | TypeKindIr::Pointer { element, .. } => {
            type_contains_associated_type(element)
        }
        TypeKindIr::Tuple(elements) => {
            elements.iter().any(type_contains_associated_type)
        }
        TypeKindIr::Array { element, .. } => {
            type_contains_associated_type(element)
        }
        TypeKindIr::BareFunction { inputs, output, .. } => {
            inputs.iter().any(type_contains_associated_type)
                || output.as_deref().is_some_and(type_contains_associated_type)
        }
        TypeKindIr::TraitObject { bounds, .. }
        | TypeKindIr::ImplTrait { bounds } => {
            bounds.iter().any(bound_contains_associated_type)
        }
        TypeKindIr::Never => false,
        TypeKindIr::Infer | TypeKindIr::Macro | TypeKindIr::Other => true,
    }
}

/// Recursively checks associated bindings and nested arguments on one path.
fn path_contains_associated_type(path: &crate::ir::PathIr) -> bool {
    path.qualified_self.is_some()
        || (path.segments.len() > 1 && path.segments[0].name == "Self")
        || path
            .segments
            .iter()
            .any(|segment| match &segment.arguments {
                PathArgumentsIr::None => false,
                PathArgumentsIr::AngleBracketed(arguments) => {
                    arguments.iter().any(|argument| match argument {
                        PathArgumentIr::Type(ty)
                        | PathArgumentIr::AssociatedType { ty, .. } => {
                            type_contains_associated_type(ty)
                        }
                        PathArgumentIr::Constraint { bounds, .. } => {
                            bounds.iter().any(bound_contains_associated_type)
                        }
                        PathArgumentIr::Other(_) => true,
                        PathArgumentIr::Lifetime(_)
                        | PathArgumentIr::Const(_)
                        | PathArgumentIr::AssociatedConst { .. } => false,
                    })
                }
                PathArgumentsIr::Parenthesized { inputs, output } => {
                    inputs.iter().any(type_contains_associated_type)
                        || output
                            .as_deref()
                            .is_some_and(type_contains_associated_type)
                }
            })
}

/// Checks whether one trait or lifetime bound contains an associated binding.
fn bound_contains_associated_type(bound: &GenericBoundIr) -> bool {
    match bound {
        GenericBoundIr::Trait { path, .. } => {
            path_contains_associated_type(path)
        }
        GenericBoundIr::Lifetime(_) => false,
        GenericBoundIr::Other(_) => true,
    }
}

/// Returns the caller-visible path of the reflection facade.
fn facade_path() -> Option<TokenStream> {
    match proc_macro_crate::crate_name("qubit-reflect") {
        Ok(proc_macro_crate::FoundCrate::Itself) => Some(quote!(crate)),
        Ok(proc_macro_crate::FoundCrate::Name(name)) => {
            let identifier = Ident::new(&name, Span::call_site());
            Some(quote!(#identifier))
        }
        Err(_) => None,
    }
}

/// Computes a stable FNV-1a fingerprint for one normalized declaration.
fn fingerprint(input: &str) -> u64 {
    input.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

/// Converts generic declaration facts into the runtime generic descriptor
/// model.
pub(crate) fn generic_definition(
    generics: &GenericsIr,
    span: Span,
    facade: &TokenStream,
) -> TokenStream {
    let parameters = generics.params.iter().map(|parameter| {
        let name = syn::LitStr::new(&parameter.name, parameter.span);
        match parameter.kind {
            GenericKindIr::Lifetime => {
                let bounds = parameter.bounds.iter().filter_map(|bound| match bound {
                    GenericBoundIr::Lifetime(value) => Some(lifetime_expression(value, parameter.span, facade)),
                    _ => None,
                });
                quote!(#facade::expression::GenericParameterDescriptor::Lifetime {
                    name: #name.into(), bounds: Box::new([#(#bounds),*]),
                    diagnostic: #facade::expression::DiagnosticText::default(),
                })
            }
            GenericKindIr::Type => {
                let subject = syn::LitStr::new(&parameter.name, parameter.span);
                let bounds = generic_bounds(&parameter.bounds, &subject, parameter.span, facade);
                let default = match parameter.default.as_ref() {
                    Some(crate::ir::GenericDefaultIr::Type(value)) => {
                        let value = type_expression(value, facade);
                        quote!(Some(#value))
                    }
                    _ => quote!(None),
                };
                quote!(#facade::expression::GenericParameterDescriptor::Type {
                    name: #name.into(), bounds: Box::new([#(#bounds),*]), default: #default,
                    diagnostic: #facade::expression::DiagnosticText::default(),
                })
            }
            GenericKindIr::Const => {
                let ty = parameter.const_type.as_ref().map(|ty| type_expression(ty, facade)).unwrap_or_else(|| quote!(#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression { path: Box::new(["_".into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::default() })));
                let default = match parameter.default.as_ref() {
                    Some(crate::ir::GenericDefaultIr::Const(value)) => const_expression(value, facade),
                    _ => quote!(None),
                };
                quote!(#facade::expression::GenericParameterDescriptor::Const {
                    name: #name.into(), ty: Box::new(#ty), default: #default,
                    diagnostic: #facade::expression::DiagnosticText::default(),
                })
            }
        }
    });
    let predicates = generics.where_predicates.iter().flat_map(|predicate| match predicate {
        WherePredicateIr::Lifetime { lifetime, bounds, .. } => {
            let lifetime = lifetime_expression(lifetime, span, facade);
            let bounds = bounds.iter().map(|bound| lifetime_expression(bound, span, facade));
            vec![quote!(#facade::expression::PredicateDescriptor::LifetimeOutlives {
                lifetime: #lifetime, bounds: Box::new([#(#bounds),*]),
                diagnostic: #facade::expression::DiagnosticText::default(),
            })]
        }
        WherePredicateIr::Type { bounded_type, lifetimes, bounds, .. } => {
            let subject = type_expression(bounded_type, facade);
            let lifetimes = lifetimes.iter().map(|lifetime| lifetime_expression(lifetime, span, facade));
            let trait_bounds: Vec<_> = bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Trait { path, .. } => {
                    let path = syn::LitStr::new(&path.source, span);
                    Some(quote!(#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression { path: Box::new([#path.into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::from(#path) })))
                }
                _ => None,
            }).collect();
            let modifiers: Vec<_> = bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Trait { modifier, .. } => Some(match modifier {
                    crate::ir::TraitBoundModifierIr::None => quote!(#facade::expression::TraitBoundModifier::None),
                    crate::ir::TraitBoundModifierIr::Maybe => quote!(#facade::expression::TraitBoundModifier::Maybe),
                }),
                _ => None,
            }).collect();
            let type_bound = (!trait_bounds.is_empty()).then(|| quote!(#facade::expression::PredicateDescriptor::TypeBound {
                subject: #subject, bounds: Box::new([#(#trait_bounds),*]), bound_modifiers: Box::new([#(#modifiers),*]), higher_ranked_lifetimes: Box::new([#(#lifetimes),*]), diagnostic: #facade::expression::DiagnosticText::default(),
            }));
            let outlives = bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Lifetime(lifetime) => {
                    let lifetime = lifetime_expression(lifetime, span, facade);
                    let subject = type_expression(bounded_type, facade);
                    Some(quote!(#facade::expression::PredicateDescriptor::TypeOutlives { ty: #subject, lifetime: #lifetime, diagnostic: #facade::expression::DiagnosticText::default() }))
                }
                _ => None,
            });
            type_bound.into_iter().chain(outlives).collect()
        }
        WherePredicateIr::Other(_) => Vec::new(),
    });
    quote!(#facade::expression::GenericDefinitionDescriptor {
        parameters: Box::new([#(#parameters),*]), predicates: Box::new([#(#predicates),*]),
        diagnostic: #facade::expression::DiagnosticText::default(),
    })
}

/// Converts generic bounds into runtime predicate descriptors.
fn generic_bounds(
    bounds: &[GenericBoundIr],
    subject: &syn::LitStr,
    span: Span,
    facade: &TokenStream,
) -> Vec<TokenStream> {
    bounds.iter().filter_map(move |bound| match bound {
        GenericBoundIr::Trait { path, lifetimes, modifier } => {
            let path = syn::LitStr::new(&path.source, span);
            let lifetimes = lifetimes.iter().map(|lifetime| lifetime_expression(lifetime, span, facade));
            let modifier = match modifier {
                crate::ir::TraitBoundModifierIr::None => quote!(#facade::expression::TraitBoundModifier::None),
                crate::ir::TraitBoundModifierIr::Maybe => quote!(#facade::expression::TraitBoundModifier::Maybe),
            };
            Some(quote!(#facade::expression::PredicateDescriptor::TypeBound {
                subject: #facade::expression::TypeExpression::Parameter(#subject.into()),
                bounds: Box::new([#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression { path: Box::new([#path.into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::from(#path) })]),
                bound_modifiers: Box::new([#modifier]), higher_ranked_lifetimes: Box::new([#(#lifetimes),*]), diagnostic: #facade::expression::DiagnosticText::default(),
            }))
        }
        GenericBoundIr::Lifetime(lifetime) => {
            let lifetime = lifetime_expression(lifetime, span, facade);
            Some(quote!(#facade::expression::PredicateDescriptor::TypeOutlives {
                ty: #facade::expression::TypeExpression::Parameter(#subject.into()), lifetime: #lifetime,
                diagnostic: #facade::expression::DiagnosticText::default(),
            }))
        }
        GenericBoundIr::Other(_) => None,
    }).collect()
}

/// Converts source lifetime syntax into the runtime lifetime expression model.
fn lifetime_expression(
    lifetime: &str,
    span: Span,
    facade: &TokenStream,
) -> TokenStream {
    if lifetime == "'static" {
        return quote!(#facade::expression::LifetimeExpression::Static);
    }
    let lifetime = syn::LitStr::new(lifetime.trim_start_matches('\''), span);
    quote!(#facade::expression::LifetimeExpression::Named(#lifetime.into()))
}

/// Converts the type forms required by trait item descriptors into runtime
/// expressions.
pub(crate) fn type_expression(
    ty: &TypeIr,
    facade: &TokenStream,
) -> TokenStream {
    match &ty.kind {
        TypeKindIr::Never => quote!(#facade::expression::TypeExpression::Never),
        TypeKindIr::Path(path) => path_expression(path, ty, facade),
        TypeKindIr::Reference {
            lifetime,
            mutable,
            element,
        } => {
            let target = type_expression(element, facade);
            let lifetime = match lifetime.as_deref() {
                Some("'static") => {
                    quote!(#facade::expression::LifetimeExpression::Static)
                }
                Some(value) => {
                    let value = syn::LitStr::new(
                        value.trim_start_matches('\''),
                        ty.span,
                    );
                    quote!(#facade::expression::LifetimeExpression::Named(#value.into()))
                }
                None => quote!(#facade::expression::LifetimeExpression::Elided),
            };
            quote!(#facade::expression::TypeExpression::Reference(#facade::expression::ReferenceTypeExpression {
                lifetime: #lifetime, mutable: #mutable, target: Box::new(#target), diagnostic: #facade::expression::DiagnosticText::default(),
            }))
        }
        TypeKindIr::Tuple(elements) => {
            let elements = elements
                .iter()
                .map(|element| type_expression(element, facade));
            quote!(#facade::expression::TypeExpression::Tuple(Box::new([#(#elements),*])))
        }
        TypeKindIr::Slice(element) => {
            let element = type_expression(element, facade);
            quote!(#facade::expression::TypeExpression::Slice(Box::new(#element)))
        }
        TypeKindIr::Array { element, length } => {
            let element = type_expression(element, facade);
            let length = const_expression_value(length, facade);
            quote!(#facade::expression::TypeExpression::Array(#facade::expression::ArrayTypeExpression {
                element: Box::new(#element), length: #length, diagnostic: #facade::expression::DiagnosticText::default(),
            }))
        }
        TypeKindIr::Pointer { mutable, element } => {
            let target = type_expression(element, facade);
            quote!(#facade::expression::TypeExpression::RawPointer(#facade::expression::RawPointerTypeExpression {
                mutable: #mutable, target: Box::new(#target), diagnostic: #facade::expression::DiagnosticText::default(),
            }))
        }
        TypeKindIr::BareFunction {
            lifetimes,
            inputs,
            output,
            is_unsafe,
            abi,
            is_variadic,
        } => {
            let higher_ranked_lifetimes = lifetimes
                .iter()
                .map(|value| lifetime_expression(value, ty.span, facade));
            let parameters =
                inputs.iter().map(|value| type_expression(value, facade));
            let return_type = output
                .as_deref()
                .map(|value| type_expression(value, facade))
                .unwrap_or_else(
                    || quote!(#facade::expression::TypeExpression::Tuple(Box::new([]))),
                );
            let safety = if *is_unsafe {
                quote!(#facade::expression::FunctionSafety::Unsafe)
            } else {
                quote!(#facade::expression::FunctionSafety::Safe)
            };
            let abi = match abi.as_deref() {
                Some("C") => quote!(#facade::expression::FunctionAbi::C),
                Some("system") => {
                    quote!(#facade::expression::FunctionAbi::System)
                }
                Some(value) => {
                    let value = syn::LitStr::new(value, ty.span);
                    quote!(#facade::expression::FunctionAbi::Other(#value.into()))
                }
                None => quote!(#facade::expression::FunctionAbi::Rust),
            };
            quote!(#facade::expression::TypeExpression::FunctionPointer(#facade::expression::FunctionPointerExpression {
                abi: #abi, safety: #safety, variadic: #is_variadic, higher_ranked_lifetimes: Box::new([#(#higher_ranked_lifetimes),*]), parameters: Box::new([#(#parameters),*]), return_type: Box::new(#return_type), diagnostic: #facade::expression::DiagnosticText::default(),
            }))
        }
        TypeKindIr::TraitObject { bounds, .. } => {
            let bounds = bound_predicates(bounds, facade, ty.span);
            quote!(#facade::expression::TypeExpression::TraitObject(#facade::expression::TraitObjectExpression { bounds: Box::new([#(#bounds),*]), diagnostic: #facade::expression::DiagnosticText::default() }))
        }
        TypeKindIr::ImplTrait { bounds } => {
            let bounds = bound_predicates(bounds, facade, ty.span);
            quote!(#facade::expression::TypeExpression::Opaque(#facade::expression::OpaqueTypeExpression { bounds: Box::new([#(#bounds),*]), diagnostic: #facade::expression::DiagnosticText::default() }))
        }
        _ => {
            let source = syn::LitStr::new(&ty.source, ty.span);
            quote!(#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression {
                path: Box::new([#source.into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::from(#source),
            }))
        }
    }
}

/// Converts one parsed path into a runtime type-expression token stream.
fn path_expression(
    path: &crate::ir::PathIr,
    ty: &TypeIr,
    facade: &TokenStream,
) -> TokenStream {
    let diagnostic = syn::LitStr::new(&ty.source, ty.span);
    if path.qualified_self.is_none()
        && path.segments.len() == 1
        && path.segments[0].name == "Self"
    {
        return quote!(#facade::expression::TypeExpression::SelfType);
    }
    if path.qualified_self.is_none()
        && path.segments.len() == 1
        && matches!(path.segments[0].arguments, PathArgumentsIr::None)
        && path.segments[0]
            .name
            .chars()
            .all(|value| value.is_ascii_uppercase())
    {
        let name = syn::LitStr::new(&path.segments[0].name, ty.span);
        return quote!(#facade::expression::TypeExpression::Parameter(#name.into()));
    }
    if let Some(qualified) = &path.qualified_self {
        let self_type = type_expression(&qualified.ty, facade);
        let item = path
            .segments
            .last()
            .map(|segment| syn::LitStr::new(&segment.name, ty.span))
            .expect("qualified path has an item");
        let arguments = path
            .segments
            .last()
            .map(|segment| path_arguments(&segment.arguments, facade, ty.span))
            .unwrap_or_default();
        let trait_segments = path
            .segments
            .iter()
            .take(qualified.position)
            .map(|segment| syn::LitStr::new(&segment.name, ty.span));
        let trait_path = if qualified.has_as {
            quote!(Some(Box::new(#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression { path: Box::new([#(#trait_segments.into()),*]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::default() }))))
        } else {
            quote!(None)
        };
        return quote!(#facade::expression::TypeExpression::Associated(#facade::expression::AssociatedTypeExpression { self_type: Box::new(#self_type), trait_path: #trait_path, item: #item.into(), arguments: Box::new([#(#arguments),*]), diagnostic: #facade::expression::DiagnosticText::from(#diagnostic) }));
    }
    let segments = path
        .segments
        .iter()
        .map(|segment| syn::LitStr::new(&segment.name, ty.span));
    let arguments = path
        .segments
        .last()
        .map(|segment| path_arguments(&segment.arguments, facade, ty.span))
        .unwrap_or_default();
    quote!(#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression { path: Box::new([#(#segments.into()),*]), arguments: Box::new([#(#arguments),*]), diagnostic: #facade::expression::DiagnosticText::from(#diagnostic) }))
}

/// Converts parsed path arguments into runtime generic-argument expressions.
fn path_arguments(
    arguments: &PathArgumentsIr,
    facade: &TokenStream,
    span: Span,
) -> Vec<TokenStream> {
    match arguments {
        PathArgumentsIr::None => Vec::new(),
        PathArgumentsIr::Parenthesized { inputs, output } => {
            let inputs = inputs.iter().map(|value| type_expression(value, facade));
            let output = output.as_deref().map(|value| type_expression(value, facade)).unwrap_or_else(|| quote!(#facade::expression::TypeExpression::Tuple(Box::new([]))));
            vec![quote!(#facade::expression::GenericArgument::Type(#facade::expression::TypeExpression::FunctionPointer(#facade::expression::FunctionPointerExpression { abi: #facade::expression::FunctionAbi::Rust, safety: #facade::expression::FunctionSafety::Safe, variadic: false, higher_ranked_lifetimes: Box::new([]), parameters: Box::new([#(#inputs),*]), return_type: Box::new(#output), diagnostic: #facade::expression::DiagnosticText::default() })))]
        }
        PathArgumentsIr::AngleBracketed(values) => values.iter().filter_map(|value| match value {
            PathArgumentIr::Lifetime(value) => { let value = lifetime_expression(value, span, facade); Some(quote!(#facade::expression::GenericArgument::Lifetime(#value))) }
            PathArgumentIr::Type(value) => { let value = type_expression(value, facade); Some(quote!(#facade::expression::GenericArgument::Type(#value))) }
            PathArgumentIr::Const(value) => { let value = const_expression_value(value, facade); let source = syn::LitStr::new(&value.to_string(), span); Some(quote!(#facade::expression::GenericArgument::Const(#facade::expression::ConstGenericArgument { declared_type: Box::new(#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression { path: Box::new(["_".into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::default() })), value: #value, normalized_diagnostic: #source.into() }))) }
            PathArgumentIr::AssociatedType { name, ty } => { let name = syn::LitStr::new(name, span); let value = type_expression(ty, facade); Some(quote!(#facade::expression::GenericArgument::AssociatedType { name: #name.into(), value: Box::new(#value) })) }
            _ => None,
        }).collect(),
    }
}

/// Materializes direct external-supertrait arguments at the trait hook's
/// concrete instance.
fn external_supertrait_arguments(
    arguments: &PathArgumentsIr,
    declaration: &TraitDeclarationIr,
    facade: &TokenStream,
) -> Vec<TokenStream> {
    let PathArgumentsIr::AngleBracketed(values) = arguments else {
        return path_arguments(arguments, facade, declaration.span);
    };
    values
        .iter()
        .filter_map(|value| match value {
            PathArgumentIr::Type(
                value @ TypeIr {
                    kind: TypeKindIr::Path(path),
                    ..
                },
            ) if path.segments.len() == 1 => {
                let name = &path.segments[0].name;
                if let Some(parameter) = declaration.generics.params.iter().find(|parameter| {
                    parameter.kind == GenericKindIr::Type && parameter.name == *name
                }) {
                    let identifier = Ident::new(&parameter.name, parameter.span);
                    Some(quote!(#facade::expression::GenericArgument::Type(
                        #facade::expression::TypeExpression::Concrete(
                            #facade::expression::ConcreteTypeExpression {
                                path: Box::new([std::any::type_name::<#identifier>().into()]),
                                arguments: Box::new([]),
                                diagnostic: #facade::expression::DiagnosticText::from(
                                    std::any::type_name::<#identifier>(),
                                ),
                            },
                        ),
                    )))
                } else {
                    path_arguments(
                        &PathArgumentsIr::AngleBracketed(vec![PathArgumentIr::Type(value.clone())]),
                        facade,
                        declaration.span,
                    )
                    .into_iter()
                    .next()
                }
            }
            value => path_arguments(
                &PathArgumentsIr::AngleBracketed(vec![value.clone()]),
                facade,
                declaration.span,
            )
            .into_iter()
            .next(),
        })
        .collect()
}

/// Converts trait-object or opaque-type bounds into runtime predicates.
fn bound_predicates(
    bounds: &[GenericBoundIr],
    facade: &TokenStream,
    span: Span,
) -> Vec<TokenStream> {
    bounds.iter().filter_map(|bound| match bound { GenericBoundIr::Trait { path, modifier, lifetimes } => { let source = syn::LitStr::new(&path.source, span); let modifier = match modifier { crate::ir::TraitBoundModifierIr::None => quote!(#facade::expression::TraitBoundModifier::None), crate::ir::TraitBoundModifierIr::Maybe => quote!(#facade::expression::TraitBoundModifier::Maybe) }; let lifetimes = lifetimes.iter().map(|value| lifetime_expression(value, span, facade)); Some(quote!(#facade::expression::PredicateDescriptor::TypeBound { subject: #facade::expression::TypeExpression::SelfType, bounds: Box::new([#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression { path: Box::new([#source.into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::from(#source) })]), bound_modifiers: Box::new([#modifier]), higher_ranked_lifetimes: Box::new([#(#lifetimes),*]), diagnostic: #facade::expression::DiagnosticText::default() })) }, GenericBoundIr::Lifetime(value) => { let value = lifetime_expression(value, span, facade); Some(quote!(#facade::expression::PredicateDescriptor::TypeOutlives { ty: #facade::expression::TypeExpression::SelfType, lifetime: #value, diagnostic: #facade::expression::DiagnosticText::default() })) }, _ => None }).collect()
}

/// Converts a parsed const expression into structural runtime metadata.
fn const_expression_value(
    value: &TokenStream,
    facade: &TokenStream,
) -> TokenStream {
    let source = value.to_string();
    if let Ok(identifier) = syn::parse2::<syn::Ident>(value.clone()) {
        let name = syn::LitStr::new(&identifier.to_string(), identifier.span());
        return quote!(#facade::expression::ConstExpression::Parameter(#name.into()));
    }
    if let Ok(value) = syn::parse2::<syn::Lit>(value.clone()) {
        match value {
            syn::Lit::Bool(value) => {
                let value = value.value;
                return quote!(#facade::expression::ConstExpression::Boolean(#value));
            }
            syn::Lit::Char(value) => {
                let value = value.value();
                return quote!(#facade::expression::ConstExpression::Character(#value));
            }
            syn::Lit::Int(value) => {
                if let Ok(value) = value.base10_parse::<u128>() {
                    return quote!(#facade::expression::ConstExpression::UnsignedInteger(#value));
                }
            }
            _ => {}
        }
    }
    if let Ok(syn::Expr::Unary(value)) = syn::parse2::<syn::Expr>(value.clone())
        && matches!(value.op, syn::UnOp::Neg(_))
        && let syn::Expr::Lit(literal) = *value.expr
        && let syn::Lit::Int(value) = literal.lit
        && let Ok(value) = value.base10_parse::<i128>()
    {
        let value = -value;
        return quote!(#facade::expression::ConstExpression::SignedInteger(#value));
    }
    let source = syn::LitStr::new(&source, Span::call_site());
    quote!(compile_error!(
        concat!("unsupported const expression in #[reflect] trait: ", #source)
    ))
}

/// Converts the literal const-default subset that has a structural runtime
/// representation.
fn const_expression(value: &TokenStream, facade: &TokenStream) -> TokenStream {
    let expression = syn::parse2::<syn::Expr>(value.clone());
    let expression = match expression {
        Ok(expression) => expression,
        Err(_) => return unsupported_const_default(value),
    };
    match expression {
        syn::Expr::Lit(expression) => match expression.lit {
            syn::Lit::Bool(value) => {
                let value = value.value;
                quote!(Some(#facade::expression::ConstExpression::Boolean(#value)))
            }
            syn::Lit::Char(value) => {
                let value = value.value();
                quote!(Some(#facade::expression::ConstExpression::Character(#value)))
            }
            syn::Lit::Int(value) => {
                integer_const_expression(&value, false, facade).unwrap_or_else(
                    || unsupported_const_default(value.to_token_stream()),
                )
            }
            _ => unsupported_const_default(value),
        },
        syn::Expr::Unary(expression)
            if matches!(expression.op, syn::UnOp::Neg(_)) =>
        {
            match expression.expr.as_ref() {
                syn::Expr::Lit(expression) => match &expression.lit {
                    syn::Lit::Int(value) => {
                        integer_const_expression(value, true, facade)
                            .unwrap_or_else(|| {
                                unsupported_const_default(
                                    value.to_token_stream(),
                                )
                            })
                    }
                    _ => unsupported_const_default(value),
                },
                _ => unsupported_const_default(value),
            }
        }
        _ => unsupported_const_default(value),
    }
}

/// Converts an integer literal without relying on its whitespace-normalized
/// token rendering.
fn integer_const_expression(
    value: &syn::LitInt,
    negative: bool,
    facade: &TokenStream,
) -> Option<TokenStream> {
    let suffix = value.suffix();
    let signed = negative
        || matches!(suffix, "i8" | "i16" | "i32" | "i64" | "i128" | "isize");
    if signed {
        let magnitude = value.base10_parse::<i128>().ok()?;
        let value = if negative {
            magnitude.checked_neg()?
        } else {
            magnitude
        };
        Some(
            quote!(Some(#facade::expression::ConstExpression::SignedInteger(#value))),
        )
    } else {
        let value = value.base10_parse::<u128>().ok()?;
        Some(
            quote!(Some(#facade::expression::ConstExpression::UnsignedInteger(#value))),
        )
    }
}

/// Emits a deterministic compile error for const defaults without a runtime
/// structural value.
fn unsupported_const_default(value: impl quote::ToTokens) -> TokenStream {
    let source = syn::LitStr::new(
        &value.into_token_stream().to_string(),
        Span::call_site(),
    );
    quote!(compile_error!(
        concat!("unsupported non-literal const default in #[reflect] trait: ", #source)
    ))
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;

    use super::const_expression;

    /// Verifies that literals retain their structural value rather than their
    /// token spelling.
    #[test]
    fn test_const_default_accepts_signed_suffixed_and_escaped_literals() {
        let facade = quote!(qubit_reflect);
        let signed = const_expression(&quote!(-7i16), &facade).to_string();
        let unsigned = const_expression(&quote!(42u8), &facade).to_string();
        let escaped = const_expression(&quote!('\n'), &facade).to_string();

        assert!(signed.contains("SignedInteger"));
        assert!(signed.contains("- 7i128"));
        assert!(unsigned.contains("UnsignedInteger"));
        assert!(unsigned.contains("42u128"));
        assert!(escaped.contains("Character"));
        assert!(escaped.contains("'\\n'"));
    }

    /// Verifies that symbolic defaults fail explicitly instead of becoming
    /// symbolic values.
    #[test]
    fn test_const_default_rejects_non_literal_expression() {
        let value: TokenStream = quote!(DEFAULT_LIMIT);
        let rendered =
            const_expression(&value, &quote!(qubit_reflect)).to_string();

        assert!(rendered.contains("unsupported non-literal const default"));
        assert!(rendered.contains("DEFAULT_LIMIT"));
    }
}
