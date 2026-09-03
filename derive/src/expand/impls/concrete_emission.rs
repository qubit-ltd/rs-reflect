// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Final token emission for one concrete reflected impl.

// qubit-style: allow explicit-imports

use proc_macro2::TokenStream;
use quote::quote;

use super::concrete_impl_emission::ConcreteImplEmission;

/// Emits the isolated registration module for one concrete impl.
pub(super) fn emit(emission: ConcreteImplEmission) -> TokenStream {
    let ConcreteImplEmission {
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
    } = emission;
    quote! {
        #retained

        #[doc(hidden)]
        mod #module {
            use super::*;

            #applicability_witness

            #(#invocation_adapter_definitions)*
            #(#generic_specialization_adapter_definitions)*

            fn fragment_identity() -> #facade::__private::codegen_v2::identity::FragmentIdentity {
                #facade::__private::codegen_v2::identity::FragmentIdentity::new(
                    env!("CARGO_PKG_NAME"), module_path!(), #line, #column,
                    "impl", #fingerprint,
                )
            }

            fn runtime_identity() -> #facade::__private::codegen_v2::registration::RuntimeIdentity {
                #facade::__private::codegen_v2::registration::RuntimeIdentity::Impl(
                    ::std::any::TypeId::of::<#target>(),
                )
            }

            fn payload() -> #facade::__private::codegen_v2::registration::FragmentPayload {
                static DESCRIPTOR: ::std::sync::OnceLock<#facade::__private::codegen_v2::descriptor::ImplDescriptor> =
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
                        Option<&'static #facade::__private::codegen_v2::descriptor::TraitDefinitionDescriptor>,
                        Option<&'static #facade::__private::codegen_v2::descriptor::TraitDescriptor>,
                        &'static [Option<&'static #facade::__private::codegen_v2::descriptor::InvocationAdapter>],
                        &'static [&'static [#facade::__private::codegen_v2::descriptor::InvocationUnavailableReason]],
                        &'static [Option<#facade::__private::codegen_v2::descriptor::TypeDescriptorResolver>],
                        &'static [Option<&'static #facade::__private::codegen_v2::descriptor::AssociatedConstReader>],
                    ) = #trait_setup;
                    let definition: &'static #facade::__private::codegen_v2::descriptor::ImplDefinitionDescriptor =
                        #definition_setup;
                    let methods: &'static [#facade::__private::codegen_v2::descriptor::MethodDescriptor] =
                        ::std::boxed::Box::leak(::std::vec![#(#method_entries),*].into_boxed_slice());
                    let adapters: &[Option<&'static #facade::__private::codegen_v2::descriptor::InvocationAdapter>] =
                        &[#(#invocation_adapter_entries),*];
                    let declared_unavailable_reasons: &[&[
                        #facade::__private::codegen_v2::descriptor::InvocationUnavailableReason
                    ]] = &[#(#invocation_unavailable_reason_entries),*];
                    let specializations: ::std::vec::Vec<::std::vec::Vec<(
                        ::std::boxed::Box<[#facade::__private::codegen_v2::expression::GenericArgument]>,
                        Option<&'static #facade::__private::codegen_v2::descriptor::InvocationAdapter>,
                        &'static [#facade::__private::codegen_v2::descriptor::InvocationUnavailableReason],
                    )>> = ::std::vec![#(#method_specialization_entries),*];
                    let method_instances = if let Some(trait_descriptor) = implemented_trait {
                        trait_descriptor.methods().iter().enumerate().flat_map(|(declaration_index, declaration)| {
                            match methods
                                .iter()
                                .enumerate()
                                .find(|(_, method)| method.rust_name() == declaration.rust_name())
                            {
                                Some((method_index, method)) => {
                                    if !specializations[method_index].is_empty() {
                                        return specializations[method_index]
                                            .iter()
                                            .cloned()
                                            .map(|(arguments, adapter, reasons)| {
                                                let unavailable_reasons = if adapter.is_some() {
                                                    ::std::vec![].into_boxed_slice()
                                                } else {
                                                    reasons.to_vec().into_boxed_slice()
                                                };
                                                #facade::__private::codegen_v2::descriptor::MethodInstanceDescriptor::with_arguments(
                                                    declaration,
                                                    Some(method),
                                                    #facade::__private::codegen_v2::descriptor::MethodImplementationSource::Overridden,
                                                    adapter,
                                                    arguments,
                                                    unavailable_reasons,
                                                ).expect("generated trait method specialization is consistent")
                                            })
                                            .collect::<::std::vec::Vec<_>>();
                                    }
                                    let adapter = adapters[method_index];
                                    let unavailable_reasons: ::std::boxed::Box<[
                                        #facade::__private::codegen_v2::descriptor::InvocationUnavailableReason
                                    ]> = if adapter.is_some() {
                                        ::std::vec![].into_boxed_slice()
                                    } else {
                                        declared_unavailable_reasons[method_index]
                                            .to_vec()
                                            .into_boxed_slice()
                                    };
                                    ::std::vec![#facade::__private::codegen_v2::descriptor::MethodInstanceDescriptor::new(
                                        declaration,
                                        Some(method),
                                        #facade::__private::codegen_v2::descriptor::MethodImplementationSource::Overridden,
                                        adapter,
                                        unavailable_reasons,
                                    ).expect("generated trait method instance is consistent")]
                                },
                                None if declaration.has_default() => {
                                    let adapter = default_method_adapters
                                        .get(declaration_index)
                                        .copied()
                                        .flatten();
                                    let unavailable_reasons: ::std::boxed::Box<[
                                        #facade::__private::codegen_v2::descriptor::InvocationUnavailableReason
                                    ]> = if adapter.is_some() {
                                        ::std::vec![].into_boxed_slice()
                                    } else {
                                        default_method_unavailable_reasons
                                            .get(declaration_index)
                                            .copied()
                                            .unwrap_or(&[
                                                #facade::__private::codegen_v2::descriptor::InvocationUnavailableReason::DisabledByPolicy,
                                            ])
                                            .to_vec()
                                            .into_boxed_slice()
                                    };
                                    ::std::vec![#facade::__private::codegen_v2::descriptor::MethodInstanceDescriptor::new(
                                        declaration,
                                        None,
                                        #facade::__private::codegen_v2::descriptor::MethodImplementationSource::Defaulted,
                                        adapter,
                                        unavailable_reasons,
                                    ).expect("generated default trait method instance is consistent")]
                                },
                                None => ::std::vec![#facade::__private::codegen_v2::descriptor::MethodInstanceDescriptor::new(
                                    declaration, None,
                                    #facade::__private::codegen_v2::descriptor::MethodImplementationSource::Required,
                                    None,
                                    ::std::boxed::Box::new([
                                        #facade::__private::codegen_v2::descriptor::InvocationUnavailableReason::DisabledByPolicy,
                                    ]),
                                ).expect("generated required trait method instance is consistent")],
                            }
                        }).collect()
                    } else {
                        methods.iter().zip(adapters.iter().copied()).enumerate().flat_map(|(index, (method, adapter))| {
                            if !specializations[index].is_empty() {
                                return specializations[index].iter().cloned().map(|(arguments, adapter, reasons)| {
                                    let unavailable_reasons: ::std::boxed::Box<[
                                        #facade::__private::codegen_v2::descriptor::InvocationUnavailableReason
                                    ]> = if adapter.is_some() {
                                        ::std::vec![].into_boxed_slice()
                                    } else {
                                        reasons.to_vec().into_boxed_slice()
                                    };
                                    #facade::__private::codegen_v2::descriptor::MethodInstanceDescriptor::with_arguments(
                                        method,
                                        None,
                                        #facade::__private::codegen_v2::descriptor::MethodImplementationSource::Declared,
                                        adapter,
                                        arguments,
                                        unavailable_reasons,
                                    ).expect("generated generic method specialization is consistent")
                                }).collect::<::std::vec::Vec<_>>();
                            }
                            let unavailable_reasons: ::std::boxed::Box<[
                                #facade::__private::codegen_v2::descriptor::InvocationUnavailableReason
                            ]> = if adapter.is_some() {
                                ::std::vec![] .into_boxed_slice()
                            } else {
                                declared_unavailable_reasons[index]
                                    .to_vec()
                                    .into_boxed_slice()
                            };
                            ::std::vec![#facade::__private::codegen_v2::descriptor::MethodInstanceDescriptor::new(
                                method,
                                None,
                                #facade::__private::codegen_v2::descriptor::MethodImplementationSource::Declared,
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
                            #facade::__private::codegen_v2::descriptor::AssociatedTypeBindingDescriptor::new(
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
                                _ => #facade::__private::codegen_v2::descriptor::AssociatedConstImplementationSource::Defaulted,
                            };
                            let reader = match declaration.rust_name() {
                                #(#associated_const_reader_override_arms,)*
                                _ => associated_const_readers
                                    .get(index)
                                    .copied()
                                    .flatten(),
                            };
                            #facade::__private::codegen_v2::descriptor::AssociatedConstBindingDescriptor::new(
                                declaration,
                                implementation_source,
                                reader,
                            )
                        }).collect()
                    });
                    let mut builder = #facade::__private::codegen_v2::descriptor::ImplDescriptor::builder(
                        definition,
                        || <#target as #facade::__private::codegen_v2::Reflect>::type_descriptor(),
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
                #facade::__private::codegen_v2::registration::FragmentPayload::Impl(descriptor)
            }

            #facade::__private::codegen_v2::inventory::submit! {
                #facade::__private::codegen_v2::registration::RegistrationFragment::new(
                    #facade::__private::codegen_v2::registration::FragmentKind::Impl,
                    #facade::__private::codegen_v2::registration::StaticFragmentIdentity::new(
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
