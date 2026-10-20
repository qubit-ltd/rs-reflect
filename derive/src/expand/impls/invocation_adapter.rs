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

/// One compile-time-selected thread-safe adapter provider.
pub(super) struct ThreadSafeDefinition {
    pub(super) tokens: TokenStream,
    pub(super) selector: TokenStream,
}

/// Builds a thread-safe adapter behind a stable-Rust trait-bound probe.
///
/// The generated method-call trait keeps the target concrete at the call
/// boundary while the adapter function is generic over that trait. This lets
/// Rust type-check the thread-safe body only for targets and value types that
/// satisfy the corresponding `ValueFor<ThreadSafe>` bounds.
#[must_use]
pub(super) fn thread_safe_definition(
    method: &MethodIr,
    index: usize,
    target: &TokenStream,
    trait_call_path: &Option<TokenStream>,
    facade: &TokenStream,
) -> Option<ThreadSafeDefinition> {
    if method.qualifiers.is_async
        || method.qualifiers.is_unsafe
        || !method.generics.params.is_empty()
        || crate::expand::invocation::analysis::is_optional_borrow_return(&method.return_type)
        || crate::expand::invocation::analysis::return_contains_non_static_lifetime(&method.return_type)
        || method.parameters.iter().any(|parameter| {
            matches!(parameter.ty.kind, TypeKindIr::Reference { mutable: true, .. })
                || contains_self_type(&parameter.ty)
        })
    {
        return None;
    }
    let return_type = match &method.return_type {
        ReturnTypeIr::Unit => None,
        ReturnTypeIr::Type(ty)
            if !matches!(
                ty.kind,
                TypeKindIr::Reference { .. }
                    | TypeKindIr::Never
                    | TypeKindIr::ImplTrait { .. }
                    | TypeKindIr::Infer
                    | TypeKindIr::Macro
            ) && !contains_self_type(ty) =>
        {
            Some(ty.tokens.clone())
        }
        ReturnTypeIr::Type(_) => return None,
    };
    let receiver_kind = match method.receiver.as_ref() {
        None => None,
        Some(receiver) => match receiver.kind {
            ReceiverKindIr::Value => Some(ReceiverKindIr::Value),
            ReceiverKindIr::SharedReference => Some(ReceiverKindIr::SharedReference),
            ReceiverKindIr::MutableReference => Some(ReceiverKindIr::MutableReference),
            ReceiverKindIr::Typed => return None,
        },
    };
    let call_trait = format_ident!("__QUBIT_REFLECT_THREAD_SAFE_CALL_{index}");
    let probe = format_ident!("__QUBIT_REFLECT_THREAD_SAFE_PROBE_{index}");
    let selector_trait = format_ident!("__QUBIT_REFLECT_THREAD_SAFE_SELECT_{index}");
    let invoke = format_ident!("__qubit_reflect_thread_safe_invoke_{index}");
    let method_name = &method.name;
    let associated_arguments = (0..method.parameters.len())
        .map(|position| format_ident!("__QUBIT_REFLECT_ARGUMENT_TYPE_{position}"))
        .collect::<Vec<_>>();
    let implementation_parameters = method
        .parameters
        .iter()
        .enumerate()
        .map(|(position, parameter)| {
            let name = format_ident!("__qubit_reflect_call_argument_{position}");
            let ty = &parameter.ty.tokens;
            quote!(#name: #ty)
        })
        .collect::<Vec<_>>();
    let call_parameters = method
        .parameters
        .iter()
        .zip(&associated_arguments)
        .enumerate()
        .map(|(position, (parameter, associated_argument))| {
            let name = format_ident!("__qubit_reflect_call_argument_{position}");
            if matches!(parameter.ty.kind, TypeKindIr::Reference { mutable: false, .. }) {
                quote!(#name: &Self::#associated_argument)
            } else {
                quote!(#name: Self::#associated_argument)
            }
        })
        .collect::<Vec<_>>();
    let associated_argument_declarations = associated_arguments
        .iter()
        .zip(&method.parameters)
        .map(|(associated_argument, parameter)| {
            if matches!(parameter.ty.kind, TypeKindIr::Reference { mutable: false, .. }) {
                quote!(type #associated_argument: ?Sized;)
            } else {
                quote!(type #associated_argument;)
            }
        })
        .collect::<Vec<_>>();
    let associated_argument_definitions = associated_arguments
        .iter()
        .zip(&method.parameters)
        .map(|(associated_argument, parameter)| {
            let ty = match &parameter.ty.kind {
                TypeKindIr::Reference {
                    mutable: false,
                    element,
                    ..
                } => &element.tokens,
                _ => &parameter.ty.tokens,
            };
            quote!(type #associated_argument = #ty;)
        })
        .collect::<Vec<_>>();
    let argument_names = (0..method.parameters.len())
        .map(|position| format_ident!("__qubit_reflect_call_argument_{position}"))
        .collect::<Vec<_>>();
    let typed_arguments = method
        .parameters
        .iter()
        .enumerate()
        .map(|(position, parameter)| {
            let name = format_ident!("__qubit_reflect_argument_{position}");
            let associated_argument = &associated_arguments[position];
            match &parameter.ty.kind {
                TypeKindIr::Reference { mutable: false, .. } => {
                    quote!(let #name: &<T as #call_trait>::#associated_argument = arguments.next()
                        .expect("validated invocation contains each declared argument")
                        .into_ref::<<T as #call_trait>::#associated_argument>()
                        .unwrap_or_else(|_| unreachable!("validation checked argument type and mode"));)
                }
                _ => quote!(let #name: <T as #call_trait>::#associated_argument = arguments.next()
                    .expect("validated invocation contains each declared argument")
                    .into_owned::<<T as #call_trait>::#associated_argument>()
                    .unwrap_or_else(|_| unreachable!("validation checked argument type and mode"));),
            }
        })
        .collect::<Vec<_>>();
    let adapter_arguments = (0..method.parameters.len())
        .map(|position| format_ident!("__qubit_reflect_argument_{position}"))
        .collect::<Vec<_>>();
    let return_tokens = return_type.as_ref().map_or_else(|| quote!(()), |ty| quote!(#ty));
    let call_trait_receiver = match receiver_kind {
        Some(ReceiverKindIr::Value) => quote!(self),
        Some(ReceiverKindIr::SharedReference) => quote!(&self),
        Some(ReceiverKindIr::MutableReference) => quote!(&mut self),
        None => TokenStream::new(),
        Some(ReceiverKindIr::Typed) => unreachable!(),
    };
    let implementation_call = if receiver_kind.is_some() {
        if matches!(receiver_kind, Some(ReceiverKindIr::Value)) {
            if let Some(trait_path) = trait_call_path {
                quote!(<#target as #trait_path>::#method_name(self, #(#argument_names),*))
            } else {
                quote!(<#target>::#method_name(self, #(#argument_names),*))
            }
        } else if let Some(trait_path) = trait_call_path {
            quote!(<#target as #trait_path>::#method_name(self, #(#argument_names),*))
        } else {
            quote!(<#target>::#method_name(self, #(#argument_names),*))
        }
    } else if let Some(trait_path) = trait_call_path {
        quote!(<#target as #trait_path>::#method_name(#(#argument_names),*))
    } else {
        quote!(<#target>::#method_name(#(#argument_names),*))
    };
    let trait_receiver = (!call_trait_receiver.is_empty()).then(|| quote!(#call_trait_receiver,));
    let invoke_receiver = match receiver_kind {
        Some(ReceiverKindIr::Value) => quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver: T = receiver.expect("validated invocation has a receiver")
                .into_owned::<T>()
                .unwrap_or_else(|_| unreachable!("validation checked receiver type and mode"));
        },
        Some(ReceiverKindIr::SharedReference) => quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver = receiver.expect("validated invocation has a receiver")
                .into_ref::<T>()
                .unwrap_or_else(|_| unreachable!("validation checked receiver type and mode"));
        },
        Some(ReceiverKindIr::MutableReference) => quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver = receiver.expect("validated invocation has a receiver")
                .into_mut::<T>()
                .unwrap_or_else(|_| unreachable!("validation checked receiver type and mode"));
        },
        None => quote! { let (_receiver, arguments) = validated.into_parts(); },
        Some(ReceiverKindIr::Typed) => unreachable!(),
    };
    let value_mode = quote!(#facade::__private::codegen_v4::value::ThreadSafe);
    let receiver_bound = match receiver_kind {
        Some(ReceiverKindIr::Value) => {
            quote!(#facade::__private::codegen_v4::value::ValueFor<#value_mode>)
        }
        Some(ReceiverKindIr::SharedReference) => {
            quote!(#facade::__private::codegen_v4::value::ValueForRef<#value_mode>)
        }
        Some(ReceiverKindIr::MutableReference) => {
            quote!(#facade::__private::codegen_v4::value::ValueForMut<#value_mode>)
        }
        None => quote!(),
        Some(ReceiverKindIr::Typed) => unreachable!(),
    };
    let receiver_bound = (!receiver_bound.is_empty()).then(|| quote!(T: #receiver_bound + 'static,));
    let parameter_bounds = method
        .parameters
        .iter()
        .zip(&associated_arguments)
        .map(|(parameter, associated_argument)| match &parameter.ty.kind {
            TypeKindIr::Reference {
                mutable: false,
                ..
            } => {
                quote!(<T as #call_trait>::#associated_argument: #facade::__private::codegen_v4::value::ValueForRef<#value_mode>,)
            }
            _ => {
                quote!(<T as #call_trait>::#associated_argument: #facade::__private::codegen_v4::value::ValueFor<#value_mode>,)
            }
        });
    let return_bound = return_type
        .as_ref()
        .map(|_| quote!(<T as #call_trait>::Output: #facade::__private::codegen_v4::value::ValueFor<#value_mode>,));
    let parameter_bounds = parameter_bounds.collect::<Vec<_>>();
    let result = if return_type.is_some() {
        quote!(#facade::__private::codegen_v4::invoke::InvocationOutput::owned(result))
    } else {
        quote!(#facade::__private::codegen_v4::invoke::InvocationOutput::unit())
    };
    let invocation_receiver = receiver_kind.is_some().then(|| quote!(receiver,));
    let selector = quote! {
        (&&#probe::<#target>(::core::marker::PhantomData)).select_thread_safe()
    };
    let tokens = quote! {
        trait #call_trait {
            #(#associated_argument_declarations)*
            type Output;
            fn call(#trait_receiver #(#call_parameters),*) -> Self::Output;
        }

        impl #call_trait for #target {
            #(#associated_argument_definitions)*
            type Output = #return_tokens;
            fn call(#trait_receiver #(#implementation_parameters),*) -> Self::Output {
                #implementation_call
            }
        }

        fn #invoke<'call, T>(
            validated: #facade::__private::codegen_v4::invoke::ValidatedInvocation<'call, #value_mode>,
        ) -> #facade::__private::codegen_v4::invoke::InvocationOutput<'call, #value_mode>
        where
            T: #call_trait + 'static,
            #receiver_bound
            #(#parameter_bounds)*
            #return_bound
        {
            #invoke_receiver
            let mut arguments = arguments.into_vec().into_iter();
            #(#typed_arguments)*
            let result = <T as #call_trait>::call(#invocation_receiver #(#adapter_arguments),*);
            #result
        }

        struct #probe<T>(::core::marker::PhantomData<fn() -> T>);

        impl<T> #probe<T>
        where
            T: #call_trait + 'static,
            #receiver_bound
            #(#parameter_bounds)*
            #return_bound
        {
            const ADAPTER: #facade::__private::codegen_v4::invoke::FunctionInvocationAdapter<#value_mode> =
                #facade::__private::codegen_v4::invoke::FunctionInvocationAdapter::<#value_mode>::new(#invoke::<T>);
        }

        trait #selector_trait {
            fn select_thread_safe(self) -> Option<&'static dyn #facade::__private::codegen_v4::invoke::InvocationAdapter<#value_mode>>;
        }

        impl<T> #selector_trait for &&#probe<T>
        where
            T: #call_trait + 'static,
            #receiver_bound
            #(#parameter_bounds)*
            #return_bound
        {
            fn select_thread_safe(self) -> Option<&'static dyn #facade::__private::codegen_v4::invoke::InvocationAdapter<#value_mode>> {
                Some(&#probe::<T>::ADAPTER)
            }
        }

        impl<T> #selector_trait for &#probe<T> {
            fn select_thread_safe(self) -> Option<&'static dyn #facade::__private::codegen_v4::invoke::InvocationAdapter<#value_mode>> {
                None
            }
        }
    };
    Some(ThreadSafeDefinition { tokens, selector })
}

fn contains_self_type(ty: &TypeIr) -> bool {
    ty.tokens
        .to_string()
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .any(|identifier| identifier == "Self")
}

/// Generates one executable invocation adapter for a concrete impl method.
///
/// # Parameters
///
/// - `method`: Validated implementation method declaration.
/// - `index`: Method's declaration index.
/// - `target`: Concrete impl target type tokens.
/// - `trait_call_path`: Optional trait path used for qualified calls.
/// - `facade`: Runtime facade path used in generated references.
///
/// # Returns
///
/// Returns generated adapter tokens when the method is executable, or `None`.
#[must_use]
pub(super) fn definition(
    method: &MethodIr,
    index: usize,
    target: &TokenStream,
    trait_call_path: &Option<TokenStream>,
    _target_source: &str,
    facade: &TokenStream,
) -> Option<TokenStream> {
    definition_for_call(
        method,
        index,
        target,
        trait_call_path,
        _target_source,
        facade,
        &[],
        None,
    )
}

/// Returns the static descriptor name shared by specialization emission and
/// registration.
///
/// # Parameters
///
/// - `method_index`: Method's declaration index.
/// - `specialization_index`: Specialization's index for that method.
///
/// # Returns
///
/// Returns the stable static identifier used by both code generation phases.
#[must_use]
pub(super) fn specialization_descriptor_name(method_index: usize, specialization_index: usize) -> Ident {
    format_ident!("__QUBIT_REFLECT_GENERIC_SPECIALIZATION_ADAPTER_{method_index}_{specialization_index}")
}

/// Builds one mode-specific adapter after analyzing its invocation contract.
///
/// # Parameters
///
/// - `method`: Validated implementation method declaration.
/// - `index`: Method's declaration index.
/// - `target`: Concrete impl target type tokens.
/// - `trait_call_path`: Optional trait path used for qualified calls.
/// - `facade`: Runtime facade path used in generated references.
/// - `generic_arguments`: Concrete generic arguments for a specialization.
/// - `specialization_index`: Optional specialization index.
///
/// # Returns
///
/// Returns generated adapter tokens when executable, or `None` when blocked.
#[allow(clippy::too_many_arguments)]
#[must_use]
fn definition_for_call(
    method: &MethodIr,
    index: usize,
    target: &TokenStream,
    trait_call_path: &Option<TokenStream>,
    _target_source: &str,
    facade: &TokenStream,
    generic_arguments: &[TokenStream],
    specialization_index: Option<usize>,
) -> Option<TokenStream> {
    let adapter_suffix = specialization_index.map_or_else(|| index.to_string(), |value| format!("{index}_{value}"));
    let descriptor_name = specialization_index.map_or_else(
        || format_ident!("__QUBIT_REFLECT_INVOCATION_ADAPTER_{index}"),
        |value| specialization_descriptor_name(index, value),
    );
    let generic_call = (!generic_arguments.is_empty()).then(|| quote!(::<#(#generic_arguments),*>));
    let typed_extension_receiver = method
        .receiver
        .as_ref()
        .and_then(|receiver| typed_extension_receiver_type(receiver, target));
    let mut method_context = crate::expand::invocation::analysis::MethodContext::implementation(target);
    method_context.extension_receiver = typed_extension_receiver.clone();
    let invocation_plan = crate::expand::invocation::analysis::analyze_method(method, method_context)
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
            let mode = if invocation_plan.modes.thread_safe {
                quote!(#facade::__private::codegen_v4::value::ThreadSafe)
            } else {
                quote!(#facade::__private::codegen_v4::value::Local)
            };
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
            let receiver = if pinned_mutable {
                quote!(pinned_receiver.downcast_mut::<#target>()
                    .unwrap_or_else(|| unreachable!("validation checked pinned receiver type")))
            } else {
                quote!(pinned_receiver.downcast_ref::<#target>()
                    .unwrap_or_else(|| unreachable!("validation checked pinned receiver type")))
            };
            let call = if let Some(trait_path) = &trait_call_path {
                quote!(<#target as #trait_path>::#method_name #generic_call (receiver, #(#call_arguments),*))
            } else {
                quote!(<#target>::#method_name #generic_call (receiver, #(#call_arguments),*))
            };
            let call = if method.qualifiers.is_unsafe {
                quote!(unsafe { #call })
            } else {
                call
            };
            let output = match &method.return_type {
                ReturnTypeIr::Unit => {
                    quote! {{ #call; #facade::__private::codegen_v4::invoke::InvocationOutput::unit() }}
                }
                ReturnTypeIr::Type(TypeIr {
                    kind: TypeKindIr::Never,
                    ..
                }) => {
                    quote!(match #call {})
                }
                ReturnTypeIr::Type(TypeIr {
                    kind: TypeKindIr::Reference { mutable: false, .. },
                    ..
                }) => quote!(
                    #facade::__private::codegen_v4::invoke::InvocationOutput::shared(
                        #call,
                        #facade::__private::codegen_v4::invoke::BorrowOrigin::Receiver,
                    )
                ),
                ReturnTypeIr::Type(TypeIr {
                    kind: TypeKindIr::Reference { mutable: true, .. },
                    ..
                }) => quote!(
                    #facade::__private::codegen_v4::invoke::InvocationOutput::mutable(
                        #call,
                        #facade::__private::codegen_v4::invoke::BorrowOrigin::Receiver,
                    )
                ),
                ReturnTypeIr::Type(_) => quote! {
                    #facade::__private::codegen_v4::invoke::InvocationOutput::owned(#call)
                },
            };
            let constructor = if pinned_mutable {
                quote!(#facade::__private::codegen_v4::invoke::PinnedFunctionInvocationAdapter::<#mode, #target>::mutable(#adapter_name))
            } else {
                quote!(#facade::__private::codegen_v4::invoke::PinnedFunctionInvocationAdapter::<#mode, #target>::shared(#adapter_name))
            };
            return Some(quote! {
                fn #adapter_name<'call>(
                    pinned_receiver: #facade::__private::codegen_v4::invoke::PinnedReceiver<'call>,
                    validated: #facade::__private::codegen_v4::invoke::ValidatedInvocation<'call, #mode>,
                ) -> #facade::__private::codegen_v4::invoke::InvocationOutput<'call, #mode> {
                    let receiver = #receiver;
                    let (_receiver, arguments) = validated.into_parts();
                    let mut arguments = arguments.into_vec().into_iter();
                    #(#argument_bindings)*
                    #output
                }

                static #descriptor_name: #facade::__private::codegen_v4::invoke::PinnedFunctionInvocationAdapter<#mode, #target> = #constructor;
            });
        }
    }
    if pinned_receiver_mutable.is_some() {
        return None;
    }
    if !is_safe_invocation {
        return None;
    }
    if typed_extension_receiver.is_some() {
        return None;
    }
    let method_name = &method.name;
    let adapter_name = format_ident!("__qubit_reflect_invoke_{adapter_suffix}");
    let thread_safe = invocation_plan.modes.thread_safe;
    let mode = if thread_safe {
        quote!(#facade::__private::codegen_v4::value::ThreadSafe)
    } else {
        quote!(#facade::__private::codegen_v4::value::Local)
    };
    let thread_safe_assertions = thread_safe.then(|| {
        crate::expand::invocation::emit::thread_safe_assertions(
            method,
            target,
            typed_owned_receiver.as_ref(),
            typed_extension_receiver.as_ref(),
        )
    });
    let adapter_definition = quote! {
        static #descriptor_name: #facade::__private::codegen_v4::invoke::FunctionInvocationAdapter<#mode> =
            #facade::__private::codegen_v4::invoke::FunctionInvocationAdapter::<#mode>::new(#adapter_name);
    };
    let receiver_binding = if matches!(
        method.receiver.as_ref().map(|receiver| receiver.kind),
        Some(ReceiverKindIr::Value)
    ) {
        quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver: #target = receiver
                .expect("validated invocation has a receiver")
                .into_owned::<#target>()
                .unwrap_or_else(|_| unreachable!("validation checked receiver type and mode"));
        }
    } else if let Some(receiver_type) = &typed_owned_receiver {
        quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver: #receiver_type = receiver
                .expect("validated invocation has a receiver")
                .into_owned::<#receiver_type>()
                .unwrap_or_else(|_| unreachable!("validation checked receiver type and mode"));
        }
    } else if matches!(
        method.receiver.as_ref().map(|receiver| receiver.kind),
        Some(ReceiverKindIr::MutableReference)
    ) {
        quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver: &mut #target = receiver
                .expect("validated invocation has a receiver")
                .into_mut::<#target>()
                .unwrap_or_else(|_| unreachable!("validation checked receiver type and mode"));
        }
    } else if method.receiver.is_some() {
        quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver = receiver.expect("validated invocation has a receiver");
            let receiver: &#target = match receiver.into_ref::<#target>() {
                Ok(value) => value,
                Err(receiver) => receiver
                    .into_mut::<#target>()
                    .map(|value| &*value)
                    .unwrap_or_else(|_| unreachable!("validation checked receiver type and mode")),
            };
        }
    } else {
        quote! { let (_receiver, arguments) = validated.into_parts(); }
    };
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
    let call = if method.qualifiers.is_unsafe {
        quote!(unsafe { #call })
    } else {
        call
    };
    let borrow_origins: Vec<_> = std::iter::once(
        method
            .receiver
            .is_some()
            .then(|| quote!(#facade::__private::codegen_v4::invoke::BorrowOrigin::Receiver)),
    )
    .flatten()
    .chain(
        method
            .parameters
            .iter()
            .filter(|parameter| matches!(parameter.ty.kind, TypeKindIr::Reference { .. }))
            .map(|parameter| {
                let index = parameter.index;
                quote!(#facade::__private::codegen_v4::invoke::BorrowOrigin::Parameter(#index))
            }),
    )
    .collect();
    let optional_borrow_origin =
        crate::expand::invocation::analysis::optional_shared_borrow_origin(method).map(|origin| match origin {
            crate::expand::invocation::analysis::OptionalBorrowOrigin::Receiver => {
                quote!(#facade::__private::codegen_v4::invoke::BorrowOrigin::Receiver)
            }
            crate::expand::invocation::analysis::OptionalBorrowOrigin::Parameter(index) => {
                quote!(#facade::__private::codegen_v4::invoke::BorrowOrigin::Parameter(#index))
            }
        });
    let output = match (method.qualifiers.is_async, &method.return_type) {
        (false, ReturnTypeIr::Unit) => quote! {
            #receiver_binding
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            #call;
            #facade::__private::codegen_v4::invoke::InvocationOutput::unit()
        },
        (
            false,
            ReturnTypeIr::Type(TypeIr {
                kind: TypeKindIr::Reference { mutable: false, .. },
                ..
            }),
        ) => {
            quote! {
                #receiver_binding
                let mut arguments = arguments.into_vec().into_iter();
                #(#argument_bindings)*
                #facade::__private::codegen_v4::invoke::InvocationOutput::shared_with_origins(
                    #call,
                    &[#(#borrow_origins),*],
                )
            }
        }
        (false, ReturnTypeIr::Type(_)) if optional_borrow_origin.is_some() => {
            let origin = optional_borrow_origin
                .as_ref()
                .expect("the guarded output has a borrow origin");
            quote! {
                #receiver_binding
                let mut arguments = arguments.into_vec().into_iter();
                #(#argument_bindings)*
                #facade::__private::codegen_v4::invoke::InvocationOutput::optional_shared_with_origins(
                    #call,
                    &[#origin],
                )
            }
        }
        (
            false,
            ReturnTypeIr::Type(TypeIr {
                kind: TypeKindIr::Reference { mutable: true, .. },
                ..
            }),
        ) => {
            quote! {
                #receiver_binding
                let mut arguments = arguments.into_vec().into_iter();
                #(#argument_bindings)*
                #facade::__private::codegen_v4::invoke::InvocationOutput::mutable(
                    #call,
                    #facade::__private::codegen_v4::invoke::BorrowOrigin::Receiver,
                )
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
            #facade::__private::codegen_v4::invoke::InvocationOutput::owned(#call)
        },
        (true, ReturnTypeIr::Unit) => quote! {
            #receiver_binding
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            #facade::__private::codegen_v4::invoke::InvocationOutput::future(async move {
                    #call.await;
                    #facade::__private::codegen_v4::invoke::InvocationOutput::unit()
                })
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
            #facade::__private::codegen_v4::invoke::InvocationOutput::future(async move {
                    match #call.await {}
                })
        },
        (true, ReturnTypeIr::Type(_)) => quote! {
            #receiver_binding
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            #facade::__private::codegen_v4::invoke::InvocationOutput::future(async move {
                    #facade::__private::codegen_v4::invoke::InvocationOutput::owned(#call.await)
                })
        },
    };
    let invocation_result = quote!({ #output });
    Some(quote! {
        fn #adapter_name<'call>(
            validated: #facade::__private::codegen_v4::invoke::ValidatedInvocation<'call, #mode>,
        ) -> #facade::__private::codegen_v4::invoke::InvocationOutput<'call, #mode> {
            #thread_safe_assertions
            #invocation_result
        }

        #adapter_definition
    })
}
