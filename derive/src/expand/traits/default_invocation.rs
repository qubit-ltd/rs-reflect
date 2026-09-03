// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Invocation adapters for reflected trait default methods.

use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::LitStr;

use crate::expand::ExpansionContext;
use crate::ir::GenericBoundIr;
use crate::ir::MethodIr;
use crate::ir::PathArgumentIr;
use crate::ir::PathArgumentsIr;
use crate::ir::ReceiverKindIr;
use crate::ir::ReturnTypeIr;
use crate::ir::TypeIr;
use crate::ir::TypeKindIr;

/// Generates one concrete local adapter hook for a safely erasable default
/// method and the payload entry that exposes it to reflected impl expansion.
pub(super) fn default_method_invocation_adapter(
    method: &MethodIr,
    index: usize,
    suffix: &str,
    trait_name: &LitStr,
    hook_type_bounds: &[TokenStream],
    facade: &TokenStream,
    _context: &ExpansionContext,
) -> Option<(TokenStream, TokenStream)> {
    let target = quote!(Self);
    let has_unproven_associated_type = method
        .parameters
        .iter()
        .any(|parameter| type_contains_associated_type(&parameter.ty))
        || matches!(
            &method.return_type,
            ReturnTypeIr::Type(ty) if type_contains_associated_type(ty)
        );
    let invocation_plan = crate::expand::invocation::analysis::analyze_method(
        method,
        crate::expand::invocation::analysis::MethodContext::trait_default(
            &target,
            has_unproven_associated_type,
        ),
    )
    .expect("validated method analysis is infallible");
    let typed_owned_receiver = invocation_plan.owned_receiver_type().cloned();
    if let Some(pinned_mutable) = invocation_plan.pinned_receiver_mutability() {
        return invocation_plan.is_executable().then(|| {
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
    if !invocation_plan.is_executable() {
        return None;
    }

    let adapter_name = format_ident!("__qubit_reflect_invoke_default_{suffix}_{index}");
    let method_name = &method.name;
    debug_assert_eq!(invocation_plan.parameter_count(), method.parameters.len());
    let thread_safe = invocation_plan.modes.thread_safe;
    let catching_requested = invocation_plan.modes.catching;
    let mode = if thread_safe {
        quote!(#facade::__private::codegen_v1::value::ThreadSafe)
    } else {
        quote!(#facade::__private::codegen_v1::value::Local)
    };
    let thread_safe_assertions = thread_safe.then(|| {
        crate::expand::invocation::emit::thread_safe_assertions(
            method,
            &target,
            typed_owned_receiver.as_ref(),
            None,
        )
    });
    let catching_assertions = catching_requested.then(|| {
        crate::expand::invocation::emit::catching_assertions(
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
        quote!(#facade::__private::codegen_v1::invoke::ReceiverExpectation::owned::<Self>())
    } else if let Some(receiver_type) = &typed_owned_receiver {
        quote!(#facade::__private::codegen_v1::invoke::ReceiverExpectation::owned::<#receiver_type>())
    } else {
        match method.receiver.as_ref().map(|receiver| receiver.kind) {
            Some(ReceiverKindIr::Value) => {
                quote!(#facade::__private::codegen_v1::invoke::ReceiverExpectation::owned::<Self>())
            }
            Some(ReceiverKindIr::MutableReference) => {
                quote!(#facade::__private::codegen_v1::invoke::ReceiverExpectation::borrowed_mut::<Self>())
            }
            Some(ReceiverKindIr::SharedReference) => {
                quote!(#facade::__private::codegen_v1::invoke::ReceiverExpectation::borrowed::<Self>())
            }
            Some(ReceiverKindIr::Typed) => return None,
            None => quote!(#facade::__private::codegen_v1::invoke::ReceiverExpectation::none()),
        }
    };
    let receiver_binding = match method.receiver.as_ref().map(|receiver| receiver.kind) {
        Some(ReceiverKindIr::Value) => quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver: Self = match receiver {
                Some(#facade::__private::codegen_v1::invoke::InvocationReceiver::Owned(value)) =>
                    #facade::__private::codegen_v1::value::DynamicOwned::<#mode>::downcast::<Self>(value)
                        .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                _ => unreachable!("validation checked receiver mode"),
            };
        },
        Some(ReceiverKindIr::Typed) if typed_owned_receiver.is_some() => {
            let receiver_type = typed_owned_receiver.as_ref().expect("checked above");
            quote! {
                let (receiver, arguments) = validated.into_parts();
                let receiver: #receiver_type = match receiver {
                    Some(#facade::__private::codegen_v1::invoke::InvocationReceiver::Owned(value)) =>
                        #facade::__private::codegen_v1::value::DynamicOwned::<#mode>::downcast::<#receiver_type>(value)
                            .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                    _ => unreachable!("validation checked receiver mode"),
                };
            }
        }
        Some(ReceiverKindIr::MutableReference) => quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver: &mut Self = match receiver {
                Some(#facade::__private::codegen_v1::invoke::InvocationReceiver::Mut(value)) =>
                    #facade::__private::codegen_v1::value::DynamicMut::<#mode>::downcast::<Self>(value)
                        .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                _ => unreachable!("validation checked receiver mode"),
            };
        },
        Some(ReceiverKindIr::SharedReference) => quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver: &Self = match receiver {
                Some(#facade::__private::codegen_v1::invoke::InvocationReceiver::Ref(value)) =>
                    #facade::__private::codegen_v1::value::DynamicRef::<#mode>::downcast::<Self>(value)
                        .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                Some(#facade::__private::codegen_v1::invoke::InvocationReceiver::Mut(value)) => {
                    let value = #facade::__private::codegen_v1::value::DynamicMut::<#mode>::downcast::<Self>(value)
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
        .map(|parameter| crate::expand::invocation::emit::argument_expectation(parameter, facade))
        .collect();
    let argument_bindings: Vec<_> = method
        .parameters
        .iter()
        .map(|parameter| crate::expand::invocation::emit::argument_binding(parameter, facade, &mode))
        .collect();
    let call_arguments: Vec<_> = method
        .parameters
        .iter()
        .map(|parameter| format_ident!("__qubit_reflect_argument_{}", parameter.index))
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
            .then(|| quote!(#facade::__private::codegen_v1::invoke::BorrowOrigin::Receiver)),
    )
    .flatten()
    .chain(
        method
            .parameters
            .iter()
            .filter(|parameter| matches!(parameter.ty.kind, TypeKindIr::Reference { .. }))
            .map(|parameter| {
                let index = parameter.index;
                quote!(#facade::__private::codegen_v1::invoke::BorrowOrigin::Parameter(#index))
            }),
    )
    .collect();
    let output = match (method.qualifiers.is_async, &method.return_type) {
        (false, ReturnTypeIr::Unit) => quote! {
            #receiver_binding
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            #call;
            #facade::__private::codegen_v1::invoke::InvocationOutput::Unit
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
                quote!(#facade::__private::codegen_v1::value::DynamicRef::<#mode>::new_str(#call))
            } else {
                quote!(#facade::__private::codegen_v1::value::DynamicRef::<#mode>::new(#call))
            };
            quote! {
                #receiver_binding
                let mut arguments = arguments.into_vec().into_iter();
                #(#argument_bindings)*
                #facade::__private::codegen_v1::invoke::InvocationOutput::Ref {
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
                quote!(#facade::__private::codegen_v1::value::DynamicMut::<#mode>::new_str_mut(#call))
            } else {
                quote!(#facade::__private::codegen_v1::value::DynamicMut::<#mode>::new(#call))
            };
            quote! {
                #receiver_binding
                let mut arguments = arguments.into_vec().into_iter();
                #(#argument_bindings)*
                #facade::__private::codegen_v1::invoke::InvocationOutput::Mut {
                    value: #value,
                    origin: #facade::__private::codegen_v1::invoke::BorrowOrigin::Receiver,
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
            #facade::__private::codegen_v1::invoke::InvocationOutput::Owned(
                #facade::__private::codegen_v1::value::DynamicOwned::<#mode>::new(#call),
            )
        },
        (true, ReturnTypeIr::Unit) => quote! {
            #receiver_binding
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            #facade::__private::codegen_v1::invoke::InvocationOutput::Future(
                #facade::__private::codegen_v1::invoke::ReflectedFuture::<#mode>::new(async move {
                    #call.await;
                    #facade::__private::codegen_v1::invoke::InvocationOutput::Unit
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
            #facade::__private::codegen_v1::invoke::InvocationOutput::Future(
                #facade::__private::codegen_v1::invoke::ReflectedFuture::<#mode>::new(async move {
                    match #call.await {}
                }),
            )
        },
        (true, ReturnTypeIr::Type(_)) => quote! {
            #receiver_binding
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            #facade::__private::codegen_v1::invoke::InvocationOutput::Future(
                #facade::__private::codegen_v1::invoke::ReflectedFuture::<#mode>::new(async move {
                    #facade::__private::codegen_v1::invoke::InvocationOutput::Owned(
                        #facade::__private::codegen_v1::value::DynamicOwned::<#mode>::new(#call.await),
                    )
                }),
            )
        },
    };
    let catching_adapter_binding = if catching_requested {
        let catching_call = match &method.return_type {
            ReturnTypeIr::Unit => quote! {
                #call;
                #facade::__private::codegen_v1::invoke::InvocationOutput::Unit
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
                    quote!(#facade::__private::codegen_v1::value::DynamicRef::<#mode>::new_str(#call))
                } else {
                    quote!(#facade::__private::codegen_v1::value::DynamicRef::<#mode>::new(#call))
                };
                quote! {
                    #facade::__private::codegen_v1::invoke::InvocationOutput::Ref {
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
                    quote!(#facade::__private::codegen_v1::value::DynamicMut::<#mode>::new_str_mut(#call))
                } else {
                    quote!(#facade::__private::codegen_v1::value::DynamicMut::<#mode>::new(#call))
                };
                quote! {
                    #facade::__private::codegen_v1::invoke::InvocationOutput::Mut {
                        value: #value,
                        origin: #facade::__private::codegen_v1::invoke::BorrowOrigin::Receiver,
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
                #facade::__private::codegen_v1::invoke::InvocationOutput::Owned(
                    #facade::__private::codegen_v1::value::DynamicOwned::<#mode>::new(#call),
                )
            },
        };
        quote! {
            #[cfg(panic = "unwind")]
            let catching_adapter: #facade::__private::codegen_v1::invoke::CatchingInvocationAdapter<#mode> = |invocation| {
                #catching_assertions
                let identity = #facade::__private::codegen_v1::identity::MemberId::new(
                    #trait_name,
                    "default-method",
                    #index,
                    #facade::__private::codegen_v1::identity::FragmentIdentity::new(
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
                    Err(payload) => Ok(Err(#facade::__private::codegen_v1::invoke::InvocationPanic::new(identity, payload))),
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
            invocation: #facade::__private::codegen_v1::invoke::Invocation<'call, #mode>,
        ) -> ::core::result::Result<
            #facade::__private::codegen_v1::invoke::InvocationOutput<'call, #mode>,
            #facade::__private::codegen_v1::invoke::InvocationFailure<'call, #mode>,
        >
        where
            Self: Sized + 'static,
            #(#hook_type_bounds,)*
        {
            #thread_safe_assertions
            let identity = #facade::__private::codegen_v1::identity::MemberId::new(
                #trait_name,
                "default-method",
                #index,
                #facade::__private::codegen_v1::identity::FragmentIdentity::new(
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
        quote!(#facade::__private::codegen_v1::descriptor::InvocationAdapter::thread_safe_with_catching(
            Self::#adapter_name,
            catching_adapter,
        ))
    } else if catching_requested {
        quote!(#facade::__private::codegen_v1::descriptor::InvocationAdapter::local_with_catching(
            Self::#adapter_name,
            catching_adapter,
        ))
    } else if thread_safe {
        quote!(#facade::__private::codegen_v1::descriptor::InvocationAdapter::thread_safe(Self::#adapter_name))
    } else {
        quote!(#facade::__private::codegen_v1::descriptor::InvocationAdapter::local(Self::#adapter_name))
    };
    let unavailable_catching_constructor = if thread_safe {
        quote!(#facade::__private::codegen_v1::descriptor::InvocationAdapter::thread_safe_with_unavailable_catching(
            Self::#adapter_name,
        ))
    } else {
        quote!(#facade::__private::codegen_v1::descriptor::InvocationAdapter::local_with_unavailable_catching(
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
                as &'static #facade::__private::codegen_v1::descriptor::InvocationAdapter)
        }}
    } else {
        quote! {
            Some(::std::boxed::Box::leak(::std::boxed::Box::new(
                #adapter_constructor,
            )) as &'static #facade::__private::codegen_v1::descriptor::InvocationAdapter)
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
    trait_name: &LitStr,
    hook_type_bounds: &[TokenStream],
    facade: &TokenStream,
    pinned_mutable: bool,
) -> (TokenStream, TokenStream) {
    let adapter_name = format_ident!("__qubit_reflect_invoke_default_pinned_{suffix}_{index}");
    let method_name = &method.name;
    let mode = quote!(#facade::__private::codegen_v1::value::Local);
    let parameter_expectations: Vec<_> = method
        .parameters
        .iter()
        .map(|parameter| crate::expand::invocation::emit::argument_expectation(parameter, facade))
        .collect();
    let argument_bindings: Vec<_> = method
        .parameters
        .iter()
        .map(|parameter| crate::expand::invocation::emit::argument_binding(parameter, facade, &mode))
        .collect();
    let call_arguments: Vec<_> = method
        .parameters
        .iter()
        .map(|parameter| format_ident!("__qubit_reflect_argument_{}", parameter.index))
        .collect();
    let invocation_type = if pinned_mutable {
        quote!(#facade::__private::codegen_v1::invoke::PinnedMutInvocation<'call, Self, #mode>)
    } else {
        quote!(#facade::__private::codegen_v1::invoke::PinnedRefInvocation<'call, Self, #mode>)
    };
    let failure_type = if pinned_mutable {
        quote!(#facade::__private::codegen_v1::invoke::PinnedMutInvocationFailure<'call, Self, #mode>)
    } else {
        quote!(#facade::__private::codegen_v1::invoke::PinnedRefInvocationFailure<'call, Self, #mode>)
    };
    let adapter_type = if pinned_mutable {
        quote!(#facade::__private::codegen_v1::invoke::PinnedMutAdapter<Self, #mode>)
    } else {
        quote!(#facade::__private::codegen_v1::invoke::PinnedRefAdapter<Self, #mode>)
    };
    let call = quote!(Self::#method_name(receiver, #(#call_arguments),*));
    let output = match method.return_type {
        ReturnTypeIr::Unit => quote! {
            #call;
            #facade::__private::codegen_v1::invoke::InvocationOutput::Unit
        },
        ReturnTypeIr::Type(TypeIr {
            kind: TypeKindIr::Never,
            ..
        }) => quote!(match #call {}),
        ReturnTypeIr::Type(_) => quote! {
            #facade::__private::codegen_v1::invoke::InvocationOutput::Owned(
                #facade::__private::codegen_v1::value::DynamicOwned::<#mode>::new(#call),
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
            #facade::__private::codegen_v1::invoke::InvocationOutput<'call, #mode>,
            #failure_type,
        >
        where
            Self: Sized + 'static,
            #(#hook_type_bounds,)*
        {
            let identity = #facade::__private::codegen_v1::identity::MemberId::new(
                #trait_name,
                "default-method",
                #index,
                #facade::__private::codegen_v1::identity::FragmentIdentity::new(
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
        quote!(#facade::__private::codegen_v1::descriptor::InvocationAdapter::pinned_mut_local(adapter))
    } else {
        quote!(#facade::__private::codegen_v1::descriptor::InvocationAdapter::pinned_ref_local(adapter))
    };
    let adapter_entry = quote! {{
        let adapter: #adapter_type = Self::#adapter_name;
        let adapter: &'static #adapter_type =
            ::std::boxed::Box::leak(::std::boxed::Box::new(adapter));
        Some(::std::boxed::Box::leak(::std::boxed::Box::new(#constructor))
            as &'static #facade::__private::codegen_v1::descriptor::InvocationAdapter)
    }};
    (adapter_item, adapter_entry)
}

/// Returns whether a dynamic signature contains an associated type whose
/// concrete `'static` bound cannot be proven at the trait declaration site.
pub(super) fn type_contains_associated_type(ty: &TypeIr) -> bool {
    match &ty.kind {
        TypeKindIr::Path(path) => path_contains_associated_type(path),
        TypeKindIr::Reference { element, .. }
        | TypeKindIr::Slice(element)
        | TypeKindIr::Pointer { element, .. } => type_contains_associated_type(element),
        TypeKindIr::Tuple(elements) => elements.iter().any(type_contains_associated_type),
        TypeKindIr::Array { element, .. } => type_contains_associated_type(element),
        TypeKindIr::BareFunction { inputs, output, .. } => {
            inputs.iter().any(type_contains_associated_type)
                || output.as_deref().is_some_and(type_contains_associated_type)
        }
        TypeKindIr::TraitObject { bounds, .. } | TypeKindIr::ImplTrait { bounds } => {
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
                        PathArgumentIr::Type(ty) | PathArgumentIr::AssociatedType { ty, .. } => {
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
                        || output.as_deref().is_some_and(type_contains_associated_type)
                }
            })
}

/// Checks whether one trait or lifetime bound contains an associated binding.
fn bound_contains_associated_type(bound: &GenericBoundIr) -> bool {
    match bound {
        GenericBoundIr::Trait { path, .. } => path_contains_associated_type(path),
        GenericBoundIr::Lifetime(_) => false,
        GenericBoundIr::Other(_) => true,
    }
}

