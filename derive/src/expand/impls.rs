// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Expansion of distributed registration fragments for `#[reflect_impl]`.

mod specialization_codegen;

use specialization_codegen::simple_generic_specialization_adapter;
use specialization_codegen::specialization_arguments;
use specialization_codegen::specialization_associated_type_resolver_arms;
use specialization_codegen::specialization_replacements;
use specialization_codegen::substitute_impl_associated_item_types;
use specialization_codegen::substitute_impl_lifetimes;
use specialization_codegen::substitute_impl_method_types;
use specialization_codegen::substitute_tokens;
use specialization_codegen::substitute_type_tokens;
use specialization_codegen::typed_extension_receiver_type;

use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;

use crate::expand::ExpansionContext;
use crate::ir::GenericKindIr;
use crate::ir::HelperName;
use crate::ir::HelperValueIr;
use crate::ir::ImplDeclarationIr;
use crate::ir::MethodIr;
use crate::ir::ParameterPatternKindIr;
use crate::ir::ReceiverKindIr;
use crate::ir::ReturnTypeIr;
use crate::ir::TypeIr;
use crate::ir::TypeKindIr;
use crate::ir::VisibilityIr;

/// Expands an impl unchanged and submits a lazily-built implementation
/// fragment.
///
/// The descriptor graph is deliberately constructed during registry
/// initialization, not from an inventory constructor. This keeps user code out
/// of linker startup and uses the T12 registration protocol exclusively.
pub(crate) fn expand_impl(
    declaration: ImplDeclarationIr,
    context: &ExpansionContext,
) -> TokenStream {
    if !declaration.generics.params.is_empty() {
        return expand_generic_impl_specializations(declaration, context);
    }
    expand_concrete_impl(
        declaration,
        quote!(::std::vec![]),
        None,
        None,
        Vec::new(),
        context.facade(),
        context,
    )
}

/// Retains a generic impl and emits one concrete registration fragment for
/// every explicit specialization.
///
/// Blanket and constrained impls have no finite runtime `TypeId` set, so an
/// impl without `specialize(...)` deliberately remains descriptor-only.
fn expand_generic_impl_specializations(
    declaration: ImplDeclarationIr,
    context: &ExpansionContext,
) -> TokenStream {
    let retained = declaration.retained_tokens.clone();
    let facade = context.facade().clone();
    let definition = expand_generic_impl_definition(&declaration, &facade, context);
    if declaration.specializations.is_empty() {
        return quote!(#retained #definition);
    }
    let shared_definition = generic_impl_definition_module(&declaration);
    let fragments = declaration.specializations.iter().map(|specialization| {
        let specialization_arguments =
            specialization_arguments(specialization, &declaration.generics, &facade);
        let arguments = quote!(#specialization_arguments.into_vec());
        let replacements = specialization_replacements(specialization);
        let associated_type_resolver_arms =
            specialization_associated_type_resolver_arms(&declaration, &replacements, &facade);
        let lifetime_names: Vec<_> = declaration
            .generics
            .params
            .iter()
            .filter(|parameter| parameter.kind == GenericKindIr::Lifetime)
            .map(|parameter| parameter.name.as_str())
            .collect();
        let mut concrete = declaration.clone();
        concrete.retained_tokens = TokenStream::new();
        concrete.specializations.clear();
        substitute_impl_lifetimes(&mut concrete, &lifetime_names);
        substitute_type_tokens(&mut concrete.target_type, &replacements);
        if let Some(trait_path) = &mut concrete.trait_path {
            trait_path.tokens = substitute_tokens(&trait_path.tokens, &replacements);
            trait_path.source = trait_path.tokens.to_string();
        }
        substitute_impl_method_types(&mut concrete, &replacements);
        substitute_impl_associated_item_types(&mut concrete, &replacements);
        let applicability_witness =
            impl_specialization_applicability_witness(&declaration, &concrete.target_type.tokens);
        let definition = quote!(super::#shared_definition::definition());
        expand_concrete_impl(
            concrete,
            arguments,
            Some(definition),
            Some(applicability_witness),
            associated_type_resolver_arms,
            &facade,
            context,
        )
    });
    quote!(#retained #definition #(#fragments)*)
}

/// Emits one declaration-level registration for a generic inherent impl.
///
/// This payload deliberately has no `TypeId`: it describes the symbolic impl
/// declaration and cannot enter a concrete type's effective view.
fn expand_generic_impl_definition(
    declaration: &ImplDeclarationIr,
    facade: &TokenStream,
    context: &ExpansionContext,
) -> TokenStream {
    let fingerprint = context.fingerprint(&declaration.retained_tokens.to_string());
    let location = declaration.span.start();
    let line = location.line as u32;
    let column = location.column as u32;
    let module = generic_impl_definition_module(declaration);
    let target = super::traits::type_expression(&declaration.target_type, facade);
    let generics =
        super::traits::generic_definition(&declaration.generics, declaration.span, facade);
    let methods = definition_method_entries(declaration, facade);
    let associated_types = declaration.associated_types.iter().map(|item| {
        let rust_name = syn::LitStr::new(&item.name.to_string(), item.span);
        quote!(#facade::descriptor::ImplAssociatedTypeDescriptor::new(#rust_name))
    });
    let associated_consts = declaration.associated_consts.iter().map(|item| {
        let rust_name = syn::LitStr::new(&item.name.to_string(), item.span);
        let declared_type = super::traits::type_expression(&item.ty, facade);
        quote!(#facade::descriptor::ImplAssociatedConstDescriptor::new(
            #rust_name,
            #declared_type,
        ))
    });
    let external_id = declaration
        .attributes
        .iter()
        .find_map(|attribute| match &attribute.value {
            HelperValueIr::ExternalTraitId(value) => Some(value.as_str()),
            _ => None,
        });
    let definition_constructor = if let Some(trait_path) = &declaration.trait_path {
        let path = trait_path
            .segments
            .iter()
            .map(|segment| segment.name.as_str())
            .collect::<Vec<_>>()
            .join("::");
        let path = syn::LitStr::new(&path, declaration.span);
        let trait_id = match external_id {
            Some(id) => quote!(Some(#facade::descriptor::TraitId::External(
                #facade::identity::ExternalTraitId::new(#id)
                    .expect("validated external trait ID"),
            ))),
            None => quote!(None),
        };
        quote! {
            #facade::descriptor::ImplDefinitionDescriptor::new_unresolved_trait(
                fragment_identity(),
                #target,
                #path,
                #trait_id,
                ::std::boxed::Box::leak(::std::boxed::Box::new(#generics)),
            )
        }
    } else {
        quote! {
            #facade::descriptor::ImplDefinitionDescriptor::new(
                fragment_identity(),
                #target,
                #facade::descriptor::ImplKind::Inherent,
                None,
                ::std::boxed::Box::leak(::std::boxed::Box::new(#generics)),
            ).expect("generated generic impl definition is consistent")
        }
    };
    let external_registration = match (declaration.trait_path.as_ref(), external_id) {
        (Some(path), Some(id)) => {
            let path = path.tokens.clone();
            quote! {
                fn external_trait_runtime_identity() -> #facade::__private::codegen_v1::registration::RuntimeIdentity {
                    #facade::__private::codegen_v1::registration::RuntimeIdentity::Trait(
                        #facade::descriptor::TraitId::External(
                            #facade::identity::ExternalTraitId::new(#id)
                                .expect("validated external trait ID"),
                        ),
                    )
                }

                fn external_trait_payload() -> #facade::__private::codegen_v1::registration::FragmentPayload {
                    static TRAIT: ::std::sync::OnceLock<#facade::descriptor::TraitDefinitionDescriptor> =
                        ::std::sync::OnceLock::new();
                    let descriptor = TRAIT.get_or_init(|| {
                        #facade::descriptor::TraitDefinitionDescriptor::new(
                            #facade::descriptor::TraitId::External(
                                #facade::identity::ExternalTraitId::new(#id)
                                    .expect("validated external trait ID"),
                            ),
                            stringify!(#path),
                            stringify!(#path),
                            stringify!(#path),
                            #facade::descriptor::TraitCompleteness::ExternalIncomplete,
                            ::std::boxed::Box::leak(::std::boxed::Box::new(
                                #facade::expression::GenericDefinitionDescriptor::new(
                                    ::std::vec::Vec::<#facade::expression::GenericParameterDescriptor>::new(),
                                    ::std::vec::Vec::<#facade::expression::PredicateDescriptor>::new(),
                                ),
                            )),
                        )
                    });
                    #facade::__private::codegen_v1::registration::FragmentPayload::Trait(descriptor)
                }

                #facade::__private::codegen_v1::inventory::submit! {
                    #facade::__private::codegen_v1::registration::RegistrationFragment::new(
                        #facade::__private::codegen_v1::registration::FragmentKind::Trait,
                        #facade::__private::codegen_v1::registration::StaticFragmentIdentity::new(
                            env!("CARGO_PKG_NAME"), module_path!(), #line, #column,
                            "external-trait", #fingerprint ^ 0x9e3779b97f4a7c15_u64,
                        ),
                        external_trait_runtime_identity,
                        external_trait_payload,
                    )
                }
            }
        }
        _ => TokenStream::new(),
    };
    quote! {
        #[doc(hidden)]
        mod #module {
            use super::*;

            fn fragment_identity() -> #facade::identity::FragmentIdentity {
                #facade::identity::FragmentIdentity::new(
                    env!("CARGO_PKG_NAME"), module_path!(), #line, #column,
                    "impl-definition", #fingerprint,
                )
            }

            pub(super) fn definition() -> &'static #facade::descriptor::ImplDefinitionDescriptor {
                static DEFINITION: ::std::sync::OnceLock<#facade::descriptor::ImplDefinitionDescriptor> =
                    ::std::sync::OnceLock::new();
                let definition = DEFINITION.get_or_init(|| #definition_constructor);
                definition.initialize_methods(|definition| {
                    ::std::vec![#(#methods),*].into_boxed_slice()
                });
                definition.initialize_associated_items(|_| (
                    ::std::vec![#(#associated_types),*].into_boxed_slice(),
                    ::std::vec![#(#associated_consts),*].into_boxed_slice(),
                ));
                definition
            }

            fn runtime_identity() -> #facade::__private::codegen_v1::registration::RuntimeIdentity {
                #facade::__private::codegen_v1::registration::RuntimeIdentity::ImplDefinition(
                    fragment_identity(),
                )
            }

            fn payload() -> #facade::__private::codegen_v1::registration::FragmentPayload {
                #facade::__private::codegen_v1::registration::FragmentPayload::ImplDefinition(definition())
            }

            #facade::__private::codegen_v1::inventory::submit! {
                #facade::__private::codegen_v1::registration::RegistrationFragment::new(
                    #facade::__private::codegen_v1::registration::FragmentKind::ImplDefinition,
                    #facade::__private::codegen_v1::registration::StaticFragmentIdentity::new(
                        env!("CARGO_PKG_NAME"), module_path!(), #line, #column,
                        "impl-definition", #fingerprint,
                    ),
                    runtime_identity,
                    payload,
                )
            }

            #external_registration
        }
    }
}

