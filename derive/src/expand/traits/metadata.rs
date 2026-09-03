// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Descriptor metadata emitted for reflected trait declarations.

use proc_macro2::TokenStream;
use quote::quote;
use syn::LitStr;

use super::generic_definition;
use super::trait_metadata::TraitMetadata;
use super::type_expression;
use crate::expand::expression_codegen::const_expression;
use crate::expand::expression_codegen::function_abi;
use crate::expand::expression_codegen::lifetime_expression;
use crate::expand::expression_codegen::path_type_expression;
use crate::expand::generic_environment::GenericEnvironment;
use crate::ir::GenericBoundIr;
use crate::ir::GenericKindIr;
use crate::ir::ParameterPatternKindIr;
use crate::ir::ReceiverKindIr;
use crate::ir::ReturnTypeIr;
use crate::ir::TraitDeclarationIr;
use crate::ir::TypeKindIr;

/// Emits structural method, associated-item, and generic metadata.
pub(super) fn build(
    declaration: &TraitDeclarationIr,
    trait_name: &LitStr,
    facade: &TokenStream,
) -> TraitMetadata {
    let trait_name_literal = trait_name;
    let environment = GenericEnvironment::from_generics(&declaration.generics);
    let methods: Vec<_> = declaration
    .methods
    .iter()
    .enumerate()
    .map(|(method_index, method)| {
        let method_environment = environment.clone().with_generics(&method.generics);
        let name = LitStr::new(&method.name.to_string(), method.span);
        let query = method
            .attributes
            .iter()
            .find_map(|attribute| attribute.rename())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| method.name.to_string());
        let query = LitStr::new(&query, method.span);
        let index = method_index;
        let receiver = match &method.receiver {
            Some(receiver) => match receiver.kind {
                ReceiverKindIr::Value => quote!(Some(#facade::__private::codegen_v2::descriptor::ReceiverDescriptor::Owned)),
                ReceiverKindIr::SharedReference => quote!(Some(#facade::__private::codegen_v2::descriptor::ReceiverDescriptor::Shared)),
                ReceiverKindIr::MutableReference => quote!(Some(#facade::__private::codegen_v2::descriptor::ReceiverDescriptor::Mutable)),
                ReceiverKindIr::Typed => {
                    let declaration = LitStr::new(&receiver.declaration.to_string(), receiver.span);
                    quote!(Some(#facade::__private::codegen_v2::descriptor::ReceiverDescriptor::Explicit(#declaration)))
                }
            },
            None => quote!(None),
        };
        let parameters = method.parameters.iter().map(|parameter| {
            let name = parameter.name.as_deref().map(|name| LitStr::new(name, parameter.span));
            let name = match name {
                Some(name) => quote!(Some(#name)),
                None => quote!(None),
            };
            let pattern = match parameter.pattern.kind {
                ParameterPatternKindIr::Identifier => {
                    quote!(#facade::__private::codegen_v2::descriptor::ParameterPatternDescriptor::Identifier)
                }
                ParameterPatternKindIr::Wildcard => {
                    quote!(#facade::__private::codegen_v2::descriptor::ParameterPatternDescriptor::Wildcard)
                }
                ParameterPatternKindIr::Destructure => {
                    let source = LitStr::new(&parameter.pattern.source, parameter.span);
                    quote!(#facade::__private::codegen_v2::descriptor::ParameterPatternDescriptor::Destructure(#source.into()))
                }
            };
            let passing = match &parameter.ty.kind {
                TypeKindIr::Reference { mutable: true, .. } => {
                    quote!(#facade::__private::codegen_v2::descriptor::ParameterPassingMode::MutableBorrow)
                }
                TypeKindIr::Reference { .. } => quote!(#facade::__private::codegen_v2::descriptor::ParameterPassingMode::SharedBorrow),
                _ => quote!(#facade::__private::codegen_v2::descriptor::ParameterPassingMode::Owned),
            };
            let ty = type_expression(&parameter.ty, &method_environment, facade);
            let index = parameter.index;
            quote!(#facade::__private::codegen_v2::descriptor::ParameterDescriptor::new(#index, #name, #pattern, #passing, #ty, None))
        });
        let return_value = match &method.return_type {
            ReturnTypeIr::Unit => quote!(#facade::__private::codegen_v2::descriptor::ReturnDescriptor::unit()),
            ReturnTypeIr::Type(ty) => {
                let expression = type_expression(ty, &method_environment, facade);
                let kind = match ty.kind {
                    TypeKindIr::Never => quote!(#facade::__private::codegen_v2::descriptor::ReturnKind::Never),
                    TypeKindIr::Reference { .. } => quote!(#facade::__private::codegen_v2::descriptor::ReturnKind::Reference),
                    TypeKindIr::ImplTrait { .. } => quote!(#facade::__private::codegen_v2::descriptor::ReturnKind::Opaque),
                    _ => quote!(#facade::__private::codegen_v2::descriptor::ReturnKind::Concrete),
                };
                quote!(#facade::__private::codegen_v2::descriptor::ReturnDescriptor::new(#kind, Some(#expression), None))
            }
        };
        let qualifiers = &method.qualifiers;
        let method_generic_definition = generic_definition(&method.generics, method.span, facade);
        let is_async = qualifiers.is_async;
        let is_unsafe = qualifiers.is_unsafe;
        let is_const = qualifiers.is_const;
        let is_variadic = qualifiers.is_variadic;
        let has_default = method.has_default;
        let abi = qualifiers
            .abi
            .as_deref()
            .map(|abi| function_abi(Some(abi), method.span, facade));
        let abi = match abi {
            Some(abi) => quote!(Some(#abi)),
            None => quote!(None),
        };
        quote! {
            #facade::__private::codegen_v2::descriptor::MethodDescriptor::builder(
                #facade::__private::codegen_v2::identity::MemberId::new(
                    #trait_name_literal,
                    "method",
                    #index,
                    #facade::__private::codegen_v2::identity::FragmentIdentity::new(
                        env!("CARGO_PKG_NAME"), module_path!(), line!(), column!(), "method", #index as u64,
                    ),
                ),
                #name,
                #query,
                #facade::__private::codegen_v2::descriptor::MethodDeclarationOwner::Trait(definition),
            )
            .visibility(#facade::__private::codegen_v2::descriptor::MethodVisibility::InheritedFromTrait)
            .receiver(#receiver)
            .parameters(vec![#(#parameters),*])
            .return_value(#return_value)
            .qualifiers(#facade::__private::codegen_v2::descriptor::MethodQualifiers::new(
                #is_async, #is_unsafe, #is_const, #abi, #is_variadic,
            ))
            .generic_definition(&#method_generic_definition)
            .has_default(#has_default)
            .build()
        }
    })
    .collect();
    let associated_types: Vec<_> = declaration.associated_types.iter().enumerate().map(|(index, item)| {
    let name = LitStr::new(&item.name.to_string(), item.span);
    let item_environment = environment.clone().with_generics(&item.generics);
    let bounds = item.bounds.iter().filter_map(|bound| match bound {
        GenericBoundIr::Trait { path, lifetimes, modifier } => {
            let path = path_type_expression(path, &item_environment, facade);
            let lifetimes = lifetimes
                .iter()
                .map(|lifetime| lifetime_expression(lifetime, item.span, facade));
            let modifier = match modifier { crate::ir::TraitBoundModifierIr::None => quote!(#facade::__private::codegen_v2::expression::TraitBoundModifier::None), crate::ir::TraitBoundModifierIr::Maybe => quote!(#facade::__private::codegen_v2::expression::TraitBoundModifier::Maybe) };
            Some(quote!(#facade::__private::codegen_v2::expression::type_bound(
                #facade::__private::codegen_v2::expression::parameter(#name),
                Box::new([#path]),
                Box::new([#modifier]),
                Box::new([#(#lifetimes),*]),
            )))
        }
        GenericBoundIr::Lifetime(lifetime) => {
            let lifetime = lifetime_expression(lifetime, item.span, facade);
            Some(quote!(#facade::__private::codegen_v2::expression::PredicateDescriptor::TypeOutlives {
                ty: #facade::__private::codegen_v2::expression::parameter(#name),
                lifetime: #lifetime,
                diagnostic: #facade::__private::codegen_v2::expression::DiagnosticText::default(),
            }))
        }
        _ => None,
    });
    let default = item
        .value
        .as_ref()
        .map(|value| type_expression(value, &item_environment, facade));
    let default = match default { Some(value) => quote!(Some(#value)), None => quote!(None) };
    let generic_definition = generic_definition(&item.generics, item.span, facade);
    quote!(#facade::__private::codegen_v2::descriptor::AssociatedTypeDescriptor::new_with_generic_definition(
        #index,
        #name,
        #name,
        Box::new([#(#bounds),*]),
        #default,
        #generic_definition,
    ))
}).collect();
    let associated_consts: Vec<_> = declaration
    .associated_consts
    .iter()
    .enumerate()
    .map(|(index, item)| {
        let name = LitStr::new(&item.name.to_string(), item.span);
        let ty = type_expression(&item.ty, &environment, facade);
        let has_default = item.value.is_some();
        quote!(#facade::__private::codegen_v2::descriptor::AssociatedConstDescriptor::new(#index, #name, #name, #ty, #has_default))
    })
    .collect();
    let parameters = declaration.generics.params.iter().map(|parameter| {
    let name = LitStr::new(&parameter.name, declaration.span);
    match parameter.kind {
        GenericKindIr::Lifetime => {
            let bounds = parameter.bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Lifetime(lifetime) => Some(lifetime_expression(
                    lifetime,
                    declaration.span,
                    facade,
                )),
                _ => None,
            });
            quote!(__qubit_reflect_codegen::expression::lifetime_parameter(
                #name,
                Box::new([#(#bounds),*]),
                __qubit_reflect_codegen::expression::DiagnosticText::default(),
            ))
        }
        GenericKindIr::Type => {
            let subject = LitStr::new(&parameter.name, declaration.span);
            let bounds = parameter.bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Trait { path, lifetimes, modifier } => {
                    let path = path_type_expression(path, &environment, facade);
                    let lifetimes = lifetimes.iter().map(|lifetime| {
                        lifetime_expression(lifetime, declaration.span, facade)
                    });
                    let modifier = match modifier {
                        crate::ir::TraitBoundModifierIr::None => quote!(#facade::__private::codegen_v2::expression::TraitBoundModifier::None),
                        crate::ir::TraitBoundModifierIr::Maybe => quote!(#facade::__private::codegen_v2::expression::TraitBoundModifier::Maybe),
                    };
                    Some(quote!(#facade::__private::codegen_v2::expression::type_bound(
                        #facade::__private::codegen_v2::expression::parameter(#subject),
                        Box::new([#path]),
                        Box::new([#modifier]),
                        Box::new([#(#lifetimes),*]),
                    )))
                }
                GenericBoundIr::Lifetime(lifetime) => {
                    let lifetime = lifetime_expression(lifetime, declaration.span, facade);
                    Some(quote!(#facade::__private::codegen_v2::expression::PredicateDescriptor::TypeOutlives {
                        ty: #facade::__private::codegen_v2::expression::parameter(#subject),
                        lifetime: #lifetime,
                        diagnostic: #facade::__private::codegen_v2::expression::DiagnosticText::default(),
                    }))
                }
                _ => None,
            });
            let default = match parameter.default.as_ref() {
                Some(crate::ir::GenericDefaultIr::Type(value)) => {
                    let value = type_expression(value, &environment, facade);
                    quote!(Some(#value))
                }
                _ => quote!(None),
            };
            quote!(__qubit_reflect_codegen::expression::type_parameter(
                #name,
                Box::new([#(#bounds),*]),
                #default,
                __qubit_reflect_codegen::expression::DiagnosticText::default(),
            ))
        }
        GenericKindIr::Const => {
            let const_type = parameter
                .const_type
                .as_ref()
                .map(|value| value.source.as_str())
                .unwrap_or("_");
            let const_type = LitStr::new(const_type, declaration.span);
            let default = match parameter.default.as_ref() {
                Some(crate::ir::GenericDefaultIr::Const(value)) => {
                    const_expression(value, &environment, facade)
                }
                _ => quote!(None),
            };
            quote! {
                __qubit_reflect_codegen::expression::const_generic_parameter(
                    #name,
                    __qubit_reflect_codegen::expression::TypeExpression::Concrete(
                        __qubit_reflect_codegen::expression::concrete(
                            vec![#const_type.into()].into_boxed_slice(),
                            vec![].into_boxed_slice(),
                            __qubit_reflect_codegen::expression::DiagnosticText::from(#const_type),
                        ),
                    ),
                    #default,
                    __qubit_reflect_codegen::expression::DiagnosticText::default(),
                )
            }
        }
    }
});
    let where_predicates = declaration.generics.where_predicates.iter().flat_map(|predicate| match predicate {
    crate::ir::WherePredicateIr::Lifetime { lifetime, bounds, .. } => {
        let lifetime = lifetime_expression(lifetime, declaration.span, facade);
        let bounds: Vec<_> = bounds.iter().map(|bound| {
            lifetime_expression(bound, declaration.span, facade)
        }).collect();
        vec![quote!(#facade::__private::codegen_v2::expression::lifetime_outlives(
            #lifetime,
            Box::new([#(#bounds),*]),
        ))]
    }
    crate::ir::WherePredicateIr::Type {
        bounded_type,
        lifetimes,
        bounds,
        ..
    } => {
        let subject = type_expression(bounded_type, &environment, facade);
        let higher_ranked_lifetimes: Vec<_> = lifetimes.iter().map(|lifetime| {
            lifetime_expression(lifetime, declaration.span, facade)
        }).collect();
        let trait_bounds: Vec<_> = bounds.iter().filter_map(|bound| match bound {
            GenericBoundIr::Trait { path, .. } => {
                Some(path_type_expression(path, &environment, facade))
            }
            _ => None,
        }).collect();
        let bound_modifiers: Vec<_> = bounds.iter().filter_map(|bound| match bound {
            GenericBoundIr::Trait { modifier, .. } => Some(match modifier {
                crate::ir::TraitBoundModifierIr::None => {
                    quote!(#facade::__private::codegen_v2::expression::TraitBoundModifier::None)
                }
                crate::ir::TraitBoundModifierIr::Maybe => {
                    quote!(#facade::__private::codegen_v2::expression::TraitBoundModifier::Maybe)
                }
            }),
            _ => None,
        }).collect();
        let type_bound = (!trait_bounds.is_empty()).then(|| {
            quote!(#facade::__private::codegen_v2::expression::type_bound(
                #subject,
                Box::new([#(#trait_bounds),*]),
                Box::new([#(#bound_modifiers),*]),
                Box::new([#(#higher_ranked_lifetimes),*]),
            ))
        });
        let lifetime_bounds = bounds.iter().filter_map(|bound| match bound {
            GenericBoundIr::Lifetime(lifetime) => {
                let lifetime = lifetime_expression(lifetime, declaration.span, facade);
                let subject = type_expression(bounded_type, &environment, facade);
                Some(quote!(#facade::__private::codegen_v2::expression::PredicateDescriptor::TypeOutlives {
                    ty: #subject,
                    lifetime: #lifetime,
                    diagnostic: #facade::__private::codegen_v2::expression::DiagnosticText::default(),
                }))
            }
            _ => None,
        });
        type_bound.into_iter().chain(lifetime_bounds).collect()
    }
    _ => Vec::new(),
});
    TraitMetadata {
        methods,
        associated_types,
        associated_consts,
        parameters: parameters.collect(),
        where_predicates: where_predicates.collect(),
    }
}
