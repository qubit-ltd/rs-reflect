// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shared token-emission primitives driven by validated invocation plans.
//!
//! Concrete impl adapters and trait-default hooks have different ownership,
//! receiver, and registration shells. Their owners retain those shells while
//! this module emits the semantic fragments that must remain identical.

use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use quote::quote_spanned;

use super::plan::AvailabilityPlan;
use super::plan::InvocationPlan;
use super::plan::OutputPlan;
use super::plan::UnavailableReasonPlan;
use crate::expand::ExpansionContext;
use crate::ir::MethodIr;
use crate::ir::ParameterIr;
use crate::ir::ReceiverKindIr;
use crate::ir::ReturnTypeIr;
use crate::ir::TypeKindIr;

/// Emits the canonical runtime unavailability slice for one adapter plan.
///
/// # Parameters
///
/// - `plan`: Validated invocation availability plan.
/// - `context`: Expansion context containing the runtime facade.
///
/// # Returns
///
/// Returns generated tokens for the ordered reasons, or an empty slice for
/// executable plans.
#[must_use]
pub(crate) fn emit_unavailable_reasons(plan: &InvocationPlan, context: &ExpansionContext) -> TokenStream {
    debug_assert!(!plan.is_executable() || !matches!(plan.output, OutputPlan::Opaque | OutputPlan::Unsupported));
    let facade = context.facade();
    let reasons = match &plan.availability {
        AvailabilityPlan::Executable => return quote!(&[]),
        AvailabilityPlan::DescribedOnly(reasons) => reasons,
    };
    let reasons = reasons.iter().map(|reason| match reason {
        UnavailableReasonPlan::UnsupportedReceiver => {
            quote!(#facade::__private::codegen_v4::invoke::InvocationUnavailableReason::UnsupportedReceiver)
        }
        UnavailableReasonPlan::UnspecializedGeneric => {
            quote!(#facade::__private::codegen_v4::invoke::InvocationUnavailableReason::UnspecializedGeneric)
        }
        UnavailableReasonPlan::UnsafeMethod => {
            quote!(#facade::__private::codegen_v4::invoke::InvocationUnavailableReason::UnsafeMethod)
        }
        UnavailableReasonPlan::UnsupportedAbi => {
            quote!(#facade::__private::codegen_v4::invoke::InvocationUnavailableReason::UnsupportedAbi)
        }
        UnavailableReasonPlan::Variadic => {
            quote!(#facade::__private::codegen_v4::invoke::InvocationUnavailableReason::Variadic)
        }
        UnavailableReasonPlan::UnsupportedBorrowedReturn => {
            quote!(#facade::__private::codegen_v4::invoke::InvocationUnavailableReason::UnsupportedBorrowedReturn)
        }
        UnavailableReasonPlan::OpaqueReturn => {
            quote!(#facade::__private::codegen_v4::invoke::InvocationUnavailableReason::OpaqueReturn)
        }
        UnavailableReasonPlan::UnsupportedUnsizedValue => {
            quote!(#facade::__private::codegen_v4::invoke::InvocationUnavailableReason::UnsupportedUnsizedValue)
        }
        UnavailableReasonPlan::UnprovenDefaultConstraint => {
            quote!(#facade::__private::codegen_v4::invoke::InvocationUnavailableReason::UnprovenTraitConstraint)
        }
        UnavailableReasonPlan::UnprovenAssociatedType => {
            quote!(#facade::__private::codegen_v4::invoke::InvocationUnavailableReason::UnprovenTraitConstraint)
        }
        UnavailableReasonPlan::PinnedModeConflict => {
            quote!(#facade::__private::codegen_v4::invoke::InvocationUnavailableReason::UnsupportedReceiver)
        }
        UnavailableReasonPlan::DisabledByPolicy => {
            quote!(#facade::__private::codegen_v4::invoke::InvocationUnavailableReason::DisabledByPolicy)
        }
    });
    quote!(::std::boxed::Box::leak(::std::boxed::Box::new([#(#reasons),*])))
}

/// Emits method-local bounds required by a thread-safe adapter.
///
/// # Parameters
///
/// - `method`: Method whose receiver, arguments and result are checked.
/// - `target`: Concrete impl target type tokens.
/// - `typed_owned_receiver`: Optional recognized owned receiver type.
/// - `typed_extension_receiver`: Optional substituted extension receiver type.
///
/// # Returns
///
/// Returns generated compile-time trait assertions at the original item spans.
#[must_use]
pub(crate) fn thread_safe_assertions(
    method: &MethodIr,
    target: &TokenStream,
    typed_owned_receiver: Option<&TokenStream>,
    typed_extension_receiver: Option<&TokenStream>,
) -> TokenStream {
    let receiver = method.receiver.as_ref().map(|receiver| {
        let span = receiver.span;
        if let Some(ty) = typed_extension_receiver.or(typed_owned_receiver) {
            quote_spanned!(span=> __qubit_reflect_assert_send_sync::<#ty>();)
        } else {
            match receiver.kind {
                ReceiverKindIr::SharedReference => {
                    quote_spanned!(span=> __qubit_reflect_assert_sync::<#target>();)
                }
                ReceiverKindIr::Value | ReceiverKindIr::MutableReference => {
                    quote_spanned!(span=> __qubit_reflect_assert_send_sync::<#target>();)
                }
                ReceiverKindIr::Typed => TokenStream::new(),
            }
        }
    });
    let parameters = method.parameters.iter().map(|parameter| {
        let span = parameter.span;
        match &parameter.ty.kind {
            TypeKindIr::Reference {
                mutable: true, element, ..
            } => {
                let ty = &element.tokens;
                quote_spanned!(span=> __qubit_reflect_assert_send_sync::<#ty>();)
            }
            TypeKindIr::Reference { element, .. } => {
                let ty = &element.tokens;
                quote_spanned!(span=> __qubit_reflect_assert_sync::<#ty>();)
            }
            _ => {
                let ty = &parameter.ty.tokens;
                quote_spanned!(span=> __qubit_reflect_assert_send_sync::<#ty>();)
            }
        }
    });
    let output = match &method.return_type {
        ReturnTypeIr::Type(ty) if !matches!(ty.kind, TypeKindIr::Reference { .. } | TypeKindIr::Never) => {
            let tokens = &ty.tokens;
            let span = ty.span;
            quote_spanned!(span=> __qubit_reflect_assert_send_sync::<#tokens>();)
        }
        _ => TokenStream::new(),
    };
    quote! {
        fn __qubit_reflect_assert_sync<T: ?Sized + ::core::marker::Sync>() {}
        fn __qubit_reflect_assert_send_sync<
            T: ?Sized + ::core::marker::Send + ::core::marker::Sync,
        >() {}
        #receiver
        #(#parameters)*
        #output
    }
}

/// Emits extraction of one already validated positional argument.
///
/// # Parameters
///
/// - `parameter`: Validated positional parameter declaration.
/// - `facade`: Runtime facade path used in generated references.
/// - `mode`: Dynamic value mode used by the generated adapter.
///
/// # Returns
///
/// Returns tokens binding the argument to its concrete Rust type.
///
/// # Panics
///
/// The generated code is unreachable if validation did not verify count, type
/// and mode.
#[must_use]
pub(crate) fn argument_binding(parameter: &ParameterIr, _facade: &TokenStream, _mode: &TokenStream) -> TokenStream {
    let argument = format_ident!("__qubit_reflect_argument_{}", parameter.index);
    match &parameter.ty.kind {
        TypeKindIr::Reference {
            mutable: true, element, ..
        } => {
            let element = &element.tokens;
            quote! {
                let #argument = arguments.next()
                    .expect("validated invocation contains each declared argument")
                    .into_mut::<#element>()
                    .unwrap_or_else(|_| unreachable!("validated invocation checked argument type and mode"));
            }
        }
        TypeKindIr::Reference { element, .. } => {
            let element = &element.tokens;
            quote! {
                let #argument = arguments.next()
                    .expect("validated invocation contains each declared argument");
                let #argument = match #argument.into_ref::<#element>() {
                    Ok(value) => value,
                    Err(value) => match value.into_mut::<#element>() {
                        Ok(value) => &*value,
                        Err(_) => unreachable!("validated invocation checked argument type and mode"),
                    },
                };
            }
        }
        _ => {
            let ty = &parameter.ty.tokens;
            quote! {
                let #argument: #ty = arguments.next()
                    .expect("validated invocation contains each declared argument")
                    .into_owned::<#ty>()
                    .unwrap_or_else(|_| unreachable!("validated invocation checked argument type and mode"));
            }
        }
    }
}