/// Returns the stable generated module name owning one generic impl
/// definition.
fn generic_impl_definition_module(declaration: &ImplDeclarationIr) -> Ident {
    let fingerprint = super::context::fingerprint(&declaration.retained_tokens.to_string());
    let location = declaration.span.start();
    format_ident!(
        "__qubit_reflect_impl_definition_{fingerprint:x}_{}_{}",
        location.line,
        location.column,
    )
}

/// Emits a private trait witness whose concrete use proves that the selected
/// specialization satisfies the original impl header and where predicates.
fn impl_specialization_applicability_witness(
    declaration: &ImplDeclarationIr,
    concrete_target: &TokenStream,
) -> TokenStream {
    let impl_declaration = &declaration.generics.impl_declaration;
    let where_clause = &declaration.generics.where_clause;
    let symbolic_target = &declaration.target_type.tokens;
    quote! {
        trait __QubitReflectImplSpecializationApplies {}

        impl #impl_declaration __QubitReflectImplSpecializationApplies for #symbolic_target
        #where_clause
        {}

        const _: fn() = || {
            fn assert_applies<T: __QubitReflectImplSpecializationApplies>() {}
            assert_applies::<#concrete_target>();
        };
    }
}

/// Builds declaration-level method descriptors without concrete adapters or
/// instances.
fn definition_method_entries(
    declaration: &ImplDeclarationIr,
    facade: &TokenStream,
) -> Vec<TokenStream> {
    let target_source = declaration.target_type.source.as_str();
    declaration
        .methods
        .iter()
        .enumerate()
        .map(|(index, method)| {
            let rust_name = method.name.to_string();
            let query_name = method
                .attributes
                .iter()
                .find(|attribute| attribute.name == HelperName::Rename)
                .and_then(|attribute| attribute.rename())
                .unwrap_or(&rust_name)
                .to_owned();
            let receiver = match &method.receiver {
                Some(receiver) => match receiver.kind {
                    ReceiverKindIr::Value => quote!(Some(#facade::descriptor::ReceiverDescriptor::Owned)),
                    ReceiverKindIr::SharedReference => quote!(Some(#facade::descriptor::ReceiverDescriptor::Shared)),
                    ReceiverKindIr::MutableReference => quote!(Some(#facade::descriptor::ReceiverDescriptor::Mutable)),
                    ReceiverKindIr::Typed => {
                        let value = syn::LitStr::new(&receiver.declaration.to_string(), receiver.span);
                        quote!(Some(#facade::descriptor::ReceiverDescriptor::Explicit(#value)))
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
                    Some(value) => quote!(Some(#value)),
                    None => quote!(None),
                };
                let pattern = match parameter.pattern.kind {
                    ParameterPatternKindIr::Identifier => {
                        quote!(#facade::descriptor::ParameterPatternDescriptor::Identifier)
                    }
                    ParameterPatternKindIr::Wildcard => {
                        quote!(#facade::descriptor::ParameterPatternDescriptor::Wildcard)
                    }
                    ParameterPatternKindIr::Destructure => {
                        let source = syn::LitStr::new(&parameter.pattern.source, parameter.span);
                        quote!(#facade::descriptor::ParameterPatternDescriptor::Destructure(#source.into()))
                    }
                };
                let passing = match &parameter.ty.kind {
                    TypeKindIr::Reference { mutable: true, .. } => {
                        quote!(#facade::descriptor::ParameterPassingMode::MutableBorrow)
                    }
                    TypeKindIr::Reference { .. } => {
                        quote!(#facade::descriptor::ParameterPassingMode::SharedBorrow)
                    }
                    _ => quote!(#facade::descriptor::ParameterPassingMode::Owned),
                };
                let ty = super::traits::type_expression(&parameter.ty, facade);
                let parameter_index = parameter.index;
                quote!(#facade::descriptor::ParameterDescriptor::new(
                    #parameter_index, #name, #pattern, #passing, #ty, None,
                ))
            });
            let return_value = match &method.return_type {
                ReturnTypeIr::Unit => quote!(#facade::descriptor::ReturnDescriptor::unit()),
                ReturnTypeIr::Type(ty) => {
                    let expression = super::traits::type_expression(ty, facade);
                    let kind = match &ty.kind {
                        TypeKindIr::Never => quote!(#facade::descriptor::ReturnKind::Never),
                        TypeKindIr::Reference { .. } => quote!(#facade::descriptor::ReturnKind::Reference),
                        TypeKindIr::ImplTrait { .. } => quote!(#facade::descriptor::ReturnKind::Opaque),
                        _ => quote!(#facade::descriptor::ReturnKind::Concrete),
                    };
                    quote!(#facade::descriptor::ReturnDescriptor::new(#kind, Some(#expression), None))
                }
            };
            let visibility = match &method.visibility {
                VisibilityIr::Public => quote!(#facade::descriptor::MethodVisibility::Declared(
                    #facade::identity::Visibility::Public,
                )),
                VisibilityIr::Crate => quote!(#facade::descriptor::MethodVisibility::Declared(
                    #facade::identity::Visibility::Crate,
                )),
                VisibilityIr::Super => quote!(#facade::descriptor::MethodVisibility::Declared(
                    #facade::identity::Visibility::Super,
                )),
                VisibilityIr::Restricted(path) => {
                    let path = syn::LitStr::new(&path.source, method.span);
                    quote!(#facade::descriptor::MethodVisibility::Declared(
                        #facade::identity::Visibility::Restricted(#path.into()),
                    ))
                }
                VisibilityIr::SelfValue | VisibilityIr::Inherited => {
                    quote!(#facade::descriptor::MethodVisibility::Declared(
                        #facade::identity::Visibility::Private,
                    ))
                }
            };
            let generic_definition = super::traits::generic_definition(&method.generics, method.span, facade);
            let qualifiers = &method.qualifiers;
            let is_async = qualifiers.is_async;
            let is_unsafe = qualifiers.is_unsafe;
            let is_const = qualifiers.is_const;
            let is_variadic = qualifiers.is_variadic;
            let abi = qualifiers.abi.as_deref().map(|abi| syn::LitStr::new(abi, method.span));
            let abi = match abi {
                Some(value) => quote!(Some(#facade::expression::FunctionAbi::Other(#value.into()))),
                None => quote!(None),
            };
            quote! {
                #facade::descriptor::MethodDescriptor::builder(
                    #facade::identity::MemberId::new(
                        #target_source, "method", #index, fragment_identity(),
                    ),
                    #rust_name,
                    #query_name,
                    #facade::descriptor::MethodDeclarationOwner::Impl(definition),
                )
                .visibility(#visibility)
                .receiver(#receiver)
                .parameters(::std::vec![#(#parameters),*])
                .return_value(#return_value)
                .qualifiers(#facade::descriptor::MethodQualifiers::new(
                    #is_async, #is_unsafe, #is_const, #abi, #is_variadic,
                ))
                .generic_definition(&#generic_definition)
                .build()
            }
        })
        .collect()
}

/// Expands one concrete implementation registration fragment.
fn expand_concrete_impl(
    declaration: ImplDeclarationIr,
    impl_arguments: TokenStream,
    shared_definition: Option<TokenStream>,
    applicability_witness: Option<TokenStream>,
    specialized_associated_type_resolver_arms: Vec<TokenStream>,
    facade: &TokenStream,
    context: &ExpansionContext,
) -> TokenStream {
    let facade = facade.clone();
    let retained = declaration.retained_tokens;
    let target = declaration.target_type.tokens;
    let impl_generic_definition =
        super::traits::generic_definition(&declaration.generics, declaration.span, &facade);
    let trait_path = declaration
        .trait_path
        .as_ref()
        .map(|path| path.tokens.clone());
    let trait_call_path = trait_path.clone();
    let has_trait = trait_path.is_some();
    let external_id = declaration
        .attributes
        .iter()
        .find_map(|attribute| match &attribute.value {
            HelperValueIr::ExternalTraitId(value) => Some(value.as_str()),
            _ => None,
        });
    let fingerprint = context.fingerprint(&format!("{}{}", retained, target));
    let location = declaration.span.start();
    let line = location.line as u32;
    let column = location.column as u32;
    let module = format_ident!("__qubit_reflect_impl_{fingerprint:x}_{line}_{column}");
    let target_source = declaration.target_type.source;
    let invocation_adapter_definitions = declaration.methods.iter().enumerate().filter_map(|(index, method)| {
        let typed_extension_receiver = method
            .receiver
            .as_ref()
            .and_then(|receiver| typed_extension_receiver_type(receiver, &target));
        let mut method_context = super::invocation::analysis::MethodContext::implementation(&target);
        method_context.extension_receiver = typed_extension_receiver.clone();
        let invocation_plan = super::invocation::analysis::analyze_method(method, method_context)
            .expect("validated method analysis is infallible");
        debug_assert_eq!(invocation_plan.parameter_count(), method.parameters.len());
        let typed_owned_receiver = invocation_plan.owned_receiver_type().cloned();
        let typed_extension_receiver = invocation_plan.extension_receiver_type().cloned();
        let is_safe_invocation = invocation_plan.is_executable();
        let pinned_receiver_mutable = invocation_plan.pinned_receiver_mutability();
        if let Some(pinned_mutable) = pinned_receiver_mutable {
            let is_safe_pinned_invocation = invocation_plan.is_executable();
            if is_safe_pinned_invocation {
                let method_name = &method.name;
                let adapter_name = format_ident!("__qubit_reflect_invoke_pinned_{index}");
                let adapter_token_name = format_ident!("__QUBIT_REFLECT_PINNED_ADAPTER_{index}");
                let descriptor_name = format_ident!("__QUBIT_REFLECT_INVOCATION_ADAPTER_{index}");
                let parameter_expectations: Vec<_> = method
                    .parameters
                    .iter()
                    .map(|parameter| super::invocation::emit::argument_expectation(parameter, &facade))
                    .collect();
                let mode = quote!(#facade::value::Local);
                let argument_bindings: Vec<_> = method
                    .parameters
                    .iter()
                    .map(|parameter| super::invocation::emit::argument_binding(parameter, &facade, &mode))
                    .collect();
                let call_arguments: Vec<_> = method
                    .parameters
                    .iter()
                    .map(|parameter| format_ident!("__qubit_reflect_argument_{}", parameter.index))
                    .collect();
                let invocation_type = if pinned_mutable {
                    quote!(#facade::invoke::PinnedMutInvocation<'call, #target, #facade::value::Local>)
                } else {
                    quote!(#facade::invoke::PinnedRefInvocation<'call, #target, #facade::value::Local>)
                };
                let failure_type = if pinned_mutable {
                    quote!(#facade::invoke::PinnedMutInvocationFailure<'call, #target, #facade::value::Local>)
                } else {
                    quote!(#facade::invoke::PinnedRefInvocationFailure<'call, #target, #facade::value::Local>)
                };
                let adapter_type = if pinned_mutable {
                    quote!(#facade::invoke::PinnedMutAdapter<#target, #facade::value::Local>)
                } else {
                    quote!(#facade::invoke::PinnedRefAdapter<#target, #facade::value::Local>)
                };
                let constructor = if pinned_mutable {
                    quote!(#facade::descriptor::InvocationAdapter::pinned_mut_local(&#adapter_token_name))
                } else {
                    quote!(#facade::descriptor::InvocationAdapter::pinned_ref_local(&#adapter_token_name))
                };
                let call = if let Some(trait_path) = &trait_call_path {
                    quote!(<#target as #trait_path>::#method_name(receiver, #(#call_arguments),*))
                } else {
                    quote!(<#target>::#method_name(receiver, #(#call_arguments),*))
                };
                let output = match method.return_type {
                    ReturnTypeIr::Unit => quote! {
                        #call;
                        #facade::invoke::InvocationOutput::Unit
                    },
                    ReturnTypeIr::Type(TypeIr {
                        kind: TypeKindIr::Never,
                        ..
                    }) => {
                        quote!(match #call {})
                    }
                    ReturnTypeIr::Type(_) => quote! {
                        #facade::invoke::InvocationOutput::Owned(
                            #facade::value::DynamicOwned::<#facade::value::Local>::new(
                                #call,
                            ),
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
                return Some(quote! {
                    fn #adapter_name<'call>(
                        invocation: #invocation_type,
                    ) -> ::core::result::Result<
                        #facade::invoke::InvocationOutput<'call, #facade::value::Local>,
                        #failure_type,
                    > {
                        let identity = #facade::identity::MemberId::new(
                            #target_source, "method", #index, fragment_identity(),
                        );
                        let validated = invocation.validate(&identity, &[#(#parameter_expectations),*])?;
                        let (receiver, arguments) = validated.into_parts();
                        let mut arguments = arguments.into_vec().into_iter();
                        #(#argument_bindings)*
                        #invocation_result
                    }

                    static #adapter_token_name: #adapter_type = #adapter_name;
                    static #descriptor_name: #facade::descriptor::InvocationAdapter = #constructor;
                });
            }
        }
        if pinned_receiver_mutable.is_some() {
            return None;
        }
        if !is_safe_invocation {
            return None;
        }
        let method_name = &method.name;
        let adapter_name = format_ident!("__qubit_reflect_invoke_{index}");
        let catching_adapter_name = format_ident!("__qubit_reflect_invoke_catching_{index}");
        let descriptor_name = format_ident!("__QUBIT_REFLECT_INVOCATION_ADAPTER_{index}");
        let thread_safe = invocation_plan.modes.thread_safe;
        let catching_requested = invocation_plan.modes.catching;
        let mode = if thread_safe {
            quote!(#facade::value::ThreadSafe)
        } else {
            quote!(#facade::value::Local)
        };
        let thread_safe_assertions = thread_safe.then(|| {
            super::invocation::emit::thread_safe_assertions(
                method,
                &target,
                typed_owned_receiver.as_ref(),
                typed_extension_receiver.as_ref(),
            )
        });
        let catching_assertions = catching_requested.then(|| {
            super::invocation::emit::catching_assertions(
                method,
                &target,
                typed_owned_receiver.as_ref(),
                typed_extension_receiver.as_ref(),
            )
        });
        let adapter_constructor = if catching_requested && thread_safe {
            quote!(#facade::descriptor::InvocationAdapter::thread_safe_with_catching(
                #adapter_name,
                #catching_adapter_name,
            ))
        } else if catching_requested {
            quote!(#facade::descriptor::InvocationAdapter::local_with_catching(
                #adapter_name,
                #catching_adapter_name,
            ))
        } else if thread_safe {
            quote!(#facade::descriptor::InvocationAdapter::thread_safe(#adapter_name))
        } else {
            quote!(#facade::descriptor::InvocationAdapter::local(#adapter_name))
        };
        let unavailable_catching_constructor = if thread_safe {
            quote!(#facade::descriptor::InvocationAdapter::thread_safe_with_unavailable_catching(
                #adapter_name,
            ))
        } else {
            quote!(#facade::descriptor::InvocationAdapter::local_with_unavailable_catching(
                #adapter_name,
            ))
        };
        let adapter_definition = if catching_requested {
            quote! {
                #[cfg(panic = "unwind")]
                static #descriptor_name: #facade::descriptor::InvocationAdapter =
                    #adapter_constructor;
                #[cfg(panic = "abort")]
                static #descriptor_name: #facade::descriptor::InvocationAdapter =
                    #unavailable_catching_constructor;
            }
        } else {
            quote! {
                static #descriptor_name: #facade::descriptor::InvocationAdapter =
                    #adapter_constructor;
            }
        };
        let receiver_expectation = if matches!(
            method.receiver.as_ref().map(|receiver| receiver.kind),
            Some(ReceiverKindIr::Value)
        ) {
            quote!(#facade::invoke::ReceiverExpectation::owned::<#target>())
        } else if let Some(receiver_type) = &typed_owned_receiver {
            quote!(#facade::invoke::ReceiverExpectation::owned::<#receiver_type>())
        } else if matches!(
            method.receiver.as_ref().map(|receiver| receiver.kind),
            Some(ReceiverKindIr::MutableReference)
        ) {
            quote!(#facade::invoke::ReceiverExpectation::borrowed_mut::<#target>())
        } else if method.receiver.is_some() {
            quote!(#facade::invoke::ReceiverExpectation::borrowed::<#target>())
        } else {
            quote!(#facade::invoke::ReceiverExpectation::none())
        };
        let receiver_binding = if let Some(receiver_type) = &typed_extension_receiver {
            quote! {
                let adapter = <#target as #facade::Reflect>::type_descriptor()
                    .get_capability(#facade::invoke::receiver_adapter_key::<#receiver_type, #mode>());
                let (receiver, arguments) = validated
                    .adapt_receiver::<#receiver_type>(&identity, adapter)?;
            }
        } else if matches!(
            method.receiver.as_ref().map(|receiver| receiver.kind),
            Some(ReceiverKindIr::Value)
        ) {
            quote! {
                let (receiver, arguments) = validated.into_parts();
                let receiver: #target = match receiver {
                    Some(#facade::invoke::InvocationReceiver::Owned(value)) =>
                        #facade::value::DynamicOwned::<#mode>::downcast::<#target>(value)
                            .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                    _ => unreachable!("validation checked receiver mode"),
                };
            }
        } else if let Some(receiver_type) = &typed_owned_receiver {
            quote! {
                let (receiver, arguments) = validated.into_parts();
                let receiver: #receiver_type = match receiver {
                    Some(#facade::invoke::InvocationReceiver::Owned(value)) =>
                        #facade::value::DynamicOwned::<#mode>::downcast::<#receiver_type>(value)
                            .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                    _ => unreachable!("validation checked receiver mode"),
                };
            }
        } else if matches!(
            method.receiver.as_ref().map(|receiver| receiver.kind),
            Some(ReceiverKindIr::MutableReference)
        ) {
            quote! {
                let (receiver, arguments) = validated.into_parts();
                let receiver: &mut #target = match receiver {
                    Some(#facade::invoke::InvocationReceiver::Mut(value)) =>
                        #facade::value::DynamicMut::<#mode>::downcast::<#target>(value)
                            .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                    _ => unreachable!("validation checked receiver mode"),
                };
            }
        } else if method.receiver.is_some() {
            quote! {
                let (receiver, arguments) = validated.into_parts();
                let receiver: &#target = match receiver {
                    Some(#facade::invoke::InvocationReceiver::Ref(value)) =>
                        #facade::value::DynamicRef::<#mode>::downcast::<#target>(value)
                            .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                    Some(#facade::invoke::InvocationReceiver::Mut(value)) => {
                        let value = #facade::value::DynamicMut::<#mode>::downcast::<#target>(value)
                            .unwrap_or_else(|_| unreachable!("validation checked receiver type"));
                        &*value
                    }
                    _ => unreachable!("validation checked receiver mode"),
                };
            }
        } else {
            quote! { let (_receiver, arguments) = validated.into_parts(); }
        };
        let parameter_expectations: Vec<_> = method
            .parameters
            .iter()
            .map(|parameter| super::invocation::emit::argument_expectation(parameter, &facade))
            .collect();
        let invocation_validation = if typed_extension_receiver.is_some() {
            quote!(invocation.validate_arguments(&identity, &[#(#parameter_expectations),*])?)
        } else {
            quote!(invocation.validate(
                    &identity,
                    #receiver_expectation,
                    &[#(#parameter_expectations),*],
                )?)
        };
        let argument_bindings: Vec<_> = method
            .parameters
            .iter()
            .map(|parameter| super::invocation::emit::argument_binding(parameter, &facade, &mode))
            .collect();
        let call_arguments: Vec<_> = method
            .parameters
            .iter()
            .map(|parameter| format_ident!("__qubit_reflect_argument_{}", parameter.index))
            .collect();
        let argument_bindings = &argument_bindings;
        let call_arguments = &call_arguments;
        let call = if let Some(trait_path) = &trait_call_path {
            if method.receiver.is_some() {
                quote!(<#target as #trait_path>::#method_name(receiver, #(#call_arguments),*))
            } else {
                quote!(<#target as #trait_path>::#method_name(#(#call_arguments),*))
            }
        } else if method.receiver.is_some() {
            quote!(<#target>::#method_name(receiver, #(#call_arguments),*))
        } else {
            quote!(<#target>::#method_name(#(#call_arguments),*))
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
                .filter(|parameter| matches!(parameter.ty.kind, TypeKindIr::Reference { .. }))
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
                let value = if super::invocation::analysis::is_str_type(element) {
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
                            mutable: true, element, ..
                        },
                    ..
                }),
            ) => {
                let value = if super::invocation::analysis::is_str_type(element) {
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
        let catching_definition = if catching_requested {
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
                    let value = if super::invocation::analysis::is_str_type(element) {
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
                            mutable: true, element, ..
                        },
                    ..
                }) => {
                    let value = if super::invocation::analysis::is_str_type(element) {
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
                        #facade::value::DynamicOwned::<#mode>::new(
                            #call,
                        ),
                    )
                },
            };
            quote! {
                #[cfg(panic = "unwind")]
                fn #catching_adapter_name<'call>(
                    invocation: #facade::invoke::Invocation<'call, #mode>,
                ) -> ::core::result::Result<
                    ::core::result::Result<
                        #facade::invoke::InvocationOutput<'call, #mode>,
                        #facade::invoke::InvocationPanic,
                    >,
                    #facade::invoke::InvocationFailure<'call, #mode>,
                > {
                    #catching_assertions
                    let identity = #facade::identity::MemberId::new(
                        #target_source, "method", #index, fragment_identity(),
                    );
                    let validated = #invocation_validation;
                    #receiver_binding
                    let mut arguments = arguments.into_vec().into_iter();
                    #(#argument_bindings)*
                    match ::std::panic::catch_unwind(|| { #catching_call }) {
                        Ok(output) => Ok(Ok(output)),
                        Err(payload) => Ok(Err(#facade::invoke::InvocationPanic::new(identity, payload))),
                    }
                }
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
        Some(quote! {
            fn #adapter_name<'call>(
                invocation: #facade::invoke::Invocation<'call, #mode>,
            ) -> ::core::result::Result<
                #facade::invoke::InvocationOutput<'call, #mode>,
                #facade::invoke::InvocationFailure<'call, #mode>,
            > {
                #thread_safe_assertions
                let identity = #facade::identity::MemberId::new(
                    #target_source, "method", #index, fragment_identity(),
                );
                let validated = #invocation_validation;
                #invocation_result
            }

            #catching_definition

            #adapter_definition
        })
    });
    let invocation_adapter_entries = declaration.methods.iter().enumerate().map(|(index, method)| {
        let typed_extension_receiver = method
            .receiver
            .as_ref()
            .and_then(|receiver| typed_extension_receiver_type(receiver, &target));
        let mut method_context = super::invocation::analysis::MethodContext::implementation(&target);
        method_context.extension_receiver = typed_extension_receiver.clone();
        let invocation_plan = super::invocation::analysis::analyze_method(method, method_context)
            .expect("validated method analysis is infallible");
        debug_assert_eq!(invocation_plan.parameter_count(), method.parameters.len());
        let typed_extension_receiver = invocation_plan.extension_receiver_type().cloned();
        if invocation_plan.pinned_receiver_mutability().is_some() {
            let is_safe_pinned_invocation = invocation_plan.is_executable();
            if is_safe_pinned_invocation {
                let descriptor_name = format_ident!("__QUBIT_REFLECT_INVOCATION_ADAPTER_{index}");
                return quote!(Some(&#descriptor_name));
            }
            return quote!(None);
        }
        let is_safe_invocation = invocation_plan.is_executable();
        if is_safe_invocation {
            let descriptor_name = format_ident!("__QUBIT_REFLECT_INVOCATION_ADAPTER_{index}");
            if let Some(receiver_type) = typed_extension_receiver {
                let mode = if method
                    .attributes
                    .iter()
                    .any(|attribute| attribute.name == HelperName::ThreadSafe)
                {
                    quote!(#facade::value::ThreadSafe)
                } else {
                    quote!(#facade::value::Local)
                };
                quote!(
                    if <#target as #facade::Reflect>::type_descriptor()
                        .get_capability(#facade::invoke::receiver_adapter_key::<#receiver_type, #mode>())
                        .is_some()
                    {
                        Some(&#descriptor_name)
                    } else {
                        None
                    }
                )
            } else {
                quote!(Some(&#descriptor_name))
            }
        } else {
            quote!(None)
        }
    });
    let invocation_unavailable_reason_entries: Vec<_> = declaration
        .methods
        .iter()
        .map(|method| invocation_unavailable_reason_entry(method, &target, context))
        .collect();
    let generic_specialization_adapter_definitions = declaration
        .methods
        .iter()
        .enumerate()
        .flat_map(|(method_index, method)| {
            let target = target.clone();
            let target_source = target_source.clone();
            let facade = facade.clone();
            method.specializations.iter().enumerate().filter_map(
                move |(specialization_index, specialization)| {
                    simple_generic_specialization_adapter(
                        method,
                        specialization,
                        &target,
                        &target_source,
                        &facade,
                        method_index,
                        specialization_index,
                    )
                },
            )
        });
    let method_specialization_entries = declaration.methods.iter().enumerate().map(|(method_index, method)| {
        let entries = method
            .specializations
            .iter()
            .enumerate()
            .map(|(specialization_index, specialization)| {
                let arguments = specialization_arguments(specialization, &method.generics, &facade);
                let adapter_name = format_ident!(
                    "__QUBIT_REFLECT_GENERIC_SPECIALIZATION_ADAPTER_{method_index}_{specialization_index}"
                );
                if simple_generic_specialization_adapter(
                    method,
                    specialization,
                    &target,
                    &target_source,
                    &facade,
                    method_index,
                    specialization_index,
                )
                .is_some()
                {
                    quote!((#arguments, Some(&#adapter_name)))
                } else {
                    quote!((#arguments, None))
                }
            });
        quote!(::std::vec![#(#entries),*])
    });
    let method_entries = declaration
        .methods
        .iter()
        .enumerate()
        .map(|(index, method)| {
        let rust_name = method.name.to_string();
        let query_name = method
            .attributes
            .iter()
            .find(|attribute| attribute.name == HelperName::Rename)
            .and_then(|attribute| attribute.rename())
            .unwrap_or(&rust_name)
            .to_owned();
        let receiver = match &method.receiver {
            Some(receiver) => match receiver.kind {
                ReceiverKindIr::Value => quote!(Some(#facade::descriptor::ReceiverDescriptor::Owned)),
                ReceiverKindIr::SharedReference => quote!(Some(#facade::descriptor::ReceiverDescriptor::Shared)),
                ReceiverKindIr::MutableReference => quote!(Some(#facade::descriptor::ReceiverDescriptor::Mutable)),
                ReceiverKindIr::Typed => {
                    let value = syn::LitStr::new(&receiver.declaration.to_string(), receiver.span);
                    quote!(Some(#facade::descriptor::ReceiverDescriptor::Explicit(#value)))
                }
            },
            None => quote!(None),
        };
        let parameters = method.parameters.iter().map(|parameter| {
            let name = parameter.name.as_deref().map(|name| syn::LitStr::new(name, parameter.span));
            let name = match name { Some(value) => quote!(Some(#value)), None => quote!(None) };
            let pattern = match parameter.pattern.kind {
                ParameterPatternKindIr::Identifier => quote!(#facade::descriptor::ParameterPatternDescriptor::Identifier),
                ParameterPatternKindIr::Wildcard => quote!(#facade::descriptor::ParameterPatternDescriptor::Wildcard),
                ParameterPatternKindIr::Destructure => {
                    let source = syn::LitStr::new(&parameter.pattern.source, parameter.span);
                    quote!(#facade::descriptor::ParameterPatternDescriptor::Destructure(#source.into()))
                }
            };
            let passing = match &parameter.ty.kind {
                TypeKindIr::Reference { mutable: true, .. } => quote!(#facade::descriptor::ParameterPassingMode::MutableBorrow),
                TypeKindIr::Reference { .. } => quote!(#facade::descriptor::ParameterPassingMode::SharedBorrow),
                _ => quote!(#facade::descriptor::ParameterPassingMode::Owned),
            };
            let ty = super::traits::type_expression(&parameter.ty, &facade);
            let parameter_index = parameter.index;
            quote!(#facade::descriptor::ParameterDescriptor::new(#parameter_index, #name, #pattern, #passing, #ty, None))
        });
        let return_value = match &method.return_type {
            ReturnTypeIr::Unit => quote!(#facade::descriptor::ReturnDescriptor::unit()),
            ReturnTypeIr::Type(ty) => {
                let expression = super::traits::type_expression(ty, &facade);
                let kind = match &ty.kind {
                    TypeKindIr::Never => quote!(#facade::descriptor::ReturnKind::Never),
                    TypeKindIr::Reference { .. } => quote!(#facade::descriptor::ReturnKind::Reference),
                    TypeKindIr::ImplTrait { .. } => quote!(#facade::descriptor::ReturnKind::Opaque),
                    _ => quote!(#facade::descriptor::ReturnKind::Concrete),
                };
                quote!(#facade::descriptor::ReturnDescriptor::new(#kind, Some(#expression), None))
            }
        };
        let visibility = match &method.visibility {
            VisibilityIr::Public => quote!(#facade::descriptor::MethodVisibility::Declared(#facade::identity::Visibility::Public)),
            VisibilityIr::Crate => quote!(#facade::descriptor::MethodVisibility::Declared(#facade::identity::Visibility::Crate)),
            VisibilityIr::Super => quote!(#facade::descriptor::MethodVisibility::Declared(#facade::identity::Visibility::Super)),
            VisibilityIr::Restricted(path) => { let path = syn::LitStr::new(&path.source, method.span); quote!(#facade::descriptor::MethodVisibility::Declared(#facade::identity::Visibility::Restricted(#path.into()))) },
            VisibilityIr::SelfValue | VisibilityIr::Inherited => quote!(#facade::descriptor::MethodVisibility::Declared(#facade::identity::Visibility::Private)),
        };
        let generic_definition = super::traits::generic_definition(&method.generics, method.span, &facade);
        let qualifiers = &method.qualifiers;
        let is_async = qualifiers.is_async;
        let is_unsafe = qualifiers.is_unsafe;
        let is_const = qualifiers.is_const;
        let is_variadic = qualifiers.is_variadic;
        let abi = qualifiers.abi.as_deref().map(|abi| syn::LitStr::new(abi, method.span));
        let abi = match abi { Some(value) => quote!(Some(#facade::expression::FunctionAbi::Other(#value.into()))), None => quote!(None) };
        quote! {
            #facade::descriptor::MethodDescriptor::builder(
                #facade::identity::MemberId::new(
                    #target_source,
                    "method",
                    #index,
                    fragment_identity(),
                ),
                #rust_name,
                #query_name,
                #facade::descriptor::MethodDeclarationOwner::Impl(definition),
            )
            .visibility(#visibility)
            .receiver(#receiver)
            .parameters(vec![#(#parameters),*])
            .return_value(#return_value)
            .qualifiers(#facade::descriptor::MethodQualifiers::new(
                #is_async, #is_unsafe, #is_const, #abi, #is_variadic,
            ))
            .generic_definition(&#generic_definition)
            .build()
        }
    });
    let external_method_entries = declaration
        .methods
        .iter()
        .enumerate()
        .map(|(index, method)| {
            let rust_name = method.name.to_string();
            let query_name = method
                .attributes
                .iter()
                .find(|attribute| attribute.name == HelperName::Rename)
                .and_then(|attribute| attribute.rename())
                .unwrap_or(&rust_name)
                .to_owned();
            let receiver = match &method.receiver {
                Some(receiver) => match receiver.kind {
                    ReceiverKindIr::Value => quote!(Some(#facade::descriptor::ReceiverDescriptor::Owned)),
                    ReceiverKindIr::SharedReference => quote!(Some(#facade::descriptor::ReceiverDescriptor::Shared)),
                    ReceiverKindIr::MutableReference => quote!(Some(#facade::descriptor::ReceiverDescriptor::Mutable)),
                    ReceiverKindIr::Typed => { let value = syn::LitStr::new(&receiver.declaration.to_string(), receiver.span); quote!(Some(#facade::descriptor::ReceiverDescriptor::Explicit(#value))) }
                },
                None => quote!(None),
            };
            let parameters = method.parameters.iter().map(|parameter| {
                let name = parameter.name.as_deref().map(|name| syn::LitStr::new(name, parameter.span));
                let name = match name { Some(value) => quote!(Some(#value)), None => quote!(None) };
                let pattern = match parameter.pattern.kind {
                    ParameterPatternKindIr::Identifier => quote!(#facade::descriptor::ParameterPatternDescriptor::Identifier),
                    ParameterPatternKindIr::Wildcard => quote!(#facade::descriptor::ParameterPatternDescriptor::Wildcard),
                    ParameterPatternKindIr::Destructure => { let source = syn::LitStr::new(&parameter.pattern.source, parameter.span); quote!(#facade::descriptor::ParameterPatternDescriptor::Destructure(#source.into())) }
                };
                let passing = match &parameter.ty.kind { TypeKindIr::Reference { mutable: true, .. } => quote!(#facade::descriptor::ParameterPassingMode::MutableBorrow), TypeKindIr::Reference { .. } => quote!(#facade::descriptor::ParameterPassingMode::SharedBorrow), _ => quote!(#facade::descriptor::ParameterPassingMode::Owned) };
                let ty = super::traits::type_expression(&parameter.ty, &facade);
                let parameter_index = parameter.index;
                quote!(#facade::descriptor::ParameterDescriptor::new(#parameter_index, #name, #pattern, #passing, #ty, None))
            });
            let return_value = match &method.return_type {
                ReturnTypeIr::Unit => quote!(#facade::descriptor::ReturnDescriptor::unit()),
                ReturnTypeIr::Type(ty) => { let expression = super::traits::type_expression(ty, &facade); let kind = match &ty.kind { TypeKindIr::Never => quote!(#facade::descriptor::ReturnKind::Never), TypeKindIr::Reference { .. } => quote!(#facade::descriptor::ReturnKind::Reference), TypeKindIr::ImplTrait { .. } => quote!(#facade::descriptor::ReturnKind::Opaque), _ => quote!(#facade::descriptor::ReturnKind::Concrete) }; quote!(#facade::descriptor::ReturnDescriptor::new(#kind, Some(#expression), None)) }
            };
            let generic_definition = super::traits::generic_definition(&method.generics, method.span, &facade);
            let qualifiers = &method.qualifiers;
            let is_async = qualifiers.is_async;
            let is_unsafe = qualifiers.is_unsafe;
            let is_const = qualifiers.is_const;
            let is_variadic = qualifiers.is_variadic;
            let abi = qualifiers.abi.as_deref().map(|abi| syn::LitStr::new(abi, method.span));
            let abi = match abi { Some(value) => quote!(Some(#facade::expression::FunctionAbi::Other(#value.into()))), None => quote!(None) };
            quote! {
                #facade::descriptor::MethodDescriptor::builder(
                    #facade::identity::MemberId::new(
                        #target_source, "external-method", #index, fragment_identity(),
                    ),
                    #rust_name, #query_name,
                    #facade::descriptor::MethodDeclarationOwner::Trait(external),
                )
                .visibility(#facade::descriptor::MethodVisibility::InheritedFromTrait)
                .receiver(#receiver)
                .parameters(vec![#(#parameters),*])
                .return_value(#return_value)
                .qualifiers(#facade::descriptor::MethodQualifiers::new(
                    #is_async, #is_unsafe, #is_const, #abi, #is_variadic,
                ))
                .generic_definition(&#generic_definition)
                .build()
            }
        });
    let associated_type_binding_arms: Vec<_> = declaration
        .associated_types
        .iter()
        .filter_map(|item| {
            let value = item.value.as_ref()?;
            let name = syn::LitStr::new(&item.name.to_string(), item.span);
            let value = super::traits::type_expression(value, &facade);
            Some(quote!(#name => #value))
        })
        .collect();
    let associated_const_override_arms: Vec<_> = declaration
        .associated_consts
        .iter()
        .map(|item| {
            let name = syn::LitStr::new(&item.name.to_string(), item.span);
            quote!(#name => #facade::descriptor::AssociatedConstImplementationSource::Overridden)
        })
        .collect();
    let associated_const_reader_override_arms: Vec<_> = trait_path
        .as_ref()
        .map(|path| {
            declaration
                .associated_consts
                .iter()
                .map(|item| {
                    let rust_name = syn::LitStr::new(&item.name.to_string(), item.span);
                    let const_name = &item.name;
                    let qualified_target = quote!(<#target as #path>);
                    let value_type =
                        substitute_tokens(&item.ty.tokens, &[(Ident::new("Self", item.span), qualified_target)]);
                    quote!(
                        #rust_name => Some(
                            #facade::__private::codegen_v1::descriptor::associated_const_reader::<#value_type>(
                                || <#target as #path>::#const_name,
                            ),
                        )
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let kind = if has_trait {
        quote!(#facade::descriptor::ImplKind::Trait)
    } else {
        quote!(#facade::descriptor::ImplKind::Inherent)
    };
    let trait_setup = match (trait_path, external_id) {
        (None, _) => quote!((None, None, &[], &[], &[], &[])),
        (Some(path), Some(id)) => quote!({
            let external: &'static #facade::descriptor::TraitDefinitionDescriptor =
                ::std::boxed::Box::leak(::std::boxed::Box::new(
                #facade::descriptor::TraitDefinitionDescriptor::new(
                    #facade::descriptor::TraitId::External(
                        #facade::identity::ExternalTraitId::new(#id)
                            .expect("validated external trait ID"),
                    ),
                    stringify!(#path),
                    stringify!(#path),
                    stringify!(#path),
                    #facade::descriptor::TraitCompleteness::ExternalIncomplete,
                    ::std::boxed::Box::leak(::std::boxed::Box::new(
                        #facade::expression::GenericDefinitionDescriptor::new(
                            ::std::vec::Vec::<#facade::expression::GenericParameterDescriptor>::new(),
                            ::std::vec::Vec::<#facade::expression::PredicateDescriptor>::new(),
                        ),
                    )),
                ),
            ));
            let applied: &'static #facade::descriptor::TraitDescriptor =
                ::std::boxed::Box::leak(::std::boxed::Box::new(
                {
                    let methods: &'static [#facade::descriptor::MethodDescriptor] =
                        ::std::boxed::Box::leak(::std::vec![#(#external_method_entries),*].into_boxed_slice());
                #facade::descriptor::TraitDescriptor::builder(external)
                    .methods(methods)
                    .build()
                    .expect("external trait application is valid")
                }
            ));
            (Some(external), Some(applied), &[], &[], &[], &[])
        }),
        (Some(path), None) => quote!({
            let payload = <#target as #path>::__qubit_reflect_trait_payload();
            (
                Some(payload.definition()),
                Some(payload.applied()),
                payload.default_method_adapters(),
                payload.default_method_unavailable_reasons(),
                payload.associated_type_resolvers(),
                payload.associated_const_readers(),
            )
        }),
    };
    let implemented_trait = if has_trait {
        quote!(implemented_trait)
    } else {
        quote!(None)
    };
    let trait_definition = if has_trait {
        quote!(trait_definition)
    } else {
        quote!(None)
    };
    let external_registration = match external_id {
        Some(id) => {
            let path = declaration
                .trait_path
                .as_ref()
                .expect("external trait ID is valid only on a trait impl")
                .tokens
                .clone();
            quote! {
                fn external_trait_runtime_identity() -> #facade::__private::codegen_v1::registration::RuntimeIdentity {
                    #facade::__private::codegen_v1::registration::RuntimeIdentity::Trait(
                        #facade::descriptor::TraitId::External(
                            #facade::identity::ExternalTraitId::new(#id)
                                .expect("validated external trait ID"),
                        ),
                    )
                }

                fn external_trait_payload() -> #facade::__private::codegen_v1::registration::FragmentPayload {
                    static DESCRIPTOR: ::std::sync::OnceLock<#facade::descriptor::TraitDefinitionDescriptor> =
                        ::std::sync::OnceLock::new();
                    let descriptor = DESCRIPTOR.get_or_init(|| #facade::descriptor::TraitDefinitionDescriptor::new(
                        #facade::descriptor::TraitId::External(
                            #facade::identity::ExternalTraitId::new(#id)
                                .expect("validated external trait ID"),
                        ),
                        stringify!(#path), stringify!(#path), stringify!(#path),
                        #facade::descriptor::TraitCompleteness::ExternalIncomplete,
                        ::std::boxed::Box::leak(::std::boxed::Box::new(
                            #facade::expression::GenericDefinitionDescriptor::new(
                                ::std::vec::Vec::<#facade::expression::GenericParameterDescriptor>::new(),
                                ::std::vec::Vec::<#facade::expression::PredicateDescriptor>::new(),
                            ),
                        )),
                    ));
                    #facade::__private::codegen_v1::registration::FragmentPayload::Trait(descriptor)
                }

                #facade::__private::codegen_v1::inventory::submit! {
                    #facade::__private::codegen_v1::registration::RegistrationFragment::new(
                        #facade::__private::codegen_v1::registration::FragmentKind::Trait,
                        #facade::__private::codegen_v1::registration::StaticFragmentIdentity::new(
                            env!("CARGO_PKG_NAME"), module_path!(), #line, #column,
                            "external-trait", #fingerprint ^ 0x9e3779b97f4a7c15_u64,
                        ),
                        external_trait_runtime_identity,
                        external_trait_payload,
                    )
                }
            }
        }
        None => quote!(),
    };
    let definition_setup = shared_definition.unwrap_or_else(|| {
        quote! {
            ::std::boxed::Box::leak(::std::boxed::Box::new(
                #facade::descriptor::ImplDefinitionDescriptor::new(
                    fragment_identity(),
                    #facade::expression::TypeExpression::Concrete(
                        #facade::__private::codegen_v1::expression::concrete(
                            vec![#target_source.into()].into_boxed_slice(),
                            ::std::vec::Vec::new().into_boxed_slice(),
                            #facade::expression::DiagnosticText::from(#target_source),
                        ),
                    ),
                    #kind,
                    #trait_definition,
                    ::std::boxed::Box::leak(::std::boxed::Box::new(
                        #impl_generic_definition,
                    )),
                ).expect("generated impl definition is consistent"),
            ))
        }
    });
    quote! {
        #retained

        #[doc(hidden)]
        mod #module {
            use super::*;

            #applicability_witness

            #(#invocation_adapter_definitions)*
            #(#generic_specialization_adapter_definitions)*

            fn fragment_identity() -> #facade::identity::FragmentIdentity {
                #facade::identity::FragmentIdentity::new(
                    env!("CARGO_PKG_NAME"), module_path!(), #line, #column,
                    "impl", #fingerprint,
                )
            }

            fn runtime_identity() -> #facade::__private::codegen_v1::registration::RuntimeIdentity {
                #facade::__private::codegen_v1::registration::RuntimeIdentity::Impl(
                    ::std::any::TypeId::of::<#target>(),
                )
            }

            fn payload() -> #facade::__private::codegen_v1::registration::FragmentPayload {
                static DESCRIPTOR: ::std::sync::OnceLock<#facade::descriptor::ImplDescriptor> =
                    ::std::sync::OnceLock::new();
                let descriptor = DESCRIPTOR.get_or_init(|| {
                    let (
                        trait_definition,
                        implemented_trait,
                        default_method_adapters,
                        default_method_unavailable_reasons,
                        associated_type_resolvers,
                        associated_const_readers,
                    ): (
                        Option<&'static #facade::descriptor::TraitDefinitionDescriptor>,
                        Option<&'static #facade::descriptor::TraitDescriptor>,
                        &'static [Option<&'static #facade::descriptor::InvocationAdapter>],
                        &'static [&'static [#facade::descriptor::InvocationUnavailableReason]],
                        &'static [Option<#facade::descriptor::TypeDescriptorResolver>],
                        &'static [Option<&'static #facade::descriptor::AssociatedConstReader>],
                    ) = #trait_setup;
                    let definition: &'static #facade::descriptor::ImplDefinitionDescriptor =
                        #definition_setup;
                    let methods: &'static [#facade::descriptor::MethodDescriptor] =
                        ::std::boxed::Box::leak(::std::vec![#(#method_entries),*].into_boxed_slice());
                    let adapters: &[Option<&'static #facade::descriptor::InvocationAdapter>] =
                        &[#(#invocation_adapter_entries),*];
                    let declared_unavailable_reasons: &[&[
                        #facade::descriptor::InvocationUnavailableReason
                    ]] = &[#(#invocation_unavailable_reason_entries),*];
                    let method_instances = if let Some(trait_descriptor) = implemented_trait {
                        trait_descriptor.methods().iter().enumerate().map(|(declaration_index, declaration)| {
                            match methods
                                .iter()
                                .enumerate()
                                .find(|(_, method)| method.rust_name() == declaration.rust_name())
                            {
                                Some((method_index, method)) => {
                                    let adapter = adapters[method_index];
                                    let unavailable_reasons: ::std::boxed::Box<[
                                        #facade::descriptor::InvocationUnavailableReason
                                    ]> = if adapter.is_some() {
                                        ::std::vec![].into_boxed_slice()
                                    } else {
                                        declared_unavailable_reasons[method_index]
                                            .to_vec()
                                            .into_boxed_slice()
                                    };
                                    #facade::descriptor::MethodInstanceDescriptor::new(
                                        declaration,
                                        Some(method),
                                        #facade::descriptor::MethodImplementationSource::Overridden,
                                        adapter,
                                        unavailable_reasons,
                                    ).expect("generated trait method instance is consistent")
                                },
                                None if declaration.has_default() => {
                                    let adapter = default_method_adapters
                                        .get(declaration_index)
                                        .copied()
                                        .flatten();
                                    let unavailable_reasons: ::std::boxed::Box<[
                                        #facade::descriptor::InvocationUnavailableReason
                                    ]> = if adapter.is_some() {
                                        ::std::vec![].into_boxed_slice()
                                    } else {
                                        default_method_unavailable_reasons
                                            .get(declaration_index)
                                            .copied()
                                            .unwrap_or(&[
                                                #facade::descriptor::InvocationUnavailableReason::DisabledByPolicy,
                                            ])
                                            .to_vec()
                                            .into_boxed_slice()
                                    };
                                    #facade::descriptor::MethodInstanceDescriptor::new(
                                        declaration,
                                        None,
                                        #facade::descriptor::MethodImplementationSource::Defaulted,
                                        adapter,
                                        unavailable_reasons,
                                    ).expect("generated default trait method instance is consistent")
                                },
                                None => #facade::descriptor::MethodInstanceDescriptor::new(
                                    declaration, None,
                                    #facade::descriptor::MethodImplementationSource::Required,
                                    None,
                                    ::std::boxed::Box::new([
                                        #facade::descriptor::InvocationUnavailableReason::DisabledByPolicy,
                                    ]),
                                ).expect("generated required trait method instance is consistent"),
                            }
                        }).collect()
                    } else {
                        let specializations: ::std::vec::Vec<::std::vec::Vec<(
                            ::std::boxed::Box<[#facade::expression::GenericArgument]>,
                            Option<&'static #facade::descriptor::InvocationAdapter>,
                        )>> = ::std::vec![#(#method_specialization_entries),*];
                        methods.iter().zip(adapters.iter().copied()).enumerate().flat_map(|(index, (method, adapter))| {
                            if !specializations[index].is_empty() {
                                return specializations[index].iter().cloned().map(|(arguments, adapter)| {
                                    let unavailable_reasons: ::std::boxed::Box<[
                                        #facade::descriptor::InvocationUnavailableReason
                                    ]> = if adapter.is_some() {
                                        ::std::vec![].into_boxed_slice()
                                    } else {
                                        ::std::vec![
                                            #facade::descriptor::InvocationUnavailableReason::UnsupportedSpecialization,
                                        ].into_boxed_slice()
                                    };
                                    #facade::descriptor::MethodInstanceDescriptor::with_arguments(
                                        method,
                                        None,
                                        #facade::descriptor::MethodImplementationSource::Declared,
                                        adapter,
                                        arguments,
                                        unavailable_reasons,
                                    ).expect("generated generic method specialization is consistent")
                                }).collect::<::std::vec::Vec<_>>();
                            }
                            let unavailable_reasons: ::std::boxed::Box<[
                                #facade::descriptor::InvocationUnavailableReason
                            ]> = if adapter.is_some() {
                                ::std::vec![] .into_boxed_slice()
                            } else {
                                declared_unavailable_reasons[index]
                                    .to_vec()
                                    .into_boxed_slice()
                            };
                            ::std::vec![#facade::descriptor::MethodInstanceDescriptor::new(
                                method,
                                None,
                                #facade::descriptor::MethodImplementationSource::Declared,
                                adapter,
                                unavailable_reasons,
                            ).expect("generated inherent method instance is consistent")]
                        }).collect::<::std::vec::Vec<_>>()
                    };
                    let associated_types = implemented_trait.map_or_else(::std::vec::Vec::new, |trait_descriptor| {
                        trait_descriptor.associated_types().iter().enumerate().map(|(index, declaration)| {
                            let value = match declaration.rust_name() {
                                #(#associated_type_binding_arms,)*
                                _ => declaration.default().cloned().expect(
                                    "a valid trait impl must bind every required associated type",
                                ),
                            };
                            let concrete_type = associated_type_resolvers
                                .get(index)
                                .copied()
                                .flatten()
                                .or_else(|| match declaration.rust_name() {
                                    #(#specialized_associated_type_resolver_arms,)*
                                    _ => None,
                                });
                            #facade::descriptor::AssociatedTypeBindingDescriptor::new(
                                declaration,
                                value,
                                concrete_type,
                            )
                        }).collect()
                    });
                    let associated_consts = implemented_trait.map_or_else(::std::vec::Vec::new, |trait_descriptor| {
                        trait_descriptor.associated_consts().iter().enumerate().map(|(index, declaration)| {
                            let implementation_source = match declaration.rust_name() {
                                #(#associated_const_override_arms,)*
                                _ => #facade::descriptor::AssociatedConstImplementationSource::Defaulted,
                            };
                            let reader = match declaration.rust_name() {
                                #(#associated_const_reader_override_arms,)*
                                _ => associated_const_readers
                                    .get(index)
                                    .copied()
                                    .flatten(),
                            };
                            #facade::descriptor::AssociatedConstBindingDescriptor::new(
                                declaration,
                                implementation_source,
                                reader,
                            )
                        }).collect()
                    });
                    let mut builder = #facade::descriptor::ImplDescriptor::builder(
                        definition,
                        || <#target as #facade::Reflect>::type_descriptor(),
                    )
                    .methods(methods)
                    .method_instances(method_instances)
                    .associated_types(associated_types)
                    .associated_consts(associated_consts)
                    .arguments(#impl_arguments);
                    if let Some(trait_descriptor) = #implemented_trait {
                        builder = builder.implemented_trait(trait_descriptor);
                    }
                    builder.build().expect("generated impl descriptor is consistent")
                });
                #facade::__private::codegen_v1::registration::FragmentPayload::Impl(descriptor)
            }

            #facade::__private::codegen_v1::inventory::submit! {
                #facade::__private::codegen_v1::registration::RegistrationFragment::new(
                    #facade::__private::codegen_v1::registration::FragmentKind::Impl,
                    #facade::__private::codegen_v1::registration::StaticFragmentIdentity::new(
                        env!("CARGO_PKG_NAME"), module_path!(), #line, #column,
                        "impl", #fingerprint,
                    ),
                    runtime_identity,
                    payload,
                )
            }

            #external_registration
        }
    }
}

// Generic specialization generation lives in `impls::specialization_codegen`.

/// Returns whether reflection policy explicitly suppresses invocation.
fn invocation_disabled_by_policy(method: &MethodIr) -> bool {
    method
        .attributes
        .iter()
        .any(|attribute| matches!(attribute.name, HelperName::NoInvoke | HelperName::Skip))
}

/// Expands every statically applicable invocation blocker in canonical enum
/// order.
pub(super) fn invocation_unavailable_reason_entry(
    method: &MethodIr,
    target: &TokenStream,
    context: &ExpansionContext,
) -> TokenStream {
    let plan = super::invocation::analysis::analyze_method(
        method,
        super::invocation::analysis::MethodContext::implementation(target),
    )
    .expect("validated method analysis is infallible");
    super::invocation::emit::emit_unavailable_reasons(&plan, context)
}
