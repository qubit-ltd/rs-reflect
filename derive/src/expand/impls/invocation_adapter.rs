// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Invocation adapter generation for concrete reflected impl methods.

use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;

use super::specialization_codegen::typed_extension_receiver_type;
use crate::ir::MethodIr;
use crate::ir::ReceiverKindIr;
use crate::ir::ReturnTypeIr;
use crate::ir::TypeIr;
use crate::ir::TypeKindIr;

/// Generates one executable invocation adapter for a concrete impl method.
pub(super) fn definition(
    method: &MethodIr,
    index: usize,
    target: &TokenStream,
    trait_call_path: &Option<TokenStream>,
    target_source: &str,
    facade: &TokenStream,
) -> Option<TokenStream> {
    definition_for_call(
        method,
        index,
        target,
        trait_call_path,
        target_source,
        facade,
        &[],
        None,
    )
}

/// Generates an adapter for a concrete method specialization through the same
/// analyzer and emitter used by ordinary methods.
pub(super) fn specialization_definition(
    method: &MethodIr,
    method_index: usize,
    specialization: (usize, &[TokenStream]),
    target: &TokenStream,
    trait_call_path: &Option<TokenStream>,
    target_source: &str,
    facade: &TokenStream,
) -> Option<TokenStream> {
    let (specialization_index, generic_arguments) = specialization;
    definition_for_call(
        method,
        method_index,
        target,
        trait_call_path,
        target_source,
        facade,
        generic_arguments,
        Some(specialization_index),
    )
}

/// Returns the static descriptor name shared by specialization emission and
/// registration.
pub(super) fn specialization_descriptor_name(
    method_index: usize,
    specialization_index: usize,
) -> Ident {
    format_ident!(
        "__QUBIT_REFLECT_GENERIC_SPECIALIZATION_ADAPTER_{method_index}_{specialization_index}"
    )
}

