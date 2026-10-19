// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Expansion of distributed registration fragments for `#[reflect_impl]`.

mod internal;
mod invocation_adapter;
mod specialization_codegen;

use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use specialization_codegen::typed_extension_receiver_type;

use crate::expand::ExpansionContext;
use crate::expand::generic_environment::GenericEnvironment;
use crate::ir::HelperValueIr;
use crate::ir::ImplDeclarationIr;
use crate::ir::MethodIr;
use crate::ir::ParameterPatternKindIr;
use crate::ir::ReceiverKindIr;
use crate::ir::ReturnTypeIr;
use crate::ir::TypeKindIr;
use crate::ir::VisibilityIr;

/// Emits typed codegen_v4 metadata for one implementation method.
fn method_metadata(
    method: &MethodIr,
    environment: &GenericEnvironment,
    target: &TokenStream,
    namespace: &TokenStream,
    facade: &TokenStream,
    symbolic_target: bool,
) -> TokenStream {
    let method_environment = environment.clone().with_generics(&method.generics);
    let query = method
        .attributes
        .iter()
        .find_map(|attribute| attribute.rename())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| method.name.to_string());
    let query = syn::LitStr::new(&query, method.span);
    let visibility = match method.visibility {
        VisibilityIr::Public => {
            quote!(#facade::__private::codegen_v4::invoke::MethodVisibility::Public)
        }
        VisibilityIr::Restricted(_) => {
            quote!(#facade::__private::codegen_v4::invoke::MethodVisibility::Restricted)
        }
        VisibilityIr::Crate
        | VisibilityIr::Super
        | VisibilityIr::SelfValue
        | VisibilityIr::Inherited => {
            quote!(#facade::__private::codegen_v4::invoke::MethodVisibility::Private)
        }
    };
    let receiver = match &method.receiver {
        Some(receiver_ir) => {
            let (receiver_ty, kind) = match receiver_ir.kind {
                ReceiverKindIr::Value => (target.clone(), quote!(Owned)),
                ReceiverKindIr::SharedReference => (target.clone(), quote!(Shared)),
                ReceiverKindIr::MutableReference => (target.clone(), quote!(Mutable)),
                ReceiverKindIr::Typed => {
                    let kind =
                        if super::invocation::analysis::typed_pinned_receiver_mutable(receiver_ir)
                            == Some(true)
                        {
                            quote!(PinnedMutable)
                        } else if super::invocation::analysis::typed_pinned_receiver_mutable(
                            receiver_ir,
                        ) == Some(false)
                        {
                            quote!(PinnedShared)
                        } else {
                            quote!(Owned)
                        };
                    (receiver_ir.ty.tokens.clone(), kind)
                }
            };
            let receiver_kind = quote!(#facade::__private::codegen_v4::invoke::ReceiverKind::#kind);
            if symbolic_target {
                let expression = if matches!(receiver_ir.kind, ReceiverKindIr::Typed) {
                    super::traits::type_expression(&receiver_ir.ty, &method_environment, facade)
                } else {
                    target.clone()
                };
                quote!(Some(#facade::__private::codegen_v4::invoke::ReceiverDescriptor::trait_self(
                    #receiver_kind,
                    #expression,
                )))
            } else {
                let receiver_descriptor = quote!(#facade::__private::codegen_v4::invoke::ReceiverDescriptor::for_type::<#receiver_ty>(#receiver_kind));
                if matches!(receiver_ir.kind, ReceiverKindIr::Typed) {
                    let expression = super::traits::type_expression(
                        &receiver_ir.ty,
                        &method_environment,
                        facade,
                    );
                    quote!(Some(#receiver_descriptor.with_expression(#expression)))
                } else {
                    quote!(Some(#receiver_descriptor))
                }
            }
        }
        None => quote!(None),
    };
    let parameters = method.parameters.iter().map(|parameter| {
        let pattern = match parameter.pattern.kind {
            ParameterPatternKindIr::Identifier => {
                let name = syn::LitStr::new(parameter.name.as_deref().unwrap_or(""), parameter.span);
                quote!(#facade::__private::codegen_v4::invoke::ParameterPattern::Identifier(#name))
            }
            ParameterPatternKindIr::Wildcard => {
                quote!(#facade::__private::codegen_v4::invoke::ParameterPattern::Wildcard)
            }
            ParameterPatternKindIr::Destructure => {
                let pattern = syn::LitStr::new(&parameter.pattern.source, parameter.span);
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
        let expression = super::traits::type_expression(&parameter.ty, &method_environment, facade);
        let index = parameter.index;
        quote!(#facade::__private::codegen_v4::invoke::ParameterMetadata::new(#index, #pattern, #passing, #expression))
    });
    let output = match &method.return_type {
        ReturnTypeIr::Unit => {
            quote!(#facade::__private::codegen_v4::invoke::ReturnMetadata::unit())
        }
        ReturnTypeIr::Type(ty) => {
            let expression = super::traits::type_expression(ty, &method_environment, facade);
            let kind = if method.qualifiers.is_async {
                quote!(Future)
            } else {
                match ty.kind {
                    TypeKindIr::Never => quote!(Never),
                    TypeKindIr::Reference { mutable: true, .. } => quote!(Mutable),
                    TypeKindIr::Reference { .. } => quote!(Shared),
                    TypeKindIr::ImplTrait { .. } => quote!(Opaque),
                    _ => quote!(Owned),
                }
            };
            quote!(#facade::__private::codegen_v4::invoke::ReturnMetadata::new(
                #facade::__private::codegen_v4::invoke::ReturnKind::#kind,
                #expression,
            ))
        }
    };
    let qualifiers = &method.qualifiers;
    let generics = super::traits::generic_definition(&method.generics, method.span, facade);
    let is_const = qualifiers.is_const;
    let is_async = qualifiers.is_async;
    let is_unsafe = qualifiers.is_unsafe;
    let is_variadic = qualifiers.is_variadic;
    let abi = qualifiers
        .abi
        .as_deref()
        .map(|abi| crate::expand::expression_codegen::function_abi(Some(abi), method.span, facade));
    let abi = abi.map_or_else(|| quote!(None), |abi| quote!(Some(#abi)));
    let has_default = method.has_default;
    let is_generic = !method.generics.params.is_empty();
    quote! {
        #facade::__private::codegen_v4::invoke::MethodMetadata::from_spec(
            #facade::__private::codegen_v4::invoke::MethodMetadataSpec::new(
                #query,
                #namespace,
                #visibility,
                #receiver,
                ::std::boxed::Box::leak(::std::vec![#(#parameters),*].into_boxed_slice()),
                ::std::boxed::Box::leak(::std::boxed::Box::new(#output)),
                #facade::__private::codegen_v4::invoke::MethodQualifiers::new(
                    #is_const, #is_async, #is_unsafe, #is_variadic,
                ).with_abi(#abi),
                ::std::boxed::Box::leak(::std::boxed::Box::new(#generics)),
                #is_generic,
                #has_default,
            ),
        )
    }
}

/// Expands an impl unchanged and submits a lazily-built implementation
/// fragment.
///
/// The descriptor graph is deliberately constructed during registry
/// initialization, not from an inventory constructor. This keeps user code out
/// of linker startup and uses the T12 registration protocol exclusively.
///
/// # Parameters
///
/// - `declaration`: Validated inherent or trait implementation.
/// - `context`: Expansion context containing the runtime facade.
///
/// # Returns
///
/// Returns the retained impl and generated registration metadata.
#[must_use]
pub(crate) fn expand_impl(
    declaration: ImplDeclarationIr,
    context: &ExpansionContext,
) -> TokenStream {
    let tokens = if !declaration.generics.params.is_empty() {
        let retained = declaration.retained_tokens.clone();
        let definition = expand_generic_impl_definition(&declaration, context.facade(), context);
        quote!(#retained #definition)
    } else {
        expand_concrete_impl(
            declaration,
            quote!(::std::vec![]),
            None,
            None,
            Vec::new(),
            context.facade(),
            context,
        )
    };
    super::invocation::emit::rewrite_codegen_paths(tokens)
}

/// Emits one declaration-level registration for a generic inherent impl.
///
/// This payload deliberately has no `TypeId`: it describes the symbolic impl
/// declaration and cannot enter a concrete type's effective view.
///
/// # Parameters
///
/// - `declaration`: Generic impl whose symbolic definition is registered.
/// - `facade`: Runtime facade path used in generated references.
/// - `context`: Expansion context used to fingerprint the declaration.
///
/// # Returns
///
/// Returns generated definition-provider and registration tokens.
#[must_use]
fn expand_generic_impl_definition(
    declaration: &ImplDeclarationIr,
    facade: &TokenStream,
    context: &ExpansionContext,
) -> TokenStream {
    let fingerprint = context.fingerprint(&declaration.retained_tokens.to_string());
    let module = generic_impl_definition_module(declaration);
    let marker = format_ident!("__QubitReflectImplDef_{fingerprint:016x}");
    let provider = format_ident!("__qubit_reflect_impl_definition_provider_{fingerprint:016x}");
    let codegen = quote!(#facade::__private::codegen_v4);
    let environment = GenericEnvironment::from_generics(&declaration.generics);
    let target = super::traits::type_expression(&declaration.target_type, &environment, facade);
    let generics =
        super::traits::generic_definition(&declaration.generics, declaration.span, facade);
    let target_source = syn::LitStr::new(&declaration.target_type.source, declaration.span);
    let namespace = if let Some(path) = &declaration.trait_path {
        let trait_name = syn::LitStr::new(&path.source, declaration.span);
        quote!(#codegen::invoke::MethodNamespaceMetadata::Trait {
            trait_: #trait_name,
            implementation: #target_source,
        })
    } else {
        quote!(#codegen::invoke::MethodNamespaceMetadata::Inherent {
            implementation: #target_source,
        })
    };
    let methods: Vec<_> = declaration
        .methods
        .iter()
        .map(|method| method_metadata(method, &environment, &target, &namespace, facade, true))
        .collect();
    let associated_types: Vec<_> = declaration
        .associated_types
        .iter()
        .map(|item| {
            let name = syn::LitStr::new(&item.name.to_string(), item.span);
            let value = item
                .value
                .as_ref()
                .map(|value| super::traits::type_expression(value, &environment, facade));
            let value = value.map_or_else(|| quote!(None), |value| quote!(Some(#value)));
            quote!(#codegen::traits::associated_type(
                #name,
                #codegen::traits::TraitVisibility::Public,
                &[],
                #value,
            ))
        })
        .collect();
    let associated_consts: Vec<_> = declaration
        .associated_consts
        .iter()
        .map(|item| {
            let name = syn::LitStr::new(&item.name.to_string(), item.span);
            let ty = super::traits::type_expression(&item.ty, &environment, facade);
            let value = item.value.as_ref().map(|value| {
                super::expression_codegen::const_expression(value, &environment, facade)
            });
            let value = value.map_or_else(|| quote!(None), |value| quote!(Some(#value)));
            quote!(#codegen::traits::associated_const(
                #name,
                #codegen::traits::TraitVisibility::Public,
                #ty,
                #value,
            ))
        })
        .collect();
    let implemented_trait = match &declaration.trait_path {
        Some(path) => {
            let name = syn::LitStr::new(&path.source, path.span);
            let arguments = path.segments.iter().flat_map(|segment| {
                super::expression_codegen::path_arguments(
                    &segment.arguments,
                    &environment,
                    facade,
                    path.span,
                )
            });
            quote!(Some(#codegen::traits::unresolved_trait_expression(
                ::std::boxed::Box::leak(::std::boxed::Box::new(
                    #codegen::expression::TraitExpressionData::new(
                        #name,
                        ::std::boxed::Box::leak(::std::vec![#(#arguments),*].into_boxed_slice()),
                        &[],
                    ),
                )),
            )))
        }
        None => quote!(None),
    };
    let kind = if declaration.trait_path.is_some() {
        quote!(#codegen::impls::Kind::Trait)
    } else {
        quote!(#codegen::impls::Kind::Inherent)
    };
    quote! {
        #[doc(hidden)]
        mod #module {
            use super::*;

            #[doc(hidden)]
            pub struct #marker;

            fn #provider() -> &'static #codegen::impls::ImplDefinitionSpec<#marker> {
                static VALUE: ::std::sync::OnceLock<#codegen::impls::ImplDefinitionSpec<#marker>> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| {
                    let generics = #generics;
                    let methods = ::std::boxed::Box::leak(::std::vec![#(#methods),*].into_boxed_slice());
                    let associated_types = ::std::boxed::Box::leak(::std::vec![#(#associated_types),*].into_boxed_slice());
                    let associated_consts = ::std::boxed::Box::leak(::std::vec![#(#associated_consts),*].into_boxed_slice());
                    #codegen::impls::ImplDefinitionSpec::new(
                        #kind,
                        #target,
                        #implemented_trait,
                        ::std::boxed::Box::leak(::std::boxed::Box::new(generics)),
                        #codegen::impls::Qualifiers::new(false),
                        methods,
                        associated_types,
                        associated_consts,
                    )
                })
            }

            #codegen::impls::submit_impl_definition!(#marker, #provider);
        }
    }
}

/// Returns the stable generated module name owning one generic impl
/// definition.
///
/// # Parameters
///
/// - `declaration`: Generic impl whose source location and tokens identify the
///   module.
///
/// # Returns
///
/// Returns the deterministic hidden module identifier.
#[must_use]
fn generic_impl_definition_module(declaration: &ImplDeclarationIr) -> Ident {
    let fingerprint = super::context::fingerprint(&declaration.retained_tokens.to_string());
    let location = declaration.span.start();
    format_ident!(
        "__qubit_reflect_impl_definition_{fingerprint:x}_{}_{}",
        location.line,
        location.column,
    )
}

/// Expands one concrete implementation registration fragment.
///
/// # Parameters
///
/// - `declaration`: Concrete impl declaration to emit.
/// - `impl_arguments`: Runtime generic arguments retained for specialization.
/// - `shared_definition`: Optional shared generic impl-definition provider.
/// - `applicability_witness`: Optional compile-time specialization proof.
/// - `specialized_associated_type_resolver_arms`: Resolver arms for specialized
///   associated types.
/// - `facade`: Runtime facade path used in generated references.
/// - `context`: Expansion context used to analyze invocation support.
///
/// # Returns
///
/// Returns the retained impl and generated concrete registration module.
#[must_use]
fn expand_concrete_impl(
    declaration: ImplDeclarationIr,
    _impl_arguments: TokenStream,
    _shared_definition: Option<TokenStream>,
    _applicability_witness: Option<TokenStream>,
    _specialized_associated_type_resolver_arms: Vec<TokenStream>,
    facade: &TokenStream,
    context: &ExpansionContext,
) -> TokenStream {
    let facade = facade.clone();
    let codegen = quote!(#facade::__private::codegen_v4);
    let target = declaration.target_type.tokens.clone();
    let environment = GenericEnvironment::from_generics(&declaration.generics);
    let fingerprint = context.fingerprint(&declaration.retained_tokens.to_string());
    let location = declaration.span.start();
    let module = format_ident!(
        "__qubit_reflect_impl_{fingerprint:x}_{}_{}",
        location.line,
        location.column,
    );
    let marker = format_ident!("__QubitReflectImplDef_{fingerprint:016x}");
    let target_expression =
        super::traits::type_expression(&declaration.target_type, &environment, &facade);
    let generic_definition =
        super::traits::generic_definition(&declaration.generics, declaration.span, &facade);
    let implementation_name = syn::LitStr::new(&declaration.target_type.source, declaration.span);
    let namespace = match &declaration.trait_path {
        Some(path) => {
            let trait_name = syn::LitStr::new(&path.source, declaration.span);
            quote!(#codegen::invoke::MethodNamespaceMetadata::Trait {
                trait_: #trait_name,
                implementation: #implementation_name,
            })
        }
        None => quote!(#codegen::invoke::MethodNamespaceMetadata::Inherent {
            implementation: #implementation_name,
        }),
    };
    let methods: Vec<_> = declaration
        .methods
        .iter()
        .map(|method| method_metadata(method, &environment, &target, &namespace, &facade, false))
        .collect();
    let associated_types: Vec<_> = declaration
        .associated_types
        .iter()
        .map(|item| {
            let name = syn::LitStr::new(&item.name.to_string(), item.span);
            let value = item
                .value
                .as_ref()
                .map(|value| super::traits::type_expression(value, &environment, &facade));
            let value = value.map_or_else(|| quote!(None), |value| quote!(Some(#value)));
            quote!(#codegen::traits::associated_type(
                #name,
                #codegen::traits::TraitVisibility::Public,
                &[],
                #value,
            ))
        })
        .collect();
    let associated_consts: Vec<_> = declaration
        .associated_consts
        .iter()
        .map(|item| {
            let name = syn::LitStr::new(&item.name.to_string(), item.span);
            let ty = super::traits::type_expression(&item.ty, &environment, &facade);
            let value = item.value.as_ref().map(|value| {
                super::expression_codegen::const_expression(value, &environment, &facade)
            });
            let value = value.map_or_else(|| quote!(None), |value| quote!(Some(#value)));
            quote!(#codegen::traits::associated_const(
                #name,
                #codegen::traits::TraitVisibility::Public,
                #ty,
                #value,
            ))
        })
        .collect();
    let kind = if declaration.trait_path.is_some() {
        quote!(#codegen::impls::Kind::Trait)
    } else {
        quote!(#codegen::impls::Kind::Inherent)
    };
    let is_external_trait = declaration
        .attributes
        .iter()
        .any(|attribute| matches!(attribute.value, HelperValueIr::ExternalTraitId(_)));
    let trait_definition_provider = declaration
        .trait_path
        .as_ref()
        .filter(|_| !is_external_trait)
        .map(|path| {
            let mut provider_path: syn::Path =
                syn::parse2(path.tokens.clone()).expect("validated trait path parses");
            let Some(last) = provider_path.segments.last_mut() else {
                unreachable!()
            };
            last.arguments = syn::PathArguments::None;
            last.ident = format_ident!("__QubitReflectTraitDef_{}_SPEC", last.ident);
            quote!(#provider_path)
        });
    let trait_marker_path = declaration
        .trait_path
        .as_ref()
        .filter(|_| !is_external_trait)
        .map(|path| {
            let mut marker_path: syn::Path =
                syn::parse2(path.tokens.clone()).expect("validated trait path parses");
            let Some(last) = marker_path.segments.last_mut() else {
                unreachable!()
            };
            last.arguments = syn::PathArguments::None;
            last.ident = format_ident!("__QubitReflectTraitDef_{}", last.ident);
            quote!(#marker_path)
        });
    let trait_default_methods_provider = trait_marker_path.as_ref().map(|marker_path| {
        let mut provider_path: syn::Path =
            syn::parse2(marker_path.clone()).expect("validated trait marker path parses");
        let last = provider_path
            .segments
            .last_mut()
            .expect("validated trait path has a final segment");
        last.ident = format_ident!("{}_DEFAULT_METHODS", last.ident);
        last.arguments = syn::PathArguments::None;
        quote!(#provider_path)
    });
    let trait_default_method_arguments = declaration
        .trait_path
        .as_ref()
        .and_then(|path| syn::parse2::<syn::Path>(path.tokens.clone()).ok())
        .and_then(|path| {
            path.segments
                .last()
                .map(|segment| segment.arguments.clone())
        })
        .and_then(|arguments| match arguments {
            syn::PathArguments::AngleBracketed(arguments) => Some(
                arguments
                    .args
                    .iter()
                    .filter_map(|argument| match argument {
                        syn::GenericArgument::Lifetime(lifetime) => Some(quote!(#lifetime)),
                        syn::GenericArgument::Type(ty) => Some(quote!(#ty)),
                        syn::GenericArgument::Const(value) => Some(quote!(#value)),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .unwrap_or_default();
    let trait_default_provider_arguments = {
        let lifetime_arguments = trait_default_method_arguments
            .iter()
            .filter(|argument| argument.to_string().starts_with('\''))
            .cloned()
            .collect::<Vec<_>>();
        let type_const_arguments = trait_default_method_arguments
            .iter()
            .filter(|argument| !argument.to_string().starts_with('\''))
            .cloned()
            .collect::<Vec<_>>();
        quote!(::<#(#lifetime_arguments,)* #target, #(#type_const_arguments),*>)
    };
    let overridden_method_names = declaration
        .methods
        .iter()
        .map(|method| syn::LitStr::new(&method.name.to_string(), method.span));
    let default_methods = trait_default_methods_provider.as_ref().map_or_else(
        || quote!(::std::vec::Vec::<#codegen::invoke::EffectiveMethod>::new().into_boxed_slice()),
        |provider| quote!(#provider #trait_default_provider_arguments(&[#(#overridden_method_names),*])),
    );
    let (trait_application_arguments, trait_argument_identity) = declaration
        .trait_path
        .as_ref()
        .and_then(|path| path.segments.last())
        .map(|segment| {
            let mut arguments = Vec::new();
            let mut identity = Vec::new();
            if let crate::ir::PathArgumentsIr::AngleBracketed(source_arguments) = &segment.arguments {
                for (index, argument) in source_arguments.iter().enumerate() {
                    match argument {
                        crate::ir::PathArgumentIr::Type(ty) => {
                            let parameter = quote!(definition.generic_parameter_at(#index));
                            let expression = super::traits::type_expression(ty, &environment, &facade);
                            let concrete_type = ty.tokens.clone();
                            arguments.push(quote!(#codegen::expression::GenericArgumentMetadata::type_argument::<#concrete_type>(#parameter, #expression)));
                            identity.push(quote!(#codegen::traits::ArgumentIdentity::type_argument::<#concrete_type>()));
                        }
                        crate::ir::PathArgumentIr::Const(value) => {
                            let value = super::expression_codegen::const_expression(value, &environment, &facade);
                            let parameter = quote!(definition.generic_parameter_at(#index));
                            arguments.push(quote!(#codegen::expression::GenericArgumentMetadata::const_argument(#parameter, #value)));
                            identity.push(quote!(#codegen::traits::ArgumentIdentity::const_argument(#value)));
                        }
                        crate::ir::PathArgumentIr::Lifetime(value) => {
                            let value = syn::LitStr::new(value, declaration.span);
                            identity.push(quote!(#codegen::traits::ArgumentIdentity::lifetime(#value)));
                        }
                        _ => {}
                    }
                }
            }
            (arguments, identity)
        })
        .unwrap_or_default();
    let trait_application_provider = match (
        trait_definition_provider.as_ref(),
        trait_marker_path.as_ref(),
    ) {
        (Some(provider), Some(trait_marker)) => Some(quote! {
            fn trait_application_provider() -> &'static #codegen::traits::TraitApplicationSpec<#target> {
                static VALUE: ::std::sync::OnceLock<#codegen::traits::TraitApplicationSpec<#target>> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| {
                    let definition = #provider();
                    let arguments = ::std::boxed::Box::leak(::std::vec![#(#trait_application_arguments),*].into_boxed_slice());
                    let identity = ::std::boxed::Box::leak(::std::vec![#(#trait_argument_identity),*].into_boxed_slice());
                    #codegen::traits::TraitApplicationSpec::for_target::<#trait_marker>(
                        definition,
                        arguments,
                        identity,
                        &[],
                        &[],
                        #codegen::traits::SupertraitStatus::Incomplete,
                    )
                })
            }
        }),
        _ => None,
    };
    let implemented_trait = match (
        trait_definition_provider.as_ref(),
        declaration.trait_path.as_ref(),
    ) {
        (Some(provider), Some(path)) => {
            let name = syn::LitStr::new(&path.source, path.span);
            let expression_args = path.segments.iter().flat_map(|segment| {
                super::expression_codegen::path_arguments(
                    &segment.arguments,
                    &environment,
                    &facade,
                    path.span,
                )
            });
            let expression = quote!(::std::boxed::Box::leak(::std::boxed::Box::new(
                #codegen::expression::TraitExpressionData::new(
                    #name,
                    ::std::boxed::Box::leak(::std::vec![#(#expression_args),*].into_boxed_slice()),
                    &[],
                ),
            )));
            quote!(Some(::std::boxed::Box::leak(::std::boxed::Box::new(
                #codegen::traits::trait_expression_for(#expression, #provider()),
            ))))
        }
        _ => quote!(None),
    };
    let associated_type_bindings: Vec<_> = declaration
        .associated_types
        .iter()
        .filter_map(|item| {
            let value = item.value.as_ref()?;
            let (Some(provider), Some(_)) = (
                trait_definition_provider.as_ref(),
                declaration.trait_path.as_ref(),
            ) else {
                return None;
            };
            let name = syn::LitStr::new(&item.name.to_string(), item.span);
            let expression = super::traits::type_expression(value, &environment, &facade);
            Some(quote!(#codegen::impls::associated_type_binding(
                #provider().associated_type_named(#name),
                #expression,
                #codegen::impls::ItemSource::Overridden,
            )))
        })
        .collect();
    let associated_const_bindings: Vec<_> = declaration
        .associated_consts
        .iter()
        .filter_map(|item| {
            let value = item.value.as_ref()?;
            let (Some(provider), Some(_)) = (
                trait_definition_provider.as_ref(),
                declaration.trait_path.as_ref(),
            ) else {
                return None;
            };
            let name = syn::LitStr::new(&item.name.to_string(), item.span);
            let expression =
                super::expression_codegen::const_expression(value, &environment, &facade);
            Some(quote!(#codegen::impls::associated_const_binding(
                #provider().associated_const_named(#name),
                #expression,
                #codegen::impls::ItemSource::Overridden,
            )))
        })
        .collect();
    let adapter_definitions: Vec<_> = declaration
        .methods
        .iter()
        .enumerate()
        .filter_map(|(index, method)| {
            invocation_adapter::definition(
                method,
                index,
                &target,
                &declaration
                    .trait_path
                    .as_ref()
                    .map(|path| path.tokens.clone()),
                &declaration.target_type.source,
                &facade,
            )
        })
        .collect();
    let adapter_slots: Vec<_> = declaration
        .methods
        .iter()
        .enumerate()
        .filter_map(|(index, method)| {
            let mut method_context =
                super::invocation::analysis::MethodContext::implementation(&target);
            method_context.extension_receiver = method
                .receiver
                .as_ref()
                .and_then(|receiver| typed_extension_receiver_type(receiver, &target));
            let plan = super::invocation::analysis::analyze_method(method, method_context)
                .expect("validated method analysis is infallible");
            if !plan.is_executable() {
                return None;
            }
            let name = format_ident!("__QUBIT_REFLECT_INVOCATION_ADAPTER_{index}");
            let slot = if plan.pinned_receiver_mutability().is_some() {
                if plan.modes.thread_safe {
                    quote!(.pinned_thread_safe::<#target>(&#name))
                } else {
                    quote!(.pinned_local::<#target>(&#name))
                }
            } else if plan.modes.thread_safe {
                quote!(.thread_safe(&#name))
            } else {
                quote!(.local(&#name))
            };
            Some((index, slot))
        })
        .collect();
    let effective_methods: Vec<_> = declaration
        .methods
        .iter()
        .enumerate()
        .map(|(index, method)| {
            let slots = adapter_slots
                .iter()
                .filter(|(slot_index, _)| *slot_index == index)
                .map(|(_, slot)| slot)
                .collect::<Vec<_>>();
            let adapters = if slots.is_empty() {
                quote!()
            } else {
                quote!(.with_adapters(#codegen::invoke::InvocationAdapters::builder()#(#slots)*.build()))
            };
            let mut method_context =
                super::invocation::analysis::MethodContext::implementation(&target);
            method_context.extension_receiver = method
                .receiver
                .as_ref()
                .and_then(|receiver| typed_extension_receiver_type(receiver, &target));
            let plan = super::invocation::analysis::analyze_method(method, method_context)
                .expect("validated method analysis is infallible");
            let availability = if slots.is_empty() {
                let reasons = if plan.is_executable() {
                    quote!(::std::boxed::Box::leak(::std::boxed::Box::new([
                        #codegen::invoke::InvocationUnavailableReason::AdapterUnavailable,
                    ])))
                } else {
                    super::invocation::emit::emit_unavailable_reasons(&plan, context)
                };
                quote!(.with_unavailable(#reasons))
            } else {
                quote!()
            };
            let source = if declaration.trait_path.is_some() {
                quote!(#codegen::invoke::MethodImplementationSource::Overridden)
            } else {
                quote!(#codegen::invoke::MethodImplementationSource::Declared)
            };
            quote!(#codegen::invoke::EffectiveMethod::for_type::<#target>(&methods[#index])
            #adapters
            #availability
            .with_source(#source))
        })
        .collect();
    let retained = declaration.retained_tokens;
    let registration = if trait_application_provider.is_some() {
        quote! {
            #codegen::traits::submit_trait_application!(#target, trait_application_provider);
            #codegen::impls::submit_impl_definition!(#marker, impl_definition_provider);
            #codegen::impls::submit_impl_application!(#target, impl_application_provider);
        }
    } else {
        quote! {
            #codegen::impls::submit_impl_definition!(#marker, impl_definition_provider);
            #codegen::impls::submit_impl_application!(#target, impl_application_provider);
        }
    };
    let trait_application = if declaration.trait_path.is_some() {
        quote!(Some(trait_application_provider().metadata()))
    } else {
        quote!(None)
    };
    let tokens = quote! {
        #retained

        #[doc(hidden)]
        mod #module {
            use super::*;
            struct #marker;
            #(#adapter_definitions)*
            #trait_application_provider

            fn methods() -> &'static [#codegen::invoke::MethodMetadata] {
                static VALUE: ::std::sync::OnceLock<::std::boxed::Box<[#codegen::invoke::MethodMetadata]>> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| ::std::vec![#(#methods),*].into_boxed_slice())
            }
            fn associated_types() -> &'static [#codegen::traits::AssociatedType] {
                static VALUE: ::std::sync::OnceLock<::std::boxed::Box<[#codegen::traits::AssociatedType]>> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| ::std::vec![#(#associated_types),*].into_boxed_slice())
            }
            fn associated_consts() -> &'static [#codegen::traits::AssociatedConst] {
                static VALUE: ::std::sync::OnceLock<::std::boxed::Box<[#codegen::traits::AssociatedConst]>> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| ::std::vec![#(#associated_consts),*].into_boxed_slice())
            }
            fn generic_definition() -> &'static #codegen::expression::GenericDefinitionMetadata {
                static VALUE: ::std::sync::OnceLock<#codegen::expression::GenericDefinitionMetadata> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| #generic_definition)
            }
            fn impl_definition_provider() -> &'static #codegen::impls::ImplDefinitionSpec<#marker> {
                static VALUE: ::std::sync::OnceLock<#codegen::impls::ImplDefinitionSpec<#marker>> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| #codegen::impls::ImplDefinitionSpec::new(
                    #kind,
                    #target_expression,
                    #implemented_trait,
                    generic_definition(),
                    #codegen::impls::Qualifiers::new(false),
                    methods(),
                    associated_types(),
                    associated_consts(),
                ))
            }
            fn effective_methods() -> &'static [#codegen::invoke::EffectiveMethod] {
                static VALUE: ::std::sync::OnceLock<::std::boxed::Box<[#codegen::invoke::EffectiveMethod]>> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| {
                    let methods = methods();
                    let mut effective_methods: ::std::vec::Vec<#codegen::invoke::EffectiveMethod> =
                        ::std::vec![#(#effective_methods),*];
                    effective_methods.extend(#default_methods.into_vec());
                    effective_methods.into_boxed_slice()
                })
            }
            fn impl_application_provider() -> &'static #codegen::impls::ImplApplicationSpec<#target> {
                static VALUE: ::std::sync::OnceLock<#codegen::impls::ImplApplicationSpec<#target>> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| #codegen::impls::ImplApplicationSpec::for_target::<#marker>(
                    impl_definition_provider(),
                    #trait_application,
                    &[],
                    &[],
                    effective_methods(),
                    ::std::boxed::Box::leak(::std::vec![#(#associated_type_bindings),*].into_boxed_slice()),
                    ::std::boxed::Box::leak(::std::vec![#(#associated_const_bindings),*].into_boxed_slice()),
                ))
            }
            #registration
        }
    };
    super::invocation::emit::rewrite_codegen_paths(tokens)
}

// Generic specialization generation lives in `impls::specialization_codegen`.
