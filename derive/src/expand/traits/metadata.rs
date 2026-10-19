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
///
/// # Parameters
///
/// - `declaration`: Validated trait declaration and its member metadata.
/// - `trait_name`: Diagnostic name used to identify generated members.
/// - `facade`: Runtime facade path used by generated references.
///
/// # Returns
///
/// Returns token groups for methods, associated items, generic parameters and
/// predicates.
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
        .map(|method| {
            let method_environment = environment.clone().with_generics(&method.generics);
            let name = LitStr::new(&method.name.to_string(), method.span);
            let query = method
                .attributes
                .iter()
                .find_map(|attribute| attribute.rename())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| method.name.to_string());
            let query = LitStr::new(&query, method.span);
            let receiver = match &method.receiver {
                Some(receiver) => match receiver.kind {
                    ReceiverKindIr::Value => {
                        quote!(Some(#facade::__private::codegen_v4::invoke::ReceiverDescriptor::trait_self(
                            #facade::__private::codegen_v4::invoke::ReceiverKind::Owned
                            , &#facade::__private::codegen_v4::expression::ExpressionData::SelfType
                        )))
                    }
                    ReceiverKindIr::SharedReference => {
                        quote!(Some(#facade::__private::codegen_v4::invoke::ReceiverDescriptor::trait_self(
                            #facade::__private::codegen_v4::invoke::ReceiverKind::Shared
                            , &#facade::__private::codegen_v4::expression::ExpressionData::SelfType
                        )))
                    }
                    ReceiverKindIr::MutableReference => {
                        quote!(Some(#facade::__private::codegen_v4::invoke::ReceiverDescriptor::trait_self(
                            #facade::__private::codegen_v4::invoke::ReceiverKind::Mutable
                            , &#facade::__private::codegen_v4::expression::ExpressionData::SelfType
                        )))
                    }
                    ReceiverKindIr::Typed => {
                        let receiver_expression = type_expression(&receiver.ty, &method_environment, facade);
                        let kind = if crate::expand::invocation::analysis::typed_pinned_receiver_mutable(receiver)
                            == Some(true)
                        {
                            quote!(#facade::__private::codegen_v4::invoke::ReceiverKind::PinnedMutable)
                        } else if crate::expand::invocation::analysis::typed_pinned_receiver_mutable(receiver)
                            == Some(false)
                        {
                            quote!(#facade::__private::codegen_v4::invoke::ReceiverKind::PinnedShared)
                        } else {
                            quote!(#facade::__private::codegen_v4::invoke::ReceiverKind::Owned)
                        };
                        quote!(Some(
                            #facade::__private::codegen_v4::invoke::ReceiverDescriptor::trait_self(#kind, #receiver_expression)
                        ))
                    }
                },
                None => quote!(None),
            };
            let parameters = method.parameters.iter().map(|parameter| {
                let pattern = match parameter.pattern.kind {
                    ParameterPatternKindIr::Identifier => {
                        let name = parameter.name.as_deref().unwrap_or("");
                        let name = LitStr::new(name, parameter.span);
                        quote!(#facade::__private::codegen_v4::invoke::ParameterPattern::Identifier(#name))
                    }
                    ParameterPatternKindIr::Wildcard => {
                        quote!(#facade::__private::codegen_v4::invoke::ParameterPattern::Wildcard)
                    }
                    ParameterPatternKindIr::Destructure => {
                        let pattern = LitStr::new(&parameter.pattern.source, parameter.span);
                        quote!(#facade::__private::codegen_v4::invoke::ParameterPattern::DestructurePattern(#pattern))
                    }
                };
                let passing = match &parameter.ty.kind {
                    TypeKindIr::Reference { mutable: true, .. } => {
                        quote!(#facade::__private::codegen_v4::invoke::PassingMode::Mutable)
                    }
                    TypeKindIr::Reference { .. } => {
                        quote!(#facade::__private::codegen_v4::invoke::PassingMode::Shared)
                    }
                    _ => quote!(#facade::__private::codegen_v4::invoke::PassingMode::Owned),
                };
                let ty = type_expression(&parameter.ty, &method_environment, facade);
                let index = parameter.index;
                quote!(#facade::__private::codegen_v4::invoke::ParameterMetadata::new(
                    #index, #pattern, #passing, #ty
                ))
            });
            let output = match &method.return_type {
                ReturnTypeIr::Unit => quote!(#facade::__private::codegen_v4::invoke::ReturnMetadata::unit()),
                ReturnTypeIr::Type(ty) => {
                    let expression = type_expression(ty, &method_environment, facade);
                    let kind = match ty.kind {
                        TypeKindIr::Never => quote!(#facade::__private::codegen_v4::invoke::ReturnKind::Never),
                        TypeKindIr::Reference { mutable: true, .. } => {
                            quote!(#facade::__private::codegen_v4::invoke::ReturnKind::Mutable)
                        }
                        TypeKindIr::Reference { .. } => {
                            quote!(#facade::__private::codegen_v4::invoke::ReturnKind::Shared)
                        }
                        TypeKindIr::ImplTrait { .. } => {
                            quote!(#facade::__private::codegen_v4::invoke::ReturnKind::Opaque)
                        }
                        _ => quote!(#facade::__private::codegen_v4::invoke::ReturnKind::Owned),
                    };
                    quote!(#facade::__private::codegen_v4::invoke::ReturnMetadata::new(#kind, #expression))
                }
            };
            let qualifiers = &method.qualifiers;
            let method_generic_definition = generic_definition(&method.generics, method.span, facade);
            let is_async = qualifiers.is_async;
            let is_unsafe = qualifiers.is_unsafe;
            let is_const = qualifiers.is_const;
            let is_variadic = qualifiers.is_variadic;
            let abi = qualifiers
                .abi
                .as_deref()
                .map(|abi| function_abi(Some(abi), method.span, facade));
            let abi = match abi {
                Some(abi) => quote!(Some(#abi)),
                None => quote!(None),
            };
            let has_default = method.has_default;
            let is_generic = !method.generics.params.is_empty();
            quote! {
                #facade::__private::codegen_v4::invoke::MethodMetadata::from_spec(
                    #facade::__private::codegen_v4::invoke::MethodMetadataSpec::new(
                        #query,
                        #facade::__private::codegen_v4::invoke::MethodNamespaceMetadata::Trait {
                            trait_: #trait_name_literal,
                            implementation: #name,
                        },
                        #facade::__private::codegen_v4::invoke::MethodVisibility::Public,
                        #receiver,
                        ::std::boxed::Box::leak(::std::vec![#(#parameters),*].into_boxed_slice()),
                        ::std::boxed::Box::leak(::std::boxed::Box::new(#output)),
                        #facade::__private::codegen_v4::invoke::MethodQualifiers::new(
                            #is_const, #is_async, #is_unsafe, #is_variadic,
                        ).with_abi(#abi),
                        ::std::boxed::Box::leak(::std::boxed::Box::new(#method_generic_definition)),
                        #is_generic,
                        #has_default,
                    ),
                )
            }
        })
        .collect();
    let associated_types: Vec<_> = declaration
        .associated_types
        .iter()
        .map(|item| {
            let name = LitStr::new(&item.name.to_string(), item.span);
            let item_environment = environment.clone().with_generics(&item.generics);
            let bounds = item.bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Trait {
                    path,
                    lifetimes,
                    modifier,
                } => {
                    let path = path_type_expression(path, &item_environment, facade);
                    let lifetimes = lifetimes
                        .iter()
                        .map(|lifetime| lifetime_expression(lifetime, item.span, facade));
                    let modifier = match modifier {
                        crate::ir::TraitBoundModifierIr::None => {
                            quote!(#facade::__private::codegen_v4::expression::TraitBoundModifier::None)
                        }
                        crate::ir::TraitBoundModifierIr::Maybe => {
                            quote!(#facade::__private::codegen_v4::expression::TraitBoundModifier::Maybe)
                        }
                    };
                    Some(quote!(#facade::__private::codegen_v4::expression::type_bound(
                        #facade::__private::codegen_v4::expression::parameter(#name),
                        Box::new([#path]),
                        Box::new([#modifier]),
                        Box::new([#(#lifetimes),*]),
                    )))
                }
                GenericBoundIr::Lifetime(lifetime) => {
                    let lifetime = lifetime_expression(lifetime, item.span, facade);
                    Some(
                        quote!(#facade::__private::codegen_v4::expression::PredicateDescriptor::TypeOutlives {
                            ty: #facade::__private::codegen_v4::expression::parameter(#name),
                            lifetime: #lifetime,
                            diagnostic: #facade::__private::codegen_v4::expression::DiagnosticText::default(),
                        }),
                    )
                }
                _ => None,
            });
            let default = item
                .value
                .as_ref()
                .map(|value| type_expression(value, &item_environment, facade));
            let default = match default {
                Some(value) => quote!(Some(#value)),
                None => quote!(None),
            };
            quote!(#facade::__private::codegen_v4::traits::associated_type(
                #name,
                #facade::__private::codegen_v4::traits::TraitVisibility::Public,
                &[#(#bounds),*],
                #default,
            ))
        })
        .collect();
    let associated_consts: Vec<_> = declaration
        .associated_consts
        .iter()
        .map(|item| {
            let name = LitStr::new(&item.name.to_string(), item.span);
            let ty = type_expression(&item.ty, &environment, facade);
            let default = item
                .value
                .as_ref()
                .map(|value| const_expression(value, &environment, facade))
                .unwrap_or_else(|| quote!(None));
            quote!(#facade::__private::codegen_v4::traits::associated_const(
                #name,
                #facade::__private::codegen_v4::traits::TraitVisibility::Public,
                #ty,
                #default,
            ))
        })
        .collect();
    let parameters = declaration.generics.params.iter().map(|parameter| {
        let name = LitStr::new(&parameter.name, declaration.span);
        match parameter.kind {
            GenericKindIr::Lifetime => {
                let bounds = parameter.bounds.iter().filter_map(|bound| match bound {
                    GenericBoundIr::Lifetime(lifetime) => Some(lifetime_expression(lifetime, declaration.span, facade)),
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
                    GenericBoundIr::Trait {
                        path,
                        lifetimes,
                        modifier,
                    } => {
                        let path = path_type_expression(path, &environment, facade);
                        let lifetimes = lifetimes
                            .iter()
                            .map(|lifetime| lifetime_expression(lifetime, declaration.span, facade));
                        let modifier = match modifier {
                            crate::ir::TraitBoundModifierIr::None => {
                                quote!(#facade::__private::codegen_v4::expression::TraitBoundModifier::None)
                            }
                            crate::ir::TraitBoundModifierIr::Maybe => {
                                quote!(#facade::__private::codegen_v4::expression::TraitBoundModifier::Maybe)
                            }
                        };
                        Some(quote!(#facade::__private::codegen_v4::expression::type_bound(
                            #facade::__private::codegen_v4::expression::parameter(#subject),
                            Box::new([#path]),
                            Box::new([#modifier]),
                            Box::new([#(#lifetimes),*]),
                        )))
                    }
                    GenericBoundIr::Lifetime(lifetime) => {
                        let lifetime = lifetime_expression(lifetime, declaration.span, facade);
                        Some(
                            quote!(#facade::__private::codegen_v4::expression::PredicateDescriptor::TypeOutlives {
                                ty: #facade::__private::codegen_v4::expression::parameter(#subject),
                                lifetime: #lifetime,
                                diagnostic: #facade::__private::codegen_v4::expression::DiagnosticText::default(),
                            }),
                        )
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
                    Some(crate::ir::GenericDefaultIr::Const(value)) => const_expression(value, &environment, facade),
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
    let where_predicates = declaration
        .generics
        .where_predicates
        .iter()
        .flat_map(|predicate| match predicate {
            crate::ir::WherePredicateIr::Lifetime { lifetime, bounds, .. } => {
                let lifetime = lifetime_expression(lifetime, declaration.span, facade);
                let bounds: Vec<_> = bounds
                    .iter()
                    .map(|bound| lifetime_expression(bound, declaration.span, facade))
                    .collect();
                vec![quote!(#facade::__private::codegen_v4::expression::lifetime_outlives(
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
                let higher_ranked_lifetimes: Vec<_> = lifetimes
                    .iter()
                    .map(|lifetime| lifetime_expression(lifetime, declaration.span, facade))
                    .collect();
                let trait_bounds: Vec<_> = bounds
                    .iter()
                    .filter_map(|bound| match bound {
                        GenericBoundIr::Trait { path, .. } => Some(path_type_expression(path, &environment, facade)),
                        _ => None,
                    })
                    .collect();
                let bound_modifiers: Vec<_> = bounds
                    .iter()
                    .filter_map(|bound| match bound {
                        GenericBoundIr::Trait { modifier, .. } => Some(match modifier {
                            crate::ir::TraitBoundModifierIr::None => {
                                quote!(#facade::__private::codegen_v4::expression::TraitBoundModifier::None)
                            }
                            crate::ir::TraitBoundModifierIr::Maybe => {
                                quote!(#facade::__private::codegen_v4::expression::TraitBoundModifier::Maybe)
                            }
                        }),
                        _ => None,
                    })
                    .collect();
                let type_bound = (!trait_bounds.is_empty()).then(|| {
                    quote!(#facade::__private::codegen_v4::expression::type_bound(
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
                        Some(
                            quote!(#facade::__private::codegen_v4::expression::PredicateDescriptor::TypeOutlives {
                                ty: #subject,
                                lifetime: #lifetime,
                                diagnostic: #facade::__private::codegen_v4::expression::DiagnosticText::default(),
                            }),
                        )
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