#[allow(clippy::too_many_arguments)]
fn definition_for_call(
    method: &MethodIr,
    index: usize,
    target: &TokenStream,
    trait_call_path: &Option<TokenStream>,
    target_source: &str,
    facade: &TokenStream,
    generic_arguments: &[TokenStream],
    specialization_index: Option<usize>,
) -> Option<TokenStream> {
    let adapter_suffix =
        specialization_index.map_or_else(|| index.to_string(), |value| format!("{index}_{value}"));
    let descriptor_name = specialization_index.map_or_else(
        || format_ident!("__QUBIT_REFLECT_INVOCATION_ADAPTER_{index}"),
        |value| specialization_descriptor_name(index, value),
    );
    let generic_call = (!generic_arguments.is_empty()).then(|| quote!(::<#(#generic_arguments),*>));
    let typed_extension_receiver = method
        .receiver
        .as_ref()
        .and_then(|receiver| typed_extension_receiver_type(receiver, target));
    let mut method_context =
        crate::expand::invocation::analysis::MethodContext::implementation(target);
    method_context.extension_receiver = typed_extension_receiver.clone();
    let invocation_plan =
        crate::expand::invocation::analysis::analyze_method(method, method_context)
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
            let adapter_name = format_ident!("__qubit_reflect_invoke_pinned_{adapter_suffix}");
            let adapter_token_name =
                format_ident!("__QUBIT_REFLECT_PINNED_ADAPTER_{adapter_suffix}");
            let parameter_expectations: Vec<_> = method
                .parameters
                .iter()
                .map(|parameter| {
                    crate::expand::invocation::emit::argument_expectation(parameter, facade)
                })
                .collect();
            let mode = quote!(#facade::__private::codegen_v2::value::Local);
            let argument_bindings: Vec<_> = method
                .parameters
                .iter()
                .map(|parameter| {
                    crate::expand::invocation::emit::argument_binding(parameter, facade, &mode)
                })
                .collect();
            let call_arguments: Vec<_> = method
                .parameters
                .iter()
                .map(|parameter| format_ident!("__qubit_reflect_argument_{}", parameter.index))
                .collect();
            let invocation_type = if pinned_mutable {
                quote!(#facade::__private::codegen_v2::invoke::PinnedMutInvocation<'call, #target, #facade::__private::codegen_v2::value::Local>)
            } else {
                quote!(#facade::__private::codegen_v2::invoke::PinnedRefInvocation<'call, #target, #facade::__private::codegen_v2::value::Local>)
            };
            let failure_type = if pinned_mutable {
                quote!(#facade::__private::codegen_v2::invoke::PinnedMutInvocationFailure<'call, #target, #facade::__private::codegen_v2::value::Local>)
            } else {
                quote!(#facade::__private::codegen_v2::invoke::PinnedRefInvocationFailure<'call, #target, #facade::__private::codegen_v2::value::Local>)
            };
            let adapter_type = if pinned_mutable {
                quote!(#facade::__private::codegen_v2::invoke::PinnedMutAdapter<#target, #facade::__private::codegen_v2::value::Local>)
            } else {
                quote!(#facade::__private::codegen_v2::invoke::PinnedRefAdapter<#target, #facade::__private::codegen_v2::value::Local>)
            };
            let constructor = if pinned_mutable {
                quote!(#facade::__private::codegen_v2::descriptor::InvocationAdapter::pinned_mut_local(&#adapter_token_name))
            } else {
                quote!(#facade::__private::codegen_v2::descriptor::InvocationAdapter::pinned_ref_local(&#adapter_token_name))
            };
            let call = if let Some(trait_path) = &trait_call_path {
                quote!(<#target as #trait_path>::#method_name #generic_call (receiver, #(#call_arguments),*))
            } else {
                quote!(<#target>::#method_name #generic_call (receiver, #(#call_arguments),*))
            };
            let output = match method.return_type {
                ReturnTypeIr::Unit => quote! {
                    #call;
                    #facade::__private::codegen_v2::invoke::InvocationOutput::Unit
                },
                ReturnTypeIr::Type(TypeIr {
                    kind: TypeKindIr::Never,
                    ..
                }) => {
                    quote!(match #call {})
                }
                ReturnTypeIr::Type(_) => quote! {
                    #facade::__private::codegen_v2::invoke::InvocationOutput::Owned(
                        #facade::__private::codegen_v2::value::DynamicOwned::<#facade::__private::codegen_v2::value::Local>::new(
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
                    #facade::__private::codegen_v2::invoke::InvocationOutput<'call, #facade::__private::codegen_v2::value::Local>,
                    #failure_type,
                > {
                    let identity = #facade::__private::codegen_v2::identity::MemberId::new(
                        #target_source, "method", #index, fragment_identity(),
                    );
                    let validated = invocation.validate(&identity, &[#(#parameter_expectations),*])?;
                    let (receiver, arguments) = validated.into_parts();
                    let mut arguments = arguments.into_vec().into_iter();
                    #(#argument_bindings)*
                    #invocation_result
                }

                static #adapter_token_name: #adapter_type = #adapter_name;
                static #descriptor_name: #facade::__private::codegen_v2::descriptor::InvocationAdapter = #constructor;
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
    let adapter_name = format_ident!("__qubit_reflect_invoke_{adapter_suffix}");
    let catching_adapter_name = format_ident!("__qubit_reflect_invoke_catching_{adapter_suffix}");
    let thread_safe = invocation_plan.modes.thread_safe;
    let catching_requested = invocation_plan.modes.catching;
    let mode = if thread_safe {
        quote!(#facade::__private::codegen_v2::value::ThreadSafe)
    } else {
        quote!(#facade::__private::codegen_v2::value::Local)
    };
    let thread_safe_assertions = thread_safe.then(|| {
        crate::expand::invocation::emit::thread_safe_assertions(
            method,
            target,
            typed_owned_receiver.as_ref(),
            typed_extension_receiver.as_ref(),
        )
    });
    let catching_assertions = catching_requested.then(|| {
        crate::expand::invocation::emit::catching_assertions(
            method,
            target,
            typed_owned_receiver.as_ref(),
            typed_extension_receiver.as_ref(),
        )
    });
    let adapter_constructor = if catching_requested && thread_safe {
        quote!(#facade::__private::codegen_v2::descriptor::InvocationAdapter::thread_safe_with_catching(
            #adapter_name,
            #catching_adapter_name,
        ))
    } else if catching_requested {
        quote!(#facade::__private::codegen_v2::descriptor::InvocationAdapter::local_with_catching(
            #adapter_name,
            #catching_adapter_name,
        ))
    } else if thread_safe {
        quote!(#facade::__private::codegen_v2::descriptor::InvocationAdapter::thread_safe(#adapter_name))
    } else {
        quote!(#facade::__private::codegen_v2::descriptor::InvocationAdapter::local(#adapter_name))
    };
    let unavailable_catching_constructor = if thread_safe {
        quote!(#facade::__private::codegen_v2::descriptor::InvocationAdapter::thread_safe_with_unavailable_catching(
            #adapter_name,
        ))
    } else {
        quote!(#facade::__private::codegen_v2::descriptor::InvocationAdapter::local_with_unavailable_catching(
            #adapter_name,
        ))
    };
    let adapter_definition = if catching_requested {
        quote! {
            #[cfg(panic = "unwind")]
            static #descriptor_name: #facade::__private::codegen_v2::descriptor::InvocationAdapter =
                #adapter_constructor;
            #[cfg(panic = "abort")]
            static #descriptor_name: #facade::__private::codegen_v2::descriptor::InvocationAdapter =
                #unavailable_catching_constructor;
        }
    } else {
        quote! {
            static #descriptor_name: #facade::__private::codegen_v2::descriptor::InvocationAdapter =
                #adapter_constructor;
        }
    };
    let receiver_expectation = if matches!(
        method.receiver.as_ref().map(|receiver| receiver.kind),
        Some(ReceiverKindIr::Value)
    ) {
        quote!(#facade::__private::codegen_v2::invoke::ReceiverExpectation::owned::<#target>())
    } else if let Some(receiver_type) = &typed_owned_receiver {
        quote!(#facade::__private::codegen_v2::invoke::ReceiverExpectation::owned::<#receiver_type>())
    } else if matches!(
        method.receiver.as_ref().map(|receiver| receiver.kind),
        Some(ReceiverKindIr::MutableReference)
    ) {
        quote!(#facade::__private::codegen_v2::invoke::ReceiverExpectation::borrowed_mut::<#target>())
    } else if method.receiver.is_some() {
        quote!(#facade::__private::codegen_v2::invoke::ReceiverExpectation::borrowed::<#target>())
    } else {
        quote!(#facade::__private::codegen_v2::invoke::ReceiverExpectation::none())
    };
    let receiver_binding = if let Some(receiver_type) = &typed_extension_receiver {
        quote! {
            let registry = #facade::__private::codegen_v2::registration::ReflectRegistry::initialize().ok();
            let adapter = registry.and_then(|registry| {
                registry
                    .capabilities(::std::any::TypeId::of::<#target>())
                    .get(#facade::__private::codegen_v2::invoke::receiver_adapter_key::<#receiver_type, #mode>())
            });
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
                Some(#facade::__private::codegen_v2::invoke::InvocationReceiver::Owned(value)) =>
                    #facade::__private::codegen_v2::value::DynamicOwned::<#mode>::downcast::<#target>(value)
                        .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                _ => unreachable!("validation checked receiver mode"),
            };
        }
    } else if let Some(receiver_type) = &typed_owned_receiver {
        quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver: #receiver_type = match receiver {
                Some(#facade::__private::codegen_v2::invoke::InvocationReceiver::Owned(value)) =>
                    #facade::__private::codegen_v2::value::DynamicOwned::<#mode>::downcast::<#receiver_type>(value)
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
                Some(#facade::__private::codegen_v2::invoke::InvocationReceiver::Mut(value)) =>
                    #facade::__private::codegen_v2::value::DynamicMut::<#mode>::downcast::<#target>(value)
                        .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                _ => unreachable!("validation checked receiver mode"),
            };
        }
    } else if method.receiver.is_some() {
        quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver: &#target = match receiver {
                Some(#facade::__private::codegen_v2::invoke::InvocationReceiver::Ref(value)) =>
                    #facade::__private::codegen_v2::value::DynamicRef::<#mode>::downcast::<#target>(value)
                        .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                Some(#facade::__private::codegen_v2::invoke::InvocationReceiver::Mut(value)) => {
                    let value = #facade::__private::codegen_v2::value::DynamicMut::<#mode>::downcast::<#target>(value)
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
        .map(|parameter| crate::expand::invocation::emit::argument_expectation(parameter, facade))
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
        .map(|parameter| {
            crate::expand::invocation::emit::argument_binding(parameter, facade, &mode)
        })
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
            quote!(<#target as #trait_path>::#method_name #generic_call (receiver, #(#call_arguments),*))
        } else {
            quote!(<#target as #trait_path>::#method_name #generic_call (#(#call_arguments),*))
        }
    } else if method.receiver.is_some() {
        quote!(<#target>::#method_name #generic_call (receiver, #(#call_arguments),*))
    } else {
        quote!(<#target>::#method_name #generic_call (#(#call_arguments),*))
    };
    let borrow_origins: Vec<_> = std::iter::once(
        method
            .receiver
            .is_some()
            .then(|| quote!(#facade::__private::codegen_v2::invoke::BorrowOrigin::Receiver)),
    )
    .flatten()
    .chain(
        method
            .parameters
            .iter()
            .filter(|parameter| matches!(parameter.ty.kind, TypeKindIr::Reference { .. }))
            .map(|parameter| {
                let index = parameter.index;
                quote!(#facade::__private::codegen_v2::invoke::BorrowOrigin::Parameter(#index))
            }),
    )
    .collect();
    let output = match (method.qualifiers.is_async, &method.return_type) {
        (false, ReturnTypeIr::Unit) => quote! {
            #receiver_binding
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            #call;
            #facade::__private::codegen_v2::invoke::InvocationOutput::Unit
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
            let value = if crate::expand::invocation::analysis::is_str_type(element) {
                quote!(#facade::__private::codegen_v2::value::DynamicRef::<#mode>::new_str(#call))
            } else {
                quote!(#facade::__private::codegen_v2::value::DynamicRef::<#mode>::new(#call))
            };
            quote! {
                #receiver_binding
                let mut arguments = arguments.into_vec().into_iter();
                #(#argument_bindings)*
                #facade::__private::codegen_v2::invoke::InvocationOutput::Ref {
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
            let value = if crate::expand::invocation::analysis::is_str_type(element) {
                quote!(#facade::__private::codegen_v2::value::DynamicMut::<#mode>::new_str_mut(#call))
            } else {
                quote!(#facade::__private::codegen_v2::value::DynamicMut::<#mode>::new(#call))
            };
            quote! {
                #receiver_binding
                let mut arguments = arguments.into_vec().into_iter();
                #(#argument_bindings)*
                #facade::__private::codegen_v2::invoke::InvocationOutput::Mut {
                    value: #value,
                    origin: #facade::__private::codegen_v2::invoke::BorrowOrigin::Receiver,
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
            #facade::__private::codegen_v2::invoke::InvocationOutput::Owned(
                #facade::__private::codegen_v2::value::DynamicOwned::<#mode>::new(#call),
            )
        },
        (true, ReturnTypeIr::Unit) => quote! {
            #receiver_binding
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            #facade::__private::codegen_v2::invoke::InvocationOutput::Future(
                #facade::__private::codegen_v2::invoke::ReflectedFuture::<#mode>::new(async move {
                    #call.await;
                    #facade::__private::codegen_v2::invoke::InvocationOutput::Unit
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
            #facade::__private::codegen_v2::invoke::InvocationOutput::Future(
                #facade::__private::codegen_v2::invoke::ReflectedFuture::<#mode>::new(async move {
                    match #call.await {}
                }),
            )
        },
        (true, ReturnTypeIr::Type(_)) => quote! {
            #receiver_binding
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            #facade::__private::codegen_v2::invoke::InvocationOutput::Future(
                #facade::__private::codegen_v2::invoke::ReflectedFuture::<#mode>::new(async move {
                    #facade::__private::codegen_v2::invoke::InvocationOutput::Owned(
                        #facade::__private::codegen_v2::value::DynamicOwned::<#mode>::new(#call.await),
                    )
                }),
            )
        },
    };
    let catching_definition = if catching_requested {
        let catching_call = match &method.return_type {
            ReturnTypeIr::Unit => quote! {
                #call;
                #facade::__private::codegen_v2::invoke::InvocationOutput::Unit
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
                let value = if crate::expand::invocation::analysis::is_str_type(element) {
                    quote!(#facade::__private::codegen_v2::value::DynamicRef::<#mode>::new_str(#call))
                } else {
                    quote!(#facade::__private::codegen_v2::value::DynamicRef::<#mode>::new(#call))
                };
                quote! {
                    #facade::__private::codegen_v2::invoke::InvocationOutput::Ref {
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
                let value = if crate::expand::invocation::analysis::is_str_type(element) {
                    quote!(#facade::__private::codegen_v2::value::DynamicMut::<#mode>::new_str_mut(#call))
                } else {
                    quote!(#facade::__private::codegen_v2::value::DynamicMut::<#mode>::new(#call))
                };
                quote! {
                    #facade::__private::codegen_v2::invoke::InvocationOutput::Mut {
                        value: #value,
                        origin: #facade::__private::codegen_v2::invoke::BorrowOrigin::Receiver,
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
                #facade::__private::codegen_v2::invoke::InvocationOutput::Owned(
                    #facade::__private::codegen_v2::value::DynamicOwned::<#mode>::new(
                        #call,
                    ),
                )
            },
        };
        quote! {
            #[cfg(panic = "unwind")]
            fn #catching_adapter_name<'call>(
                invocation: #facade::__private::codegen_v2::invoke::Invocation<'call, #mode>,
            ) -> ::core::result::Result<
                ::core::result::Result<
                    #facade::__private::codegen_v2::invoke::InvocationOutput<'call, #mode>,
                    #facade::__private::codegen_v2::invoke::InvocationPanic,
                >,
                #facade::__private::codegen_v2::invoke::InvocationFailure<'call, #mode>,
            > {
                #catching_assertions
                let identity = #facade::__private::codegen_v2::identity::MemberId::new(
                    #target_source, "method", #index, fragment_identity(),
                );
                let validated = #invocation_validation;
                #receiver_binding
                let mut arguments = arguments.into_vec().into_iter();
                #(#argument_bindings)*
                match ::std::panic::catch_unwind(|| { #catching_call }) {
                    Ok(output) => Ok(Ok(output)),
                    Err(payload) => Ok(Err(#facade::__private::codegen_v2::invoke::InvocationPanic::new(identity, payload))),
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
            invocation: #facade::__private::codegen_v2::invoke::Invocation<'call, #mode>,
        ) -> ::core::result::Result<
            #facade::__private::codegen_v2::invoke::InvocationOutput<'call, #mode>,
            #facade::__private::codegen_v2::invoke::InvocationFailure<'call, #mode>,
        > {
            #thread_safe_assertions
            let identity = #facade::__private::codegen_v2::identity::MemberId::new(
                #target_source, "method", #index, fragment_identity(),
            );
            let validated = #invocation_validation;
            #invocation_result
        }

        #catching_definition

        #adapter_definition
    })
}
