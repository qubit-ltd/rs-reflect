// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Expansion of distributed registration fragments for `#[reflect_impl]`.

mod concrete_emission;
mod invocation_adapter;
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
use concrete_emission::ConcreteImplEmission;

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
        quote!(#facade::__private::codegen_v1::descriptor::ImplAssociatedTypeDescriptor::new(#rust_name))
    });
    let associated_consts = declaration.associated_consts.iter().map(|item| {
        let rust_name = syn::LitStr::new(&item.name.to_string(), item.span);
        let declared_type = super::traits::type_expression(&item.ty, facade);
        quote!(#facade::__private::codegen_v1::descriptor::ImplAssociatedConstDescriptor::new(
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
            Some(id) => quote!(Some(#facade::__private::codegen_v1::descriptor::TraitId::External(
                #facade::__private::codegen_v1::identity::ExternalTraitId::new(#id)
                    .expect("validated external trait ID"),
            ))),
            None => quote!(None),
        };
        quote! {
            #facade::__private::codegen_v1::descriptor::ImplDefinitionDescriptor::new_unresolved_trait(
                fragment_identity(),
                #target,
                #path,
                #trait_id,
                ::std::boxed::Box::leak(::std::boxed::Box::new(#generics)),
            )
        }
    } else {
        quote! {
            #facade::__private::codegen_v1::descriptor::ImplDefinitionDescriptor::new(
                fragment_identity(),
                #target,
                #facade::__private::codegen_v1::descriptor::ImplKind::Inherent,
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
                        #facade::__private::codegen_v1::descriptor::TraitId::External(
                            #facade::__private::codegen_v1::identity::ExternalTraitId::new(#id)
                                .expect("validated external trait ID"),
                        ),
                    )
                }

                fn external_trait_payload() -> #facade::__private::codegen_v1::registration::FragmentPayload {
                    static TRAIT: ::std::sync::OnceLock<#facade::__private::codegen_v1::descriptor::TraitDefinitionDescriptor> =
                        ::std::sync::OnceLock::new();
                    let descriptor = TRAIT.get_or_init(|| {
                        #facade::__private::codegen_v1::descriptor::TraitDefinitionDescriptor::new(
                            #facade::__private::codegen_v1::descriptor::TraitId::External(
                                #facade::__private::codegen_v1::identity::ExternalTraitId::new(#id)
                                    .expect("validated external trait ID"),
                            ),
                            stringify!(#path),
                            stringify!(#path),
                            stringify!(#path),
                            #facade::__private::codegen_v1::descriptor::TraitCompleteness::ExternalIncomplete,
                            ::std::boxed::Box::leak(::std::boxed::Box::new(
                                #facade::__private::codegen_v1::expression::GenericDefinitionDescriptor::new(
                                    ::std::vec::Vec::<#facade::__private::codegen_v1::expression::GenericParameterDescriptor>::new(),
                                    ::std::vec::Vec::<#facade::__private::codegen_v1::expression::PredicateDescriptor>::new(),
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

            fn fragment_identity() -> #facade::__private::codegen_v1::identity::FragmentIdentity {
                #facade::__private::codegen_v1::identity::FragmentIdentity::new(
                    env!("CARGO_PKG_NAME"), module_path!(), #line, #column,
                    "impl-definition", #fingerprint,
                )
            }

            pub(super) fn definition() -> &'static #facade::__private::codegen_v1::descriptor::ImplDefinitionDescriptor {
                static DEFINITION: ::std::sync::OnceLock<#facade::__private::codegen_v1::descriptor::ImplDefinitionDescriptor> =
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
                    ReceiverKindIr::Value => quote!(Some(#facade::__private::codegen_v1::descriptor::ReceiverDescriptor::Owned)),
                    ReceiverKindIr::SharedReference => quote!(Some(#facade::__private::codegen_v1::descriptor::ReceiverDescriptor::Shared)),
                    ReceiverKindIr::MutableReference => quote!(Some(#facade::__private::codegen_v1::descriptor::ReceiverDescriptor::Mutable)),
                    ReceiverKindIr::Typed => {
                        let value = syn::LitStr::new(&receiver.declaration.to_string(), receiver.span);
                        quote!(Some(#facade::__private::codegen_v1::descriptor::ReceiverDescriptor::Explicit(#value)))
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
                        quote!(#facade::__private::codegen_v1::descriptor::ParameterPatternDescriptor::Identifier)
                    }
                    ParameterPatternKindIr::Wildcard => {
                        quote!(#facade::__private::codegen_v1::descriptor::ParameterPatternDescriptor::Wildcard)
                    }
                    ParameterPatternKindIr::Destructure => {
                        let source = syn::LitStr::new(&parameter.pattern.source, parameter.span);
                        quote!(#facade::__private::codegen_v1::descriptor::ParameterPatternDescriptor::Destructure(#source.into()))
                    }
                };
                let passing = match &parameter.ty.kind {
                    TypeKindIr::Reference { mutable: true, .. } => {
                        quote!(#facade::__private::codegen_v1::descriptor::ParameterPassingMode::MutableBorrow)
                    }
                    TypeKindIr::Reference { .. } => {
                        quote!(#facade::__private::codegen_v1::descriptor::ParameterPassingMode::SharedBorrow)
                    }
                    _ => quote!(#facade::__private::codegen_v1::descriptor::ParameterPassingMode::Owned),
                };
                let ty = super::traits::type_expression(&parameter.ty, facade);
                let parameter_index = parameter.index;
                quote!(#facade::__private::codegen_v1::descriptor::ParameterDescriptor::new(
                    #parameter_index, #name, #pattern, #passing, #ty, None,
                ))
            });
            let return_value = match &method.return_type {
                ReturnTypeIr::Unit => quote!(#facade::__private::codegen_v1::descriptor::ReturnDescriptor::unit()),
                ReturnTypeIr::Type(ty) => {
                    let expression = super::traits::type_expression(ty, facade);
                    let kind = match &ty.kind {
                        TypeKindIr::Never => quote!(#facade::__private::codegen_v1::descriptor::ReturnKind::Never),
                        TypeKindIr::Reference { .. } => quote!(#facade::__private::codegen_v1::descriptor::ReturnKind::Reference),
                        TypeKindIr::ImplTrait { .. } => quote!(#facade::__private::codegen_v1::descriptor::ReturnKind::Opaque),
                        _ => quote!(#facade::__private::codegen_v1::descriptor::ReturnKind::Concrete),
                    };
                    quote!(#facade::__private::codegen_v1::descriptor::ReturnDescriptor::new(#kind, Some(#expression), None))
                }
            };
            let visibility = match &method.visibility {
                VisibilityIr::Public => quote!(#facade::__private::codegen_v1::descriptor::MethodVisibility::Declared(
                    #facade::__private::codegen_v1::identity::Visibility::Public,
                )),
                VisibilityIr::Crate => quote!(#facade::__private::codegen_v1::descriptor::MethodVisibility::Declared(
                    #facade::__private::codegen_v1::identity::Visibility::Crate,
                )),
                VisibilityIr::Super => quote!(#facade::__private::codegen_v1::descriptor::MethodVisibility::Declared(
                    #facade::__private::codegen_v1::identity::Visibility::Super,
                )),
                VisibilityIr::Restricted(path) => {
                    let path = syn::LitStr::new(&path.source, method.span);
                    quote!(#facade::__private::codegen_v1::descriptor::MethodVisibility::Declared(
                        #facade::__private::codegen_v1::identity::Visibility::Restricted(#path.into()),
                    ))
                }
                VisibilityIr::SelfValue | VisibilityIr::Inherited => {
                    quote!(#facade::__private::codegen_v1::descriptor::MethodVisibility::Declared(
                        #facade::__private::codegen_v1::identity::Visibility::Private,
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
                Some(value) => quote!(Some(#facade::__private::codegen_v1::expression::FunctionAbi::Other(#value.into()))),
                None => quote!(None),
            };
            quote! {
                #facade::__private::codegen_v1::descriptor::MethodDescriptor::builder(
                    #facade::__private::codegen_v1::identity::MemberId::new(
                        #target_source, "method", #index, fragment_identity(),
                    ),
                    #rust_name,
                    #query_name,
                    #facade::__private::codegen_v1::descriptor::MethodDeclarationOwner::Impl(definition),
                )
                .visibility(#visibility)
                .receiver(#receiver)
                .parameters(::std::vec![#(#parameters),*])
                .return_value(#return_value)
                .qualifiers(#facade::__private::codegen_v1::descriptor::MethodQualifiers::new(
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
    let invocation_adapter_definitions: Vec<_> = declaration
        .methods
        .iter()
        .enumerate()
        .filter_map(|(index, method)| {
            invocation_adapter::definition(
                method,
                index,
                &target,
                &trait_call_path,
                &target_source,
                &facade,
            )
        })
        .collect();
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
                    quote!(#facade::__private::codegen_v1::value::ThreadSafe)
                } else {
                    quote!(#facade::__private::codegen_v1::value::Local)
                };
                quote!(
                    if <#target as #facade::__private::codegen_v1::Reflect>::type_descriptor()
                        .get_capability(#facade::__private::codegen_v1::invoke::receiver_adapter_key::<#receiver_type, #mode>())
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
                ReceiverKindIr::Value => quote!(Some(#facade::__private::codegen_v1::descriptor::ReceiverDescriptor::Owned)),
                ReceiverKindIr::SharedReference => quote!(Some(#facade::__private::codegen_v1::descriptor::ReceiverDescriptor::Shared)),
                ReceiverKindIr::MutableReference => quote!(Some(#facade::__private::codegen_v1::descriptor::ReceiverDescriptor::Mutable)),
                ReceiverKindIr::Typed => {
                    let value = syn::LitStr::new(&receiver.declaration.to_string(), receiver.span);
                    quote!(Some(#facade::__private::codegen_v1::descriptor::ReceiverDescriptor::Explicit(#value)))
                }
            },
            None => quote!(None),
        };
        let parameters = method.parameters.iter().map(|parameter| {
            let name = parameter.name.as_deref().map(|name| syn::LitStr::new(name, parameter.span));
            let name = match name { Some(value) => quote!(Some(#value)), None => quote!(None) };
            let pattern = match parameter.pattern.kind {
                ParameterPatternKindIr::Identifier => quote!(#facade::__private::codegen_v1::descriptor::ParameterPatternDescriptor::Identifier),
                ParameterPatternKindIr::Wildcard => quote!(#facade::__private::codegen_v1::descriptor::ParameterPatternDescriptor::Wildcard),
                ParameterPatternKindIr::Destructure => {
                    let source = syn::LitStr::new(&parameter.pattern.source, parameter.span);
                    quote!(#facade::__private::codegen_v1::descriptor::ParameterPatternDescriptor::Destructure(#source.into()))
                }
            };
            let passing = match &parameter.ty.kind {
                TypeKindIr::Reference { mutable: true, .. } => quote!(#facade::__private::codegen_v1::descriptor::ParameterPassingMode::MutableBorrow),
                TypeKindIr::Reference { .. } => quote!(#facade::__private::codegen_v1::descriptor::ParameterPassingMode::SharedBorrow),
                _ => quote!(#facade::__private::codegen_v1::descriptor::ParameterPassingMode::Owned),
            };
            let ty = super::traits::type_expression(&parameter.ty, &facade);
            let parameter_index = parameter.index;
            quote!(#facade::__private::codegen_v1::descriptor::ParameterDescriptor::new(#parameter_index, #name, #pattern, #passing, #ty, None))
        });
        let return_value = match &method.return_type {
            ReturnTypeIr::Unit => quote!(#facade::__private::codegen_v1::descriptor::ReturnDescriptor::unit()),
            ReturnTypeIr::Type(ty) => {
                let expression = super::traits::type_expression(ty, &facade);
                let kind = match &ty.kind {
                    TypeKindIr::Never => quote!(#facade::__private::codegen_v1::descriptor::ReturnKind::Never),
                    TypeKindIr::Reference { .. } => quote!(#facade::__private::codegen_v1::descriptor::ReturnKind::Reference),
                    TypeKindIr::ImplTrait { .. } => quote!(#facade::__private::codegen_v1::descriptor::ReturnKind::Opaque),
                    _ => quote!(#facade::__private::codegen_v1::descriptor::ReturnKind::Concrete),
                };
                quote!(#facade::__private::codegen_v1::descriptor::ReturnDescriptor::new(#kind, Some(#expression), None))
            }
        };
        let visibility = match &method.visibility {
            VisibilityIr::Public => quote!(#facade::__private::codegen_v1::descriptor::MethodVisibility::Declared(#facade::__private::codegen_v1::identity::Visibility::Public)),
            VisibilityIr::Crate => quote!(#facade::__private::codegen_v1::descriptor::MethodVisibility::Declared(#facade::__private::codegen_v1::identity::Visibility::Crate)),
            VisibilityIr::Super => quote!(#facade::__private::codegen_v1::descriptor::MethodVisibility::Declared(#facade::__private::codegen_v1::identity::Visibility::Super)),
            VisibilityIr::Restricted(path) => { let path = syn::LitStr::new(&path.source, method.span); quote!(#facade::__private::codegen_v1::descriptor::MethodVisibility::Declared(#facade::__private::codegen_v1::identity::Visibility::Restricted(#path.into()))) },
            VisibilityIr::SelfValue | VisibilityIr::Inherited => quote!(#facade::__private::codegen_v1::descriptor::MethodVisibility::Declared(#facade::__private::codegen_v1::identity::Visibility::Private)),
        };
        let generic_definition = super::traits::generic_definition(&method.generics, method.span, &facade);
        let qualifiers = &method.qualifiers;
        let is_async = qualifiers.is_async;
        let is_unsafe = qualifiers.is_unsafe;
        let is_const = qualifiers.is_const;
        let is_variadic = qualifiers.is_variadic;
        let abi = qualifiers.abi.as_deref().map(|abi| syn::LitStr::new(abi, method.span));
        let abi = match abi { Some(value) => quote!(Some(#facade::__private::codegen_v1::expression::FunctionAbi::Other(#value.into()))), None => quote!(None) };
        quote! {
            #facade::__private::codegen_v1::descriptor::MethodDescriptor::builder(
                #facade::__private::codegen_v1::identity::MemberId::new(
                    #target_source,
                    "method",
                    #index,
                    fragment_identity(),
                ),
                #rust_name,
                #query_name,
                #facade::__private::codegen_v1::descriptor::MethodDeclarationOwner::Impl(definition),
            )
            .visibility(#visibility)
            .receiver(#receiver)
            .parameters(vec![#(#parameters),*])
            .return_value(#return_value)
            .qualifiers(#facade::__private::codegen_v1::descriptor::MethodQualifiers::new(
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
                    ReceiverKindIr::Value => quote!(Some(#facade::__private::codegen_v1::descriptor::ReceiverDescriptor::Owned)),
                    ReceiverKindIr::SharedReference => quote!(Some(#facade::__private::codegen_v1::descriptor::ReceiverDescriptor::Shared)),
                    ReceiverKindIr::MutableReference => quote!(Some(#facade::__private::codegen_v1::descriptor::ReceiverDescriptor::Mutable)),
                    ReceiverKindIr::Typed => { let value = syn::LitStr::new(&receiver.declaration.to_string(), receiver.span); quote!(Some(#facade::__private::codegen_v1::descriptor::ReceiverDescriptor::Explicit(#value))) }
                },
                None => quote!(None),
            };
            let parameters = method.parameters.iter().map(|parameter| {
                let name = parameter.name.as_deref().map(|name| syn::LitStr::new(name, parameter.span));
                let name = match name { Some(value) => quote!(Some(#value)), None => quote!(None) };
                let pattern = match parameter.pattern.kind {
                    ParameterPatternKindIr::Identifier => quote!(#facade::__private::codegen_v1::descriptor::ParameterPatternDescriptor::Identifier),
                    ParameterPatternKindIr::Wildcard => quote!(#facade::__private::codegen_v1::descriptor::ParameterPatternDescriptor::Wildcard),
                    ParameterPatternKindIr::Destructure => { let source = syn::LitStr::new(&parameter.pattern.source, parameter.span); quote!(#facade::__private::codegen_v1::descriptor::ParameterPatternDescriptor::Destructure(#source.into())) }
                };
                let passing = match &parameter.ty.kind { TypeKindIr::Reference { mutable: true, .. } => quote!(#facade::__private::codegen_v1::descriptor::ParameterPassingMode::MutableBorrow), TypeKindIr::Reference { .. } => quote!(#facade::__private::codegen_v1::descriptor::ParameterPassingMode::SharedBorrow), _ => quote!(#facade::__private::codegen_v1::descriptor::ParameterPassingMode::Owned) };
                let ty = super::traits::type_expression(&parameter.ty, &facade);
                let parameter_index = parameter.index;
                quote!(#facade::__private::codegen_v1::descriptor::ParameterDescriptor::new(#parameter_index, #name, #pattern, #passing, #ty, None))
            });
            let return_value = match &method.return_type {
                ReturnTypeIr::Unit => quote!(#facade::__private::codegen_v1::descriptor::ReturnDescriptor::unit()),
                ReturnTypeIr::Type(ty) => { let expression = super::traits::type_expression(ty, &facade); let kind = match &ty.kind { TypeKindIr::Never => quote!(#facade::__private::codegen_v1::descriptor::ReturnKind::Never), TypeKindIr::Reference { .. } => quote!(#facade::__private::codegen_v1::descriptor::ReturnKind::Reference), TypeKindIr::ImplTrait { .. } => quote!(#facade::__private::codegen_v1::descriptor::ReturnKind::Opaque), _ => quote!(#facade::__private::codegen_v1::descriptor::ReturnKind::Concrete) }; quote!(#facade::__private::codegen_v1::descriptor::ReturnDescriptor::new(#kind, Some(#expression), None)) }
            };
            let generic_definition = super::traits::generic_definition(&method.generics, method.span, &facade);
            let qualifiers = &method.qualifiers;
            let is_async = qualifiers.is_async;
            let is_unsafe = qualifiers.is_unsafe;
            let is_const = qualifiers.is_const;
            let is_variadic = qualifiers.is_variadic;
            let abi = qualifiers.abi.as_deref().map(|abi| syn::LitStr::new(abi, method.span));
            let abi = match abi { Some(value) => quote!(Some(#facade::__private::codegen_v1::expression::FunctionAbi::Other(#value.into()))), None => quote!(None) };
            quote! {
                #facade::__private::codegen_v1::descriptor::MethodDescriptor::builder(
                    #facade::__private::codegen_v1::identity::MemberId::new(
                        #target_source, "external-method", #index, fragment_identity(),
                    ),
                    #rust_name, #query_name,
                    #facade::__private::codegen_v1::descriptor::MethodDeclarationOwner::Trait(external),
                )
                .visibility(#facade::__private::codegen_v1::descriptor::MethodVisibility::InheritedFromTrait)
                .receiver(#receiver)
                .parameters(vec![#(#parameters),*])
                .return_value(#return_value)
                .qualifiers(#facade::__private::codegen_v1::descriptor::MethodQualifiers::new(
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
            quote!(#name => #facade::__private::codegen_v1::descriptor::AssociatedConstImplementationSource::Overridden)
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
        quote!(#facade::__private::codegen_v1::descriptor::ImplKind::Trait)
    } else {
        quote!(#facade::__private::codegen_v1::descriptor::ImplKind::Inherent)
    };
    let trait_setup = match (trait_path, external_id) {
        (None, _) => quote!((None, None, &[], &[], &[], &[])),
        (Some(path), Some(id)) => quote!({
            let external: &'static #facade::__private::codegen_v1::descriptor::TraitDefinitionDescriptor =
                ::std::boxed::Box::leak(::std::boxed::Box::new(
                #facade::__private::codegen_v1::descriptor::TraitDefinitionDescriptor::new(
                    #facade::__private::codegen_v1::descriptor::TraitId::External(
                        #facade::__private::codegen_v1::identity::ExternalTraitId::new(#id)
                            .expect("validated external trait ID"),
                    ),
                    stringify!(#path),
                    stringify!(#path),
                    stringify!(#path),
                    #facade::__private::codegen_v1::descriptor::TraitCompleteness::ExternalIncomplete,
                    ::std::boxed::Box::leak(::std::boxed::Box::new(
                        #facade::__private::codegen_v1::expression::GenericDefinitionDescriptor::new(
                            ::std::vec::Vec::<#facade::__private::codegen_v1::expression::GenericParameterDescriptor>::new(),
                            ::std::vec::Vec::<#facade::__private::codegen_v1::expression::PredicateDescriptor>::new(),
                        ),
                    )),
                ),
            ));
            let applied: &'static #facade::__private::codegen_v1::descriptor::TraitDescriptor =
                ::std::boxed::Box::leak(::std::boxed::Box::new(
                {
                    let methods: &'static [#facade::__private::codegen_v1::descriptor::MethodDescriptor] =
                        ::std::boxed::Box::leak(::std::vec![#(#external_method_entries),*].into_boxed_slice());
                #facade::__private::codegen_v1::descriptor::TraitDescriptor::builder(external)
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
                        #facade::__private::codegen_v1::descriptor::TraitId::External(
                            #facade::__private::codegen_v1::identity::ExternalTraitId::new(#id)
                                .expect("validated external trait ID"),
                        ),
                    )
                }

                fn external_trait_payload() -> #facade::__private::codegen_v1::registration::FragmentPayload {
                    static DESCRIPTOR: ::std::sync::OnceLock<#facade::__private::codegen_v1::descriptor::TraitDefinitionDescriptor> =
                        ::std::sync::OnceLock::new();
                    let descriptor = DESCRIPTOR.get_or_init(|| #facade::__private::codegen_v1::descriptor::TraitDefinitionDescriptor::new(
                        #facade::__private::codegen_v1::descriptor::TraitId::External(
                            #facade::__private::codegen_v1::identity::ExternalTraitId::new(#id)
                                .expect("validated external trait ID"),
                        ),
                        stringify!(#path), stringify!(#path), stringify!(#path),
                        #facade::__private::codegen_v1::descriptor::TraitCompleteness::ExternalIncomplete,
                        ::std::boxed::Box::leak(::std::boxed::Box::new(
                            #facade::__private::codegen_v1::expression::GenericDefinitionDescriptor::new(
                                ::std::vec::Vec::<#facade::__private::codegen_v1::expression::GenericParameterDescriptor>::new(),
                                ::std::vec::Vec::<#facade::__private::codegen_v1::expression::PredicateDescriptor>::new(),
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
                #facade::__private::codegen_v1::descriptor::ImplDefinitionDescriptor::new(
                    fragment_identity(),
                    #facade::__private::codegen_v1::expression::TypeExpression::Concrete(
                        #facade::__private::codegen_v1::expression::concrete(
                            vec![#target_source.into()].into_boxed_slice(),
                            ::std::vec::Vec::new().into_boxed_slice(),
                            #facade::__private::codegen_v1::expression::DiagnosticText::from(#target_source),
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
    let generic_specialization_adapter_definitions =
        generic_specialization_adapter_definitions.collect();
    let method_entries = method_entries.collect();
    let invocation_adapter_entries = invocation_adapter_entries.collect();
    let method_specialization_entries = method_specialization_entries.collect();
    concrete_emission::emit(ConcreteImplEmission {
        retained,
        module,
        applicability_witness,
        invocation_adapter_definitions,
        generic_specialization_adapter_definitions,
        facade,
        line,
        column,
        fingerprint,
        target,
        trait_setup,
        definition_setup,
        method_entries,
        invocation_adapter_entries,
        invocation_unavailable_reason_entries,
        method_specialization_entries,
        implemented_trait,
        associated_type_binding_arms,
        specialized_associated_type_resolver_arms,
        associated_const_override_arms,
        associated_const_reader_override_arms,
        impl_arguments,
        external_registration,
    })
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
