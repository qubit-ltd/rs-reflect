// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Expansion of distributed registration fragments for `#[reflect_impl]`.

mod concrete_emission;
mod concrete_impl_emission;
mod internal;
mod invocation_adapter;
mod specialization_codegen;

use concrete_impl_emission::ConcreteImplEmission;
use internal::generic_substituter::substitute_type_syntax;
use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use specialization_codegen::specialization_arguments;
use specialization_codegen::specialized_method;
use specialization_codegen::typed_extension_receiver_type;

use crate::expand::ExpansionContext;
use crate::expand::generic_environment::GenericEnvironment;
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
    let location = declaration.span.start();
    let line = location.line as u32;
    let column = location.column as u32;
    let module = generic_impl_definition_module(declaration);
    let environment = GenericEnvironment::from_generics(&declaration.generics);
    let target = super::traits::type_expression(&declaration.target_type, &environment, facade);
    let generics =
        super::traits::generic_definition(&declaration.generics, declaration.span, facade);
    let methods = definition_method_entries(declaration, facade);
    let associated_types = declaration.associated_types.iter().map(|item| {
        let rust_name = syn::LitStr::new(&item.name.to_string(), item.span);
        quote!(#facade::__private::codegen_v4::descriptor::ImplAssociatedTypeDescriptor::new(#rust_name))
    });
    let associated_consts = declaration.associated_consts.iter().map(|item| {
        let rust_name = syn::LitStr::new(&item.name.to_string(), item.span);
        let declared_type = super::traits::type_expression(&item.ty, &environment, facade);
        quote!(#facade::__private::codegen_v4::descriptor::ImplAssociatedConstDescriptor::new(
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
    let reflected_provider =
        declaration
            .attributes
            .iter()
            .find_map(|attribute| match &attribute.value {
                HelperValueIr::DefinitionProviderV2(provider) => Some(provider),
                _ => None,
            });
    let reflected_provider_import =
        reflected_provider.map(|provider| quote!(use super::#provider;));
    let initialize_methods = if let Some(provider) = reflected_provider {
        quote! {
            let mut methods = ::std::vec![#(#methods),*];
            methods.retain(|method| {
                #provider().methods().iter().any(|declaration| declaration.rust_name() == method.rust_name())
            });
            methods.into_boxed_slice()
        }
    } else {
        quote!(::std::vec![#(#methods),*].into_boxed_slice())
    };
    let definition_constructor = if let Some(trait_path) = &declaration.trait_path {
        let path = trait_path
            .segments
            .iter()
            .map(|segment| segment.name.as_str())
            .collect::<Vec<_>>()
            .join("::");
        let path = syn::LitStr::new(&path, declaration.span);
        let trait_id = match external_id {
            Some(id) => {
                quote!(Some(#facade::__private::codegen_v4::descriptor::TraitId::External(
                    #facade::__private::codegen_v4::identity::ExternalTraitId::new(#id)
                        .expect("validated external trait ID"),
                )))
            }
            None => {
                if let Some(provider) = reflected_provider {
                    quote!(Some(#provider().trait_id().clone()))
                } else {
                    quote!(None)
                }
            }
        };
        let trait_application =
            super::expression_codegen::path_type_expression(trait_path, &environment, facade);
        quote! {
            #facade::__private::codegen_v4::descriptor::ImplDefinitionDescriptor::new_unresolved_trait_with_expression(
                fragment_identity(),
                #target,
                #path,
                #trait_id,
                #trait_application,
                ::std::boxed::Box::leak(::std::boxed::Box::new(#generics)),
            )
        }
    } else {
        quote! {
            #facade::__private::codegen_v4::descriptor::ImplDefinitionDescriptor::new(
                fragment_identity(),
                #target,
                #facade::__private::codegen_v4::descriptor::ImplKind::Inherent,
                None,
                ::std::boxed::Box::leak(::std::boxed::Box::new(#generics)),
            ).expect("generated generic impl definition is consistent")
        }
    };
    let external_registration = match (declaration.trait_path.as_ref(), external_id) {
        (Some(path), Some(id)) => {
            let path = path.tokens.clone();
            quote! {
                fn external_trait_runtime_identity() -> #facade::__private::codegen_v4::registration::RuntimeIdentity {
                    #facade::__private::codegen_v4::registration::RuntimeIdentity::Trait(
                        #facade::__private::codegen_v4::descriptor::TraitId::External(
                            #facade::__private::codegen_v4::identity::ExternalTraitId::new(#id)
                                .expect("validated external trait ID"),
                        ),
                    )
                }

                fn external_trait_payload() -> #facade::__private::codegen_v4::registration::FragmentPayload {
                    static TRAIT: ::std::sync::OnceLock<#facade::__private::codegen_v4::descriptor::TraitDefinitionDescriptor> =
                        ::std::sync::OnceLock::new();
                    let descriptor = TRAIT.get_or_init(|| {
                        #facade::__private::codegen_v4::descriptor::TraitDefinitionDescriptor::new(
                            #facade::__private::codegen_v4::descriptor::TraitId::External(
                                #facade::__private::codegen_v4::identity::ExternalTraitId::new(#id)
                                    .expect("validated external trait ID"),
                            ),
                            stringify!(#path),
                            stringify!(#path),
                            stringify!(#path),
                            #facade::__private::codegen_v4::descriptor::TraitCompleteness::ExternalIncomplete,
                            ::std::boxed::Box::leak(::std::boxed::Box::new(
                                #facade::__private::codegen_v4::expression::GenericDefinitionDescriptor::new(
                                    ::std::vec::Vec::<#facade::__private::codegen_v4::expression::GenericParameterDescriptor>::new(),
                                    ::std::vec::Vec::<#facade::__private::codegen_v4::expression::PredicateDescriptor>::new(),
                                ),
                            )),
                        )
                    });
                    #facade::__private::codegen_v4::registration::FragmentPayload::Trait(descriptor)
                }

                #facade::__private::codegen_v4::inventory::submit! {
                    #facade::__private::codegen_v4::registration::RegistrationFragment::new(
                        #facade::__private::codegen_v4::registration::FragmentKind::Trait,
                        #facade::__private::codegen_v4::registration::StaticFragmentIdentity::new(
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
            #reflected_provider_import

            fn fragment_identity() -> #facade::__private::codegen_v4::identity::FragmentIdentity {
                #facade::__private::codegen_v4::identity::FragmentIdentity::new(
                    env!("CARGO_PKG_NAME"), module_path!(), #line, #column,
                    "impl-definition", #fingerprint,
                )
            }

            pub(super) fn definition() -> &'static #facade::__private::codegen_v4::descriptor::ImplDefinitionDescriptor {
                static DEFINITION: ::std::sync::OnceLock<#facade::__private::codegen_v4::descriptor::ImplDefinitionDescriptor> =
                    ::std::sync::OnceLock::new();
                let definition = DEFINITION.get_or_init(|| #definition_constructor);
                definition.initialize_methods(|_definition| { #initialize_methods });
                definition.initialize_associated_items(|_| (
                    ::std::vec![#(#associated_types),*].into_boxed_slice(),
                    ::std::vec![#(#associated_consts),*].into_boxed_slice(),
                ));
                definition
            }

            fn runtime_identity() -> #facade::__private::codegen_v4::registration::RuntimeIdentity {
                #facade::__private::codegen_v4::registration::RuntimeIdentity::ImplDefinition(
                    fragment_identity(),
                )
            }

            fn payload() -> #facade::__private::codegen_v4::registration::FragmentPayload {
                #facade::__private::codegen_v4::registration::FragmentPayload::ImplDefinition(definition())
            }

            #facade::__private::codegen_v4::inventory::submit! {
                #facade::__private::codegen_v4::registration::RegistrationFragment::new(
                    #facade::__private::codegen_v4::registration::FragmentKind::ImplDefinition,
                    #facade::__private::codegen_v4::registration::StaticFragmentIdentity::new(
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

/// Builds declaration-level method descriptors without concrete adapters or
/// instances.
///
/// # Parameters
///
/// - `declaration`: Generic impl declaration containing method signatures.
/// - `facade`: Runtime facade path used in generated references.
///
/// # Returns
///
/// Returns method descriptor expressions in declaration order.
#[must_use]
fn definition_method_entries(
    declaration: &ImplDeclarationIr,
    facade: &TokenStream,
) -> Vec<TokenStream> {
    let target_source = declaration.target_type.source.as_str();
    let environment = GenericEnvironment::from_generics(&declaration.generics);
    declaration
        .methods
        .iter()
        .enumerate()
        .map(|(index, method)| {
            let method_environment = environment.clone().with_generics(&method.generics);
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
                    ReceiverKindIr::Value => quote!(Some(#facade::__private::codegen_v4::descriptor::ReceiverDescriptor::Owned)),
                    ReceiverKindIr::SharedReference => quote!(Some(#facade::__private::codegen_v4::descriptor::ReceiverDescriptor::Shared)),
                    ReceiverKindIr::MutableReference => quote!(Some(#facade::__private::codegen_v4::descriptor::ReceiverDescriptor::Mutable)),
                    ReceiverKindIr::Typed => {
                        let value = syn::LitStr::new(&receiver.declaration.to_string(), receiver.span);
                        quote!(Some(#facade::__private::codegen_v4::descriptor::ReceiverDescriptor::Explicit(#value)))
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
                        quote!(#facade::__private::codegen_v4::descriptor::ParameterPatternDescriptor::Identifier)
                    }
                    ParameterPatternKindIr::Wildcard => {
                        quote!(#facade::__private::codegen_v4::descriptor::ParameterPatternDescriptor::Wildcard)
                    }
                    ParameterPatternKindIr::Destructure => {
                        let source = syn::LitStr::new(&parameter.pattern.source, parameter.span);
                        quote!(#facade::__private::codegen_v4::descriptor::ParameterPatternDescriptor::Destructure(#source.into()))
                    }
                };
                let passing = match &parameter.ty.kind {
                    TypeKindIr::Reference { mutable: true, .. } => {
                        quote!(#facade::__private::codegen_v4::descriptor::ParameterPassingMode::MutableBorrow)
                    }
                    TypeKindIr::Reference { .. } => {
                        quote!(#facade::__private::codegen_v4::descriptor::ParameterPassingMode::SharedBorrow)
                    }
                    _ => quote!(#facade::__private::codegen_v4::descriptor::ParameterPassingMode::Owned),
                };
                let ty = super::traits::type_expression(
                    &parameter.ty,
                    &method_environment,
                    facade,
                );
                let parameter_index = parameter.index;
                quote!(#facade::__private::codegen_v4::descriptor::ParameterDescriptor::new(
                    #parameter_index, #name, #pattern, #passing, #ty, None,
                ))
            });
            let return_value = match &method.return_type {
                ReturnTypeIr::Unit => quote!(#facade::__private::codegen_v4::descriptor::ReturnDescriptor::unit()),
                ReturnTypeIr::Type(ty) => {
                    let expression =
                        super::traits::type_expression(ty, &method_environment, facade);
                    let kind = match &ty.kind {
                        TypeKindIr::Never => quote!(#facade::__private::codegen_v4::descriptor::ReturnKind::Never),
                        TypeKindIr::Reference { .. } => quote!(#facade::__private::codegen_v4::descriptor::ReturnKind::Reference),
                        TypeKindIr::ImplTrait { .. } => quote!(#facade::__private::codegen_v4::descriptor::ReturnKind::Opaque),
                        _ => quote!(#facade::__private::codegen_v4::descriptor::ReturnKind::Concrete),
                    };
                    quote!(#facade::__private::codegen_v4::descriptor::ReturnDescriptor::new(#kind, Some(#expression), None))
                }
            };
            let visibility = match &method.visibility {
                VisibilityIr::Public => quote!(#facade::__private::codegen_v4::descriptor::MethodVisibility::Declared(
                    #facade::__private::codegen_v4::identity::Visibility::Public,
                )),
                VisibilityIr::Crate => quote!(#facade::__private::codegen_v4::descriptor::MethodVisibility::Declared(
                    #facade::__private::codegen_v4::identity::Visibility::Crate,
                )),
                VisibilityIr::Super => quote!(#facade::__private::codegen_v4::descriptor::MethodVisibility::Declared(
                    #facade::__private::codegen_v4::identity::Visibility::Super,
                )),
                VisibilityIr::Restricted(path) => {
                    let path = syn::LitStr::new(&path.source, method.span);
                    quote!(#facade::__private::codegen_v4::descriptor::MethodVisibility::Declared(
                        #facade::__private::codegen_v4::identity::Visibility::Restricted(#path.into()),
                    ))
                }
                VisibilityIr::SelfValue | VisibilityIr::Inherited => {
                    quote!(#facade::__private::codegen_v4::descriptor::MethodVisibility::Declared(
                        #facade::__private::codegen_v4::identity::Visibility::Private,
                    ))
                }
            };
            let generic_definition = super::traits::generic_definition(&method.generics, method.span, facade);
            let qualifiers = &method.qualifiers;
            let is_async = qualifiers.is_async;
            let is_unsafe = qualifiers.is_unsafe;
            let is_const = qualifiers.is_const;
            let is_variadic = qualifiers.is_variadic;
            let abi = qualifiers.abi.as_deref().map(|abi| {
                super::expression_codegen::function_abi(Some(abi), method.span, facade)
            });
            let abi = match abi {
                Some(value) => quote!(Some(#value)),
                None => quote!(None),
            };
            quote! {
                #facade::__private::codegen_v4::descriptor::MethodDescriptor::builder(
                    #facade::__private::codegen_v4::identity::MemberId::new(
                        #target_source, "method", #index, fragment_identity(),
                    ),
                    #rust_name,
                    #query_name,
                    #facade::__private::codegen_v4::descriptor::MethodDeclarationOwner::Impl(definition),
                )
                .visibility(#visibility)
                .receiver(#receiver)
                .parameters(::std::vec![#(#parameters),*])
                .return_value(#return_value)
                .qualifiers(#facade::__private::codegen_v4::descriptor::MethodQualifiers::new(
                    #is_async, #is_unsafe, #is_const, #abi, #is_variadic,
                ))
                .generic_definition(&#generic_definition)
                .build()
            }
        })
        .collect()
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
    impl_arguments: TokenStream,
    shared_definition: Option<TokenStream>,
    applicability_witness: Option<TokenStream>,
    specialized_associated_type_resolver_arms: Vec<TokenStream>,
    facade: &TokenStream,
    context: &ExpansionContext,
) -> TokenStream {
    let facade = facade.clone();
    let environment = GenericEnvironment::from_generics(&declaration.generics);
    let external_trait_metadata = declaration.trait_path.as_ref().map(|path| {
        let rust_name = path
            .segments
            .last()
            .map(|segment| segment.name.clone())
            .expect("a trait path has at least one segment");
        let mut rust_path = path
            .segments
            .iter()
            .map(|segment| segment.name.as_str())
            .collect::<Vec<_>>()
            .join("::");
        if path.leading_colon {
            rust_path.insert_str(0, "::");
        }
        let arguments = path
            .segments
            .last()
            .map(|segment| {
                super::expression_codegen::path_arguments(
                    &segment.arguments,
                    &environment,
                    &facade,
                    path.span,
                )
            })
            .unwrap_or_default();
        (
            syn::LitStr::new(&rust_name, path.span),
            syn::LitStr::new(&rust_path, path.span),
            arguments,
        )
    });
    let retained = declaration.retained_tokens;
    let target = declaration.target_type.tokens.clone();
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
    let target_source = declaration.target_type.source.clone();
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
    let invocation_adapter_entries =
        declaration
            .methods
            .iter()
            .enumerate()
            .map(|(index, method)| {
                let typed_extension_receiver = method
                    .receiver
                    .as_ref()
                    .and_then(|receiver| typed_extension_receiver_type(receiver, &target));
                let mut method_context =
                    super::invocation::analysis::MethodContext::implementation(&target);
                method_context.extension_receiver = typed_extension_receiver.clone();
                let invocation_plan =
                    super::invocation::analysis::analyze_method(method, method_context)
                        .expect("validated method analysis is infallible");
                debug_assert_eq!(invocation_plan.parameter_count(), method.parameters.len());
                if invocation_plan.pinned_receiver_mutability().is_some() {
                    let is_safe_pinned_invocation = invocation_plan.is_executable();
                    if is_safe_pinned_invocation {
                        let descriptor_name =
                            format_ident!("__QUBIT_REFLECT_INVOCATION_ADAPTER_{index}");
                        return quote!(Some(&#descriptor_name));
                    }
                    return quote!(None);
                }
                let is_safe_invocation = invocation_plan.is_executable();
                if is_safe_invocation {
                    let descriptor_name =
                        format_ident!("__QUBIT_REFLECT_INVOCATION_ADAPTER_{index}");
                    quote!(Some(&#descriptor_name))
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
            let trait_call_path = trait_call_path.clone();
            method.specializations.iter().enumerate().filter_map(
                move |(specialization_index, specialization)| {
                    let (method, generic_arguments) = specialized_method(method, specialization)?;
                    invocation_adapter::specialization_definition(
                        &method,
                        method_index,
                        (specialization_index, &generic_arguments),
                        &target,
                        &trait_call_path,
                        &target_source,
                        &facade,
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
                let Some((concrete, generic_arguments)) = specialized_method(method, specialization) else {
                    return quote!((#arguments, None, &[
                        #facade::__private::codegen_v4::descriptor::InvocationUnavailableReason::UnsupportedSpecialization,
                    ]));
                };
                let mut method_context =
                    super::invocation::analysis::MethodContext::implementation(&target);
                method_context.extension_receiver = concrete
                    .receiver
                    .as_ref()
                    .and_then(|receiver| typed_extension_receiver_type(receiver, &target));
                let plan = super::invocation::analysis::analyze_method(&concrete, method_context)
                    .expect("validated specialized method analysis is infallible");
                let unavailable_reasons =
                    super::invocation::emit::emit_unavailable_reasons(&plan, context);
                let adapter_name = invocation_adapter::specialization_descriptor_name(
                    method_index,
                    specialization_index,
                );
                if invocation_adapter::specialization_definition(
                    &concrete,
                    method_index,
                    (specialization_index, &generic_arguments),
                    &target,
                    &trait_call_path,
                    &target_source,
                    &facade,
                ).is_some() {
                    quote!((#arguments, Some(&#adapter_name), #unavailable_reasons))
                } else {
                    quote!((#arguments, None, #unavailable_reasons))
                }
            });
        quote!(::std::vec![#(#entries),*])
    });
    let method_entries = declaration
        .methods
        .iter()
        .enumerate()
        .map(|(index, method)| {
        let method_environment = environment.clone().with_generics(&method.generics);
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
                ReceiverKindIr::Value => quote!(Some(#facade::__private::codegen_v4::descriptor::ReceiverDescriptor::Owned)),
                ReceiverKindIr::SharedReference => quote!(Some(#facade::__private::codegen_v4::descriptor::ReceiverDescriptor::Shared)),
                ReceiverKindIr::MutableReference => quote!(Some(#facade::__private::codegen_v4::descriptor::ReceiverDescriptor::Mutable)),
                ReceiverKindIr::Typed => {
                    let value = syn::LitStr::new(&receiver.declaration.to_string(), receiver.span);
                    quote!(Some(#facade::__private::codegen_v4::descriptor::ReceiverDescriptor::Explicit(#value)))
                }
            },
            None => quote!(None),
        };
        let parameters = method.parameters.iter().map(|parameter| {
            let name = parameter.name.as_deref().map(|name| syn::LitStr::new(name, parameter.span));
            let name = match name { Some(value) => quote!(Some(#value)), None => quote!(None) };
            let pattern = match parameter.pattern.kind {
                ParameterPatternKindIr::Identifier => quote!(#facade::__private::codegen_v4::descriptor::ParameterPatternDescriptor::Identifier),
                ParameterPatternKindIr::Wildcard => quote!(#facade::__private::codegen_v4::descriptor::ParameterPatternDescriptor::Wildcard),
                ParameterPatternKindIr::Destructure => {
                    let source = syn::LitStr::new(&parameter.pattern.source, parameter.span);
                    quote!(#facade::__private::codegen_v4::descriptor::ParameterPatternDescriptor::Destructure(#source.into()))
                }
            };
            let passing = match &parameter.ty.kind {
                TypeKindIr::Reference { mutable: true, .. } => quote!(#facade::__private::codegen_v4::descriptor::ParameterPassingMode::MutableBorrow),
                TypeKindIr::Reference { .. } => quote!(#facade::__private::codegen_v4::descriptor::ParameterPassingMode::SharedBorrow),
                _ => quote!(#facade::__private::codegen_v4::descriptor::ParameterPassingMode::Owned),
            };
            let ty = super::traits::type_expression(
                &parameter.ty,
                &method_environment,
                &facade,
            );
            let parameter_index = parameter.index;
            quote!(#facade::__private::codegen_v4::descriptor::ParameterDescriptor::new(#parameter_index, #name, #pattern, #passing, #ty, None))
        });
        let return_value = match &method.return_type {
            ReturnTypeIr::Unit => quote!(#facade::__private::codegen_v4::descriptor::ReturnDescriptor::unit()),
            ReturnTypeIr::Type(ty) => {
                let expression =
                    super::traits::type_expression(ty, &method_environment, &facade);
                let kind = match &ty.kind {
                    TypeKindIr::Never => quote!(#facade::__private::codegen_v4::descriptor::ReturnKind::Never),
                    TypeKindIr::Reference { .. } => quote!(#facade::__private::codegen_v4::descriptor::ReturnKind::Reference),
                    TypeKindIr::ImplTrait { .. } => quote!(#facade::__private::codegen_v4::descriptor::ReturnKind::Opaque),
                    _ => quote!(#facade::__private::codegen_v4::descriptor::ReturnKind::Concrete),
                };
                quote!(#facade::__private::codegen_v4::descriptor::ReturnDescriptor::new(#kind, Some(#expression), None))
            }
        };
        let visibility = match &method.visibility {
            VisibilityIr::Public => quote!(#facade::__private::codegen_v4::descriptor::MethodVisibility::Declared(#facade::__private::codegen_v4::identity::Visibility::Public)),
            VisibilityIr::Crate => quote!(#facade::__private::codegen_v4::descriptor::MethodVisibility::Declared(#facade::__private::codegen_v4::identity::Visibility::Crate)),
            VisibilityIr::Super => quote!(#facade::__private::codegen_v4::descriptor::MethodVisibility::Declared(#facade::__private::codegen_v4::identity::Visibility::Super)),
            VisibilityIr::Restricted(path) => { let path = syn::LitStr::new(&path.source, method.span); quote!(#facade::__private::codegen_v4::descriptor::MethodVisibility::Declared(#facade::__private::codegen_v4::identity::Visibility::Restricted(#path.into()))) },
            VisibilityIr::SelfValue | VisibilityIr::Inherited => quote!(#facade::__private::codegen_v4::descriptor::MethodVisibility::Declared(#facade::__private::codegen_v4::identity::Visibility::Private)),
        };
        let generic_definition = super::traits::generic_definition(&method.generics, method.span, &facade);
        let qualifiers = &method.qualifiers;
        let is_async = qualifiers.is_async;
        let is_unsafe = qualifiers.is_unsafe;
        let is_const = qualifiers.is_const;
        let is_variadic = qualifiers.is_variadic;
        let abi = qualifiers.abi.as_deref().map(|abi| {
            super::expression_codegen::function_abi(Some(abi), method.span, &facade)
        });
        let abi = match abi { Some(value) => quote!(Some(#value)), None => quote!(None) };
        quote! {
            #facade::__private::codegen_v4::descriptor::MethodDescriptor::builder(
                #facade::__private::codegen_v4::identity::MemberId::new(
                    #target_source,
                    "method",
                    #index,
                    fragment_identity(),
                ),
                #rust_name,
                #query_name,
                #facade::__private::codegen_v4::descriptor::MethodDeclarationOwner::Impl(definition),
            )
            .visibility(#visibility)
            .receiver(#receiver)
            .parameters(vec![#(#parameters),*])
            .return_value(#return_value)
            .qualifiers(#facade::__private::codegen_v4::descriptor::MethodQualifiers::new(
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
            let method_environment = environment.clone().with_generics(&method.generics);
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
                    ReceiverKindIr::Value => {
                        quote!(Some(#facade::__private::codegen_v4::descriptor::ReceiverDescriptor::Owned))
                    }
                    ReceiverKindIr::SharedReference => {
                        quote!(Some(#facade::__private::codegen_v4::descriptor::ReceiverDescriptor::Shared))
                    }
                    ReceiverKindIr::MutableReference => {
                        quote!(Some(#facade::__private::codegen_v4::descriptor::ReceiverDescriptor::Mutable))
                    }
                    ReceiverKindIr::Typed => {
                        let value = syn::LitStr::new(&receiver.declaration.to_string(), receiver.span);
                        quote!(Some(#facade::__private::codegen_v4::descriptor::ReceiverDescriptor::Explicit(#value)))
                    }
                },
                None => quote!(None),
            };
            let parameters = method.parameters.iter().map(|parameter| {
                let name = parameter.name.as_deref().map(|name| syn::LitStr::new(name, parameter.span));
                let name = match name {
                    Some(value) => quote!(Some(#value)),
                    None => quote!(None),
                };
                let pattern = match parameter.pattern.kind {
                    ParameterPatternKindIr::Identifier => {
                        quote!(#facade::__private::codegen_v4::descriptor::ParameterPatternDescriptor::Identifier)
                    }
                    ParameterPatternKindIr::Wildcard => {
                        quote!(#facade::__private::codegen_v4::descriptor::ParameterPatternDescriptor::Wildcard)
                    }
                    ParameterPatternKindIr::Destructure => {
                        let source = syn::LitStr::new(&parameter.pattern.source, parameter.span);
                        quote!(#facade::__private::codegen_v4::descriptor::ParameterPatternDescriptor::Destructure(#source.into()))
                    }
                };
                let passing = match &parameter.ty.kind {
                    TypeKindIr::Reference { mutable: true, .. } => {
                        quote!(#facade::__private::codegen_v4::descriptor::ParameterPassingMode::MutableBorrow)
                    }
                    TypeKindIr::Reference { .. } => {
                        quote!(#facade::__private::codegen_v4::descriptor::ParameterPassingMode::SharedBorrow)
                    }
                    _ => quote!(#facade::__private::codegen_v4::descriptor::ParameterPassingMode::Owned),
                };
                let ty = super::traits::type_expression(
                    &parameter.ty,
                    &method_environment,
                    &facade,
                );
                let parameter_index = parameter.index;
                quote!(#facade::__private::codegen_v4::descriptor::ParameterDescriptor::new(
                    #parameter_index,
                    #name,
                    #pattern,
                    #passing,
                    #ty,
                    None,
                ))
            });
            let return_value = match &method.return_type {
                ReturnTypeIr::Unit => quote!(#facade::__private::codegen_v4::descriptor::ReturnDescriptor::unit()),
                ReturnTypeIr::Type(ty) => {
                    let expression = super::traits::type_expression(ty, &method_environment, &facade);
                    let kind = match &ty.kind {
                        TypeKindIr::Never => quote!(#facade::__private::codegen_v4::descriptor::ReturnKind::Never),
                        TypeKindIr::Reference { .. } => quote!(#facade::__private::codegen_v4::descriptor::ReturnKind::Reference),
                        TypeKindIr::ImplTrait { .. } => quote!(#facade::__private::codegen_v4::descriptor::ReturnKind::Opaque),
                        _ => quote!(#facade::__private::codegen_v4::descriptor::ReturnKind::Concrete),
                    };
                    quote!(#facade::__private::codegen_v4::descriptor::ReturnDescriptor::new(
                        #kind,
                        Some(#expression),
                        None,
                    ))
                }
            };
            let generic_definition = super::traits::generic_definition(&method.generics, method.span, &facade);
            let qualifiers = &method.qualifiers;
            let is_async = qualifiers.is_async;
            let is_unsafe = qualifiers.is_unsafe;
            let is_const = qualifiers.is_const;
            let is_variadic = qualifiers.is_variadic;
            let abi = qualifiers.abi.as_deref().map(|abi| {
                super::expression_codegen::function_abi(Some(abi), method.span, &facade)
            });
            let abi = match abi { Some(value) => quote!(Some(#value)), None => quote!(None) };
            quote! {
                #facade::__private::codegen_v4::descriptor::MethodDescriptor::builder(
                    #facade::__private::codegen_v4::identity::MemberId::new(
                        #target_source, "external-method", #index, fragment_identity(),
                    ),
                    #rust_name, #query_name,
                    #facade::__private::codegen_v4::descriptor::MethodDeclarationOwner::Trait(external),
                )
                .visibility(#facade::__private::codegen_v4::descriptor::MethodVisibility::InheritedFromTrait)
                .receiver(#receiver)
                .parameters(vec![#(#parameters),*])
                .return_value(#return_value)
                .qualifiers(#facade::__private::codegen_v4::descriptor::MethodQualifiers::new(
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
            let value = super::traits::type_expression(value, &environment, &facade);
            Some(quote!(#name => #value))
        })
        .collect();
    let associated_const_override_arms: Vec<_> = declaration
        .associated_consts
        .iter()
        .map(|item| {
            let name = syn::LitStr::new(&item.name.to_string(), item.span);
            quote!(#name => #facade::__private::codegen_v4::descriptor::AssociatedConstImplementationSource::Overridden)
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
                        substitute_type_syntax(&item.ty.tokens, &[(Ident::new("Self", item.span), qualified_target)]);
                    quote!(
                        #rust_name => Some(
                            #facade::__private::codegen_v4::descriptor::associated_const_reader::<#value_type>(
                                || <#target as #path>::#const_name,
                            ),
                        )
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let kind = if has_trait {
        quote!(#facade::__private::codegen_v4::descriptor::ImplKind::Trait)
    } else {
        quote!(#facade::__private::codegen_v4::descriptor::ImplKind::Inherent)
    };
    let trait_setup = match (trait_path, external_id) {
        (None, _) => quote!((None, None, &[], &[], &[], &[])),
        (Some(_path), Some(id)) => {
            let (rust_name, rust_path, arguments) = external_trait_metadata
                .as_ref()
                .expect("external trait metadata exists for a trait impl");
            quote!({
                let external: &'static #facade::__private::codegen_v4::descriptor::TraitDefinitionDescriptor =
                    ::std::boxed::Box::leak(::std::boxed::Box::new(
                    #facade::__private::codegen_v4::descriptor::TraitDefinitionDescriptor::new(
                        #facade::__private::codegen_v4::descriptor::TraitId::External(
                            #facade::__private::codegen_v4::identity::ExternalTraitId::new(#id)
                                .expect("validated external trait ID"),
                        ),
                        #rust_name,
                        #rust_path,
                        #rust_name,
                        #facade::__private::codegen_v4::descriptor::TraitCompleteness::ExternalIncomplete,
                        ::std::boxed::Box::leak(::std::boxed::Box::new(
                            #facade::__private::codegen_v4::expression::GenericDefinitionDescriptor::new(
                                ::std::vec::Vec::<#facade::__private::codegen_v4::expression::GenericParameterDescriptor>::new(),
                                ::std::vec::Vec::<#facade::__private::codegen_v4::expression::PredicateDescriptor>::new(),
                            ),
                        )),
                    ),
                ));
                let applied: &'static #facade::__private::codegen_v4::descriptor::TraitDescriptor =
                    ::std::boxed::Box::leak(::std::boxed::Box::new(
                    {
                        let methods: &'static [#facade::__private::codegen_v4::descriptor::MethodDescriptor] =
                            ::std::boxed::Box::leak(::std::vec![#(#external_method_entries),*].into_boxed_slice());
                    #facade::__private::codegen_v4::descriptor::TraitDescriptor::builder(external)
                        .arguments(::std::vec![#(#arguments),*])
                        .methods(methods)
                        .build()
                        .expect("external trait application is valid")
                    }
                ));
                (Some(external), Some(applied), &[], &[], &[], &[])
            })
        }
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
    let external_registration = match external_id {
        Some(id) => {
            let (rust_name, rust_path, _) = external_trait_metadata
                .as_ref()
                .expect("external trait ID is valid only on a trait impl");
            let trait_guard_path = trait_call_path
                .as_ref()
                .expect("external trait ID is valid only on a trait impl");
            quote! {
                trait __QubitReflectExternalTraitGuard {
                    fn __qubit_reflect_reflected_trait_marker(&self);
                }

                impl<T: ?Sized> __QubitReflectExternalTraitGuard for T {
                    fn __qubit_reflect_reflected_trait_marker(&self) {}
                }

                fn __qubit_reflect_assert_external_trait<T>(value: &T)
                where
                    T: #trait_guard_path + Sized + 'static,
                {
                    // A reflected trait contributes a same-named method with
                    // a distinct return type, making this compile-time guard
                    // ambiguous instead of silently accepting an external ID.
                    let _: () = value.__qubit_reflect_reflected_trait_marker();
                }

                fn external_trait_runtime_identity() -> #facade::__private::codegen_v4::registration::RuntimeIdentity {
                    #facade::__private::codegen_v4::registration::RuntimeIdentity::Trait(
                        #facade::__private::codegen_v4::descriptor::TraitId::External(
                            #facade::__private::codegen_v4::identity::ExternalTraitId::new(#id)
                                .expect("validated external trait ID"),
                        ),
                    )
                }

                fn external_trait_payload() -> #facade::__private::codegen_v4::registration::FragmentPayload {
                    static DESCRIPTOR: ::std::sync::OnceLock<#facade::__private::codegen_v4::descriptor::TraitDefinitionDescriptor> =
                        ::std::sync::OnceLock::new();
                    let descriptor = DESCRIPTOR.get_or_init(|| #facade::__private::codegen_v4::descriptor::TraitDefinitionDescriptor::new(
                        #facade::__private::codegen_v4::descriptor::TraitId::External(
                            #facade::__private::codegen_v4::identity::ExternalTraitId::new(#id)
                                .expect("validated external trait ID"),
                        ),
                        #rust_name, #rust_path, #rust_name,
                        #facade::__private::codegen_v4::descriptor::TraitCompleteness::ExternalIncomplete,
                        ::std::boxed::Box::leak(::std::boxed::Box::new(
                            #facade::__private::codegen_v4::expression::GenericDefinitionDescriptor::new(
                                ::std::vec::Vec::<#facade::__private::codegen_v4::expression::GenericParameterDescriptor>::new(),
                                ::std::vec::Vec::<#facade::__private::codegen_v4::expression::PredicateDescriptor>::new(),
                            ),
                        )),
                    ));
                    #facade::__private::codegen_v4::registration::FragmentPayload::Trait(descriptor)
                }

                #facade::__private::codegen_v4::inventory::submit! {
                    #facade::__private::codegen_v4::registration::RegistrationFragment::new(
                        #facade::__private::codegen_v4::registration::FragmentKind::Trait,
                        #facade::__private::codegen_v4::registration::StaticFragmentIdentity::new(
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
        if let Some(trait_path) = declaration.trait_path.as_ref() {
            let target_expression = super::traits::type_expression(
                &declaration.target_type,
                &environment,
                &facade,
            );
            let trait_application = super::expression_codegen::path_type_expression(
                trait_path,
                &environment,
                &facade,
            );
            let trait_name = trait_path.source.as_str();
            quote! {
                ::std::boxed::Box::leak(::std::boxed::Box::new(
                    #facade::__private::codegen_v4::descriptor::ImplDefinitionDescriptor::new_unresolved_trait_with_expression(
                        fragment_identity(),
                        #target_expression,
                        #trait_name,
                        trait_definition.map(|definition| definition.trait_id().clone()),
                        #trait_application,
                        ::std::boxed::Box::leak(::std::boxed::Box::new(#impl_generic_definition)),
                    ),
                ))
            }
        } else {
            quote! {
                ::std::boxed::Box::leak(::std::boxed::Box::new(
                    #facade::__private::codegen_v4::descriptor::ImplDefinitionDescriptor::new(
                        fragment_identity(),
                        #facade::__private::codegen_v4::expression::TypeExpression::Concrete(
                            #facade::__private::codegen_v4::expression::concrete(
                                vec![#target_source.into()].into_boxed_slice(),
                                ::std::vec::Vec::new().into_boxed_slice(),
                                #facade::__private::codegen_v4::expression::DiagnosticText::from(#target_source),
                            ),
                        ),
                        #kind,
                        None,
                        ::std::boxed::Box::leak(::std::boxed::Box::new(#impl_generic_definition)),
                    ).expect("generated impl definition is consistent"),
                ))
            }
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

/// Expands every statically applicable invocation blocker in canonical enum
/// order.
///
/// # Parameters
///
/// - `method`: Validated method signature to analyze.
/// - `target`: Concrete implementation target type tokens.
/// - `context`: Expansion context used to emit runtime reason paths.
///
/// # Returns
///
/// Returns generated tokens for the method's ordered unavailable reasons.
#[must_use]
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
