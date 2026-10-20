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
use crate::expand::generic_environment::GenericEnvironment;
use crate::ir::GenericBoundIr;
use crate::ir::ParameterPatternKindIr;
use crate::ir::PathArgumentIr;
use crate::ir::PathArgumentsIr;
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
pub(super) fn build(declaration: &TraitDeclarationIr, trait_name: &LitStr, facade: &TokenStream) -> TraitMetadata {
    let trait_name_literal = trait_name;
    let environment = GenericEnvironment::from_generics(&declaration.generics);
    let methods: Vec<_> = declaration
        .methods
        .iter()
        .map(|method| {
            let method_environment = environment.clone().with_generics(&method.generics);
            let name = LitStr::new(&method.name.to_string(), method.span);
            let query = method.name.to_string();
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
                let concrete_type = concrete_value_type(&parameter.ty, &method_environment)
                    .map(|ty| {
                        let ty = &ty.tokens;
                        quote!(.with_concrete_type::<#ty>())
                    })
                    .unwrap_or_default();
                let index = parameter.index;
                quote!(#facade::__private::codegen_v4::invoke::ParameterMetadata::new(
                    #index, #pattern, #passing, #ty
                ) #concrete_type)
            });
            let output = match &method.return_type {
                ReturnTypeIr::Unit => quote!(#facade::__private::codegen_v4::invoke::ReturnMetadata::unit()),
                ReturnTypeIr::Type(ty) => {
                    let expression = type_expression(ty, &method_environment, facade);
                    let concrete_type = concrete_value_type(ty, &method_environment)
                        .filter(|concrete| {
                            !matches!(concrete.kind, TypeKindIr::Never)
                                && !matches!(
                                    &concrete.kind,
                                    TypeKindIr::Path(path)
                                        if path.segments.last().is_some_and(|segment| segment.name == "str")
                                )
                        })
                        .map(|ty| {
                            let ty = &ty.tokens;
                            quote!(.with_concrete_type::<#ty>())
                        })
                        .unwrap_or_default();
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
                    quote!(#facade::__private::codegen_v4::invoke::ReturnMetadata::new(#kind, #expression) #concrete_type)
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
                            trait_: concat!(module_path!(), "::", #trait_name_literal),
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
            let bounds = item
                .bounds
                .iter()
                .filter_map(|bound| associated_type_bound(bound, &name, &item_environment, item.span, facade));
            let bounds = leak_slice(bounds);
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
                #bounds,
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
    TraitMetadata {
        methods,
        associated_types,
        associated_consts,
    }
}

fn concrete_value_type<'a>(
    ty: &'a crate::ir::TypeIr,
    environment: &GenericEnvironment,
) -> Option<&'a crate::ir::TypeIr> {
    let value_type = match &ty.kind {
        TypeKindIr::Reference { element, .. } => element.as_ref(),
        _ => ty,
    };
    is_static_concrete_type(value_type, environment).then_some(value_type)
}

fn is_static_concrete_type(ty: &crate::ir::TypeIr, environment: &GenericEnvironment) -> bool {
    match &ty.kind {
        TypeKindIr::Path(path) => {
            if path.segments.iter().any(|segment| segment.name == "Self") {
                return false;
            }
            if path
                .qualified_self
                .as_ref()
                .is_some_and(|self_type| !is_static_concrete_type(&self_type.ty, environment))
            {
                return false;
            }
            path.segments.iter().all(|segment| {
                !environment.is_type_parameter(&segment.name)
                    && match &segment.arguments {
                        PathArgumentsIr::None => true,
                        PathArgumentsIr::AngleBracketed(arguments) => arguments.iter().all(|argument| match argument {
                            PathArgumentIr::Lifetime(lifetime) => lifetime == "'static",
                            PathArgumentIr::Type(ty) => is_static_concrete_type(ty, environment),
                            PathArgumentIr::Const(value) => {
                                !contains_generic_const(value.to_string().as_str(), environment)
                            }
                            PathArgumentIr::AssociatedType { .. }
                            | PathArgumentIr::AssociatedConst { .. }
                            | PathArgumentIr::Constraint { .. }
                            | PathArgumentIr::Other(_) => false,
                        }),
                        PathArgumentsIr::Parenthesized { inputs, output } => {
                            inputs.iter().all(|input| is_static_concrete_type(input, environment))
                                && output
                                    .as_ref()
                                    .is_none_or(|output| is_static_concrete_type(output, environment))
                        }
                    }
            })
        }
        TypeKindIr::Tuple(elements) => elements
            .iter()
            .all(|element| is_static_concrete_type(element, environment)),
        TypeKindIr::Array { element, length } => {
            is_static_concrete_type(element, environment)
                && !contains_generic_const(length.to_string().as_str(), environment)
        }
        TypeKindIr::Pointer { element, .. } => is_static_concrete_type(element, environment),
        TypeKindIr::Reference { .. }
        | TypeKindIr::Slice(_)
        | TypeKindIr::BareFunction { .. }
        | TypeKindIr::TraitObject { .. }
        | TypeKindIr::ImplTrait { .. }
        | TypeKindIr::Never
        | TypeKindIr::Infer
        | TypeKindIr::Macro
        | TypeKindIr::Other => false,
    }
}

fn contains_generic_const(value: &str, environment: &GenericEnvironment) -> bool {
    value
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .any(|identifier| environment.is_const_parameter(identifier))
}

fn leak_slice(values: impl IntoIterator<Item = TokenStream>) -> TokenStream {
    let values = values.into_iter().collect::<Vec<_>>();
    quote!(::std::boxed::Box::leak(::std::vec![#(#values),*].into_boxed_slice()))
}

fn associated_type_bound(
    bound: &GenericBoundIr,
    name: &LitStr,
    environment: &GenericEnvironment,
    span: proc_macro2::Span,
    facade: &TokenStream,
) -> Option<TokenStream> {
    let subject = quote!(::std::boxed::Box::leak(::std::boxed::Box::new(
        #facade::__private::codegen_v4::expression::ExpressionData::AssociatedType {
            self_type: ::std::boxed::Box::leak(::std::boxed::Box::new(
                #facade::__private::codegen_v4::expression::ExpressionData::SelfType,
            )),
            trait_: None,
            name: #name,
        },
    )));
    match bound {
        GenericBoundIr::Trait {
            path,
            lifetimes,
            modifier,
        } => {
            let path_name = LitStr::new(&path.source, path.span);
            let arguments = path.segments.iter().flat_map(|segment| {
                crate::expand::expression_codegen::path_arguments(&segment.arguments, environment, facade, path.span)
            });
            let arguments = leak_slice(arguments);
            let lifetimes = lifetimes
                .iter()
                .map(|lifetime| lifetime_expression(lifetime, span, facade));
            let lifetimes = leak_slice(lifetimes);
            let trait_expression = quote!(::std::boxed::Box::leak(::std::boxed::Box::new(
                #facade::__private::codegen_v4::expression::TraitExpressionData::new(
                    #path_name, #arguments, #lifetimes,
                ),
            )));
            let modifier = match modifier {
                crate::ir::TraitBoundModifierIr::None => {
                    quote!(#facade::__private::codegen_v4::expression::TraitBoundModifier::None)
                }
                crate::ir::TraitBoundModifierIr::Maybe => {
                    quote!(#facade::__private::codegen_v4::expression::TraitBoundModifier::Maybe)
                }
            };
            Some(
                quote!(#facade::__private::codegen_v4::expression::PredicateData::TraitBound {
                    ty: #subject,
                    trait_: #trait_expression,
                    modifier: #modifier,
                }),
            )
        }
        GenericBoundIr::Lifetime(lifetime) => {
            let lifetime = lifetime_expression(lifetime, span, facade);
            Some(
                quote!(#facade::__private::codegen_v4::expression::PredicateData::TypeOutlives {
                    ty: #subject,
                    lifetime: #lifetime,
                }),
            )
        }
        GenericBoundIr::Other(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use quote::quote;

    use super::associated_type_bound;
    use crate::expand::generic_environment::GenericEnvironment;
    use crate::ir::GenericBoundIr;
    use crate::ir::PathArgumentsIr;
    use crate::ir::PathIr;
    use crate::ir::PathSegmentIr;
    use crate::ir::TraitBoundModifierIr;

    #[test]
    fn test_associated_type_bounds_emit_typed_expression_data() {
        let span = Span::call_site();
        let bound = GenericBoundIr::Trait {
            path: PathIr {
                source: "Send".to_owned(),
                segments: vec![PathSegmentIr {
                    name: "Send".to_owned(),
                    arguments: PathArgumentsIr::None,
                }],
                leading_colon: false,
                qualified_self: None,
                tokens: quote!(Send),
                span,
            },
            modifier: TraitBoundModifierIr::None,
            lifetimes: Vec::new(),
        };
        let name = syn::LitStr::new("Item", span);
        let tokens = associated_type_bound(
            &bound,
            &name,
            &GenericEnvironment::new(),
            span,
            &quote!(::qubit_reflect),
        )
        .expect("trait bound should emit metadata")
        .to_string();

        assert!(tokens.contains("PredicateData :: TraitBound"), "{tokens}");
        assert!(tokens.contains("TraitExpressionData :: new"), "{tokens}");
        assert!(tokens.contains("ExpressionData :: AssociatedType"), "{tokens}");
        assert!(!tokens.contains("type_bound"), "{tokens}");
        assert!(!tokens.contains("PredicateDescriptor"), "{tokens}");
    }
}
