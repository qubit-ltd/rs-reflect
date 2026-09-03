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
use crate::ir::TypeIr;
use crate::ir::TypeKindIr;

/// Emits the canonical runtime unavailability slice for one adapter plan.
pub(crate) fn emit_unavailable_reasons(
    plan: &InvocationPlan,
    context: &ExpansionContext,
) -> TokenStream {
    debug_assert!(
        !plan.is_executable()
            || !matches!(plan.output, OutputPlan::Opaque | OutputPlan::Unsupported)
    );
    let facade = context.facade();
    let reasons = match &plan.availability {
        AvailabilityPlan::Executable => return quote!(&[]),
        AvailabilityPlan::DescribedOnly(reasons) => reasons,
    };
    let reasons = reasons.iter().map(|reason| match reason {
        UnavailableReasonPlan::UnsupportedReceiver => {
            quote!(#facade::__private::codegen_v2::descriptor::InvocationUnavailableReason::UnsupportedReceiver)
        }
        UnavailableReasonPlan::UnspecializedGeneric => {
            quote!(#facade::__private::codegen_v2::descriptor::InvocationUnavailableReason::UnspecializedGeneric)
        }
        UnavailableReasonPlan::UnsafeMethod => quote!(#facade::__private::codegen_v2::descriptor::InvocationUnavailableReason::UnsafeMethod),
        UnavailableReasonPlan::UnsupportedAbi => {
            quote!(#facade::__private::codegen_v2::descriptor::InvocationUnavailableReason::UnsupportedAbi)
        }
        UnavailableReasonPlan::Variadic => quote!(#facade::__private::codegen_v2::descriptor::InvocationUnavailableReason::Variadic),
        UnavailableReasonPlan::UnsupportedBorrowedReturn => {
            quote!(#facade::__private::codegen_v2::descriptor::InvocationUnavailableReason::UnsupportedBorrowedReturn)
        }
        UnavailableReasonPlan::OpaqueReturn => quote!(#facade::__private::codegen_v2::descriptor::InvocationUnavailableReason::OpaqueReturn),
        UnavailableReasonPlan::UnsupportedUnsizedValue => {
            quote!(#facade::__private::codegen_v2::descriptor::InvocationUnavailableReason::UnsupportedUnsizedValue)
        }
        UnavailableReasonPlan::UnprovenDefaultConstraint => {
            quote!(#facade::__private::codegen_v2::descriptor::InvocationUnavailableReason::UnprovenDefaultConstraint)
        }
        UnavailableReasonPlan::UnprovenAssociatedType => {
            quote!(#facade::__private::codegen_v2::descriptor::InvocationUnavailableReason::UnprovenAssociatedType)
        }
        UnavailableReasonPlan::PinnedModeConflict => {
            quote!(#facade::__private::codegen_v2::descriptor::InvocationUnavailableReason::PinnedModeConflict)
        }
        UnavailableReasonPlan::DisabledByPolicy => {
            quote!(#facade::__private::codegen_v2::descriptor::InvocationUnavailableReason::DisabledByPolicy)
        }
    });
    quote!(&[#(#reasons),*])
}

/// Emits method-local bounds required by a thread-safe adapter.
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
                mutable: true,
                element,
                ..
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
        ReturnTypeIr::Type(ty)
            if !matches!(ty.kind, TypeKindIr::Reference { .. } | TypeKindIr::Never) =>
        {
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

/// Emits method-local bounds required by a panic-catching adapter.
pub(crate) fn catching_assertions(
    method: &MethodIr,
    target: &TokenStream,
    typed_owned_receiver: Option<&TokenStream>,
    typed_extension_receiver: Option<&TokenStream>,
) -> TokenStream {
    let receiver = method.receiver.as_ref().map(|receiver| {
        let span = receiver.span;
        if let Some(ty) = typed_extension_receiver.or(typed_owned_receiver) {
            quote_spanned!(span=> __qubit_reflect_assert_unwind_safe::<#ty>();)
        } else {
            match receiver.kind {
                ReceiverKindIr::SharedReference => {
                    quote_spanned!(span=> __qubit_reflect_assert_ref_unwind_safe::<#target>();)
                }
                ReceiverKindIr::MutableReference => {
                    quote_spanned!(span=> __qubit_reflect_assert_unwind_safe::<&mut #target>();)
                }
                ReceiverKindIr::Value => {
                    quote_spanned!(span=> __qubit_reflect_assert_unwind_safe::<#target>();)
                }
                ReceiverKindIr::Typed => TokenStream::new(),
            }
        }
    });
    let parameters = method.parameters.iter().map(|parameter| {
        let span = parameter.span;
        match &parameter.ty.kind {
            TypeKindIr::Reference {
                mutable: true,
                element,
                ..
            } => {
                let ty = &element.tokens;
                quote_spanned!(span=> __qubit_reflect_assert_unwind_safe::<&mut #ty>();)
            }
            TypeKindIr::Reference { element, .. } => {
                let ty = &element.tokens;
                quote_spanned!(span=> __qubit_reflect_assert_ref_unwind_safe::<#ty>();)
            }
            _ => {
                let ty = &parameter.ty.tokens;
                quote_spanned!(span=> __qubit_reflect_assert_unwind_safe::<#ty>();)
            }
        }
    });
    let output = match &method.return_type {
        ReturnTypeIr::Type(TypeIr {
            kind:
                TypeKindIr::Reference {
                    mutable: false,
                    element,
                    ..
                },
            ..
        }) => {
            let ty = &element.tokens;
            let span = element.span;
            quote_spanned!(span=> __qubit_reflect_assert_ref_unwind_safe::<#ty>();)
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
            let ty = &element.tokens;
            let span = element.span;
            quote_spanned!(span=> __qubit_reflect_assert_unwind_safe::<&mut #ty>();)
        }
        ReturnTypeIr::Type(ty) if !matches!(ty.kind, TypeKindIr::Never) => {
            let tokens = &ty.tokens;
            let span = ty.span;
            quote_spanned!(span=> __qubit_reflect_assert_unwind_safe::<#tokens>();)
        }
        _ => TokenStream::new(),
    };
    quote! {
        fn __qubit_reflect_assert_unwind_safe<T: ?Sized + ::std::panic::UnwindSafe>() {}
        fn __qubit_reflect_assert_ref_unwind_safe<T: ?Sized + ::std::panic::RefUnwindSafe>() {}
        #receiver
        #(#parameters)*
        #output
    }
}

/// Emits the runtime expectation for one positional parameter.
pub(crate) fn argument_expectation(parameter: &ParameterIr, facade: &TokenStream) -> TokenStream {
    match &parameter.ty.kind {
        TypeKindIr::Reference {
            mutable: true,
            element,
            ..
        } => {
            let element = &element.tokens;
            quote!(#facade::__private::codegen_v2::invoke::ArgumentExpectation::borrowed_mut::<#element>())
        }
        TypeKindIr::Reference { element, .. } => {
            let element = &element.tokens;
            quote!(#facade::__private::codegen_v2::invoke::ArgumentExpectation::borrowed::<#element>())
        }
        _ => {
            let ty = &parameter.ty.tokens;
            quote!(#facade::__private::codegen_v2::invoke::ArgumentExpectation::owned::<#ty>())
        }
    }
}

/// Emits extraction of one already validated positional argument.
pub(crate) fn argument_binding(
    parameter: &ParameterIr,
    facade: &TokenStream,
    mode: &TokenStream,
) -> TokenStream {
    let argument = format_ident!("__qubit_reflect_argument_{}", parameter.index);
    match &parameter.ty.kind {
        TypeKindIr::Reference {
            mutable: true,
            element,
            ..
        } => {
            if super::analysis::is_str_type(element) {
                quote! {
                    let #argument = match arguments.next().expect("validation checked argument count") {
                        #facade::__private::codegen_v2::invoke::InvocationArg::Mut(value) =>
                            #facade::__private::codegen_v2::value::DynamicMut::<#mode>::into_str_mut(value)
                                .unwrap_or_else(|_| unreachable!("validation checked argument type")),
                        _ => unreachable!("validation checked argument mode"),
                    };
                }
            } else {
                let element = &element.tokens;
                quote! {
                    let #argument = match arguments.next().expect("validation checked argument count") {
                        #facade::__private::codegen_v2::invoke::InvocationArg::Mut(value) =>
                            #facade::__private::codegen_v2::value::DynamicMut::<#mode>::downcast::<#element>(value)
                                .unwrap_or_else(|_| unreachable!("validation checked argument type")),
                        _ => unreachable!("validation checked argument mode"),
                    };
                }
            }
        }
        TypeKindIr::Reference { element, .. } => {
            if super::analysis::is_str_type(element) {
                quote! {
                    let #argument = match arguments.next().expect("validation checked argument count") {
                        #facade::__private::codegen_v2::invoke::InvocationArg::Ref(value) =>
                            #facade::__private::codegen_v2::value::DynamicRef::<#mode>::into_str(value)
                                .unwrap_or_else(|_| unreachable!("validation checked argument type")),
                        #facade::__private::codegen_v2::invoke::InvocationArg::Mut(value) => {
                            let value = #facade::__private::codegen_v2::value::DynamicMut::<#mode>::into_str_mut(value)
                                .unwrap_or_else(|_| unreachable!("validation checked argument type"));
                            &*value
                        }
                        _ => unreachable!("validation checked argument mode"),
                    };
                }
            } else {
                let element = &element.tokens;
                quote! {
                    let #argument = match arguments.next().expect("validation checked argument count") {
                        #facade::__private::codegen_v2::invoke::InvocationArg::Ref(value) =>
                            #facade::__private::codegen_v2::value::DynamicRef::<#mode>::downcast::<#element>(value)
                                .unwrap_or_else(|_| unreachable!("validation checked argument type")),
                        #facade::__private::codegen_v2::invoke::InvocationArg::Mut(value) => {
                            let value = #facade::__private::codegen_v2::value::DynamicMut::<#mode>::downcast::<#element>(value)
                                .unwrap_or_else(|_| unreachable!("validation checked argument type"));
                            &*value
                        }
                        _ => unreachable!("validation checked argument mode"),
                    };
                }
            }
        }
        _ => {
            let ty = &parameter.ty.tokens;
            quote! {
                let #argument: #ty = match arguments.next().expect("validation checked argument count") {
                    #facade::__private::codegen_v2::invoke::InvocationArg::Owned(value) =>
                        #facade::__private::codegen_v2::value::DynamicOwned::<#mode>::downcast::<#ty>(value)
                            .unwrap_or_else(|_| unreachable!("validation checked argument type")),
                    _ => unreachable!("validation checked argument mode"),
                };
            }
        }
    }
}
