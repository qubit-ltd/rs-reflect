// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Generic adapters for trait methods that provide a default body.

use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::GenericParam;
use syn::ItemTrait;
use syn::LitStr;

use crate::expand::ExpansionContext;
use crate::ir::MethodIr;
use crate::ir::ReceiverKindIr;
use crate::ir::ReturnTypeIr;
use crate::ir::TraitDeclarationIr;
use crate::ir::TypeKindIr;

/// Emits a target-generic provider for the executable default methods of a trait.
///
/// The provider is generated beside the trait declaration, where its method
/// identifiers are available. Impl derives call it with the concrete target
/// and the names overridden by that impl.
#[must_use]
pub(super) fn default_methods_provider(
    declaration: &TraitDeclarationIr,
    retained: &ItemTrait,
    trait_name: &Ident,
    spec_provider: &Ident,
    provider: &Ident,
    facade: &TokenStream,
    context: &ExpansionContext,
) -> TokenStream {
    let codegen = quote!(#facade::__private::codegen_v4);
    let (_, generic_arguments) = trait_generic_parts(&retained.generics.params);
    let trait_arguments = if generic_arguments.is_empty() {
        quote!()
    } else {
        quote!(<#(#generic_arguments),*>)
    };
    let trait_path = quote!(#trait_name #trait_arguments);
    let mut helper_generic_parameters = ordered_generic_parameters(&retained.generics.params);
    let lifetime_count = retained
        .generics
        .params
        .iter()
        .take_while(|parameter| matches!(parameter, GenericParam::Lifetime(_)))
        .count();
    helper_generic_parameters.insert(
        lifetime_count,
        quote!(__QubitReflectTarget: #codegen::descriptor::Reflect + 'static),
    );
    let helper_generics = quote!(<#(#helper_generic_parameters),*>);
    let helper_generic_arguments = generic_arguments[..lifetime_count]
        .iter()
        .cloned()
        .chain(std::iter::once(quote!(__QubitReflectTarget)))
        .chain(generic_arguments[lifetime_count..].iter().cloned())
        .collect::<Vec<_>>();
    let target_generic_arguments = quote!(::<#(#helper_generic_arguments),*>);
    let methods = declaration
        .methods
        .iter()
        .enumerate()
        .filter(|(_, method)| method.has_default)
        .filter_map(|(index, method)| {
            emit_default_method(
                method,
                index,
                &trait_path,
                &helper_generic_parameters,
                &target_generic_arguments,
                spec_provider,
                &codegen,
                facade,
                context,
            )
        })
        .collect::<Vec<_>>();
    quote! {
        #[doc(hidden)]
        pub fn #provider #helper_generics (
            overridden: &[&'static str],
        ) -> ::std::boxed::Box<[#codegen::invoke::EffectiveMethod]>
        where
            __QubitReflectTarget: #trait_path,
        {
            let mut methods = ::std::vec::Vec::new();
            #(#methods)*
            methods.into_boxed_slice()
        }
    }
}

fn emit_default_method(
    method: &MethodIr,
    index: usize,
    trait_path: &TokenStream,
    helper_generic_parameters: &[TokenStream],
    target_generic_arguments: &TokenStream,
    spec_provider: &Ident,
    codegen: &TokenStream,
    facade: &TokenStream,
    context: &ExpansionContext,
) -> Option<TokenStream> {
    let target_type = quote!(__QubitReflectTarget);
    let method_context =
        crate::expand::invocation::analysis::MethodContext::implementation(&target_type);
    let plan = crate::expand::invocation::analysis::analyze_method(method, method_context)
        .expect("validated trait method analysis is infallible");
    if !plan.is_executable() || !method.generics.params.is_empty() {
        let reasons = crate::expand::invocation::emit::emit_unavailable_reasons(&plan, context);
        let availability = quote!(.with_unavailable(#reasons));
        let overridden_name = LitStr::new(&method.name.to_string(), method.span);
        return Some(quote! {
            if !overridden.iter().any(|name| *name == #overridden_name) {
                methods.push(
                    #codegen::invoke::EffectiveMethod::for_type::<__QubitReflectTarget>(
                        #spec_provider().method_at(#index),
                    )
                    #availability
                    .with_source(#codegen::invoke::MethodImplementationSource::Defaulted),
                );
            }
        });
    }
    let adapter = format_ident!("__qubit_reflect_default_adapter_{index}");
    let method_name = &method.name;
    let mode = quote!(#codegen::value::Local);
    let arguments = method
        .parameters
        .iter()
        .map(|parameter| {
            crate::expand::invocation::emit::argument_binding(parameter, facade, &mode)
        })
        .collect::<Vec<_>>();
    let call_arguments = method
        .parameters
        .iter()
        .map(|parameter| format_ident!("__qubit_reflect_argument_{}", parameter.index))
        .collect::<Vec<_>>();
    let pinned_receiver_mutable = method
        .receiver
        .as_ref()
        .and_then(crate::expand::invocation::analysis::typed_pinned_receiver_mutable);
    let receiver = match method.receiver.as_ref().map(|receiver| receiver.kind) {
        Some(ReceiverKindIr::SharedReference) => quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver = receiver.expect("validated invocation has a receiver")
                .into_ref::<__QubitReflectTarget>()
                .unwrap_or_else(|_| unreachable!("validated invocation checked receiver type"));
        },
        Some(ReceiverKindIr::MutableReference) => quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver = receiver.expect("validated invocation has a receiver")
                .into_mut::<__QubitReflectTarget>()
                .unwrap_or_else(|_| unreachable!("validated invocation checked receiver type"));
        },
        Some(ReceiverKindIr::Value) => quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver = receiver.expect("validated invocation has a receiver")
                .into_owned::<__QubitReflectTarget>()
                .unwrap_or_else(|_| unreachable!("validated invocation checked receiver type"));
        },
        Some(ReceiverKindIr::Typed) if pinned_receiver_mutable == Some(true) => quote! {
            let (_receiver, arguments) = validated.into_parts();
            let receiver = pinned_receiver.downcast_mut::<__QubitReflectTarget>()
                .unwrap_or_else(|| unreachable!("validated invocation checked pinned receiver type"));
        },
        Some(ReceiverKindIr::Typed) if pinned_receiver_mutable == Some(false) => quote! {
            let (_receiver, arguments) = validated.into_parts();
            let receiver = pinned_receiver.downcast_ref::<__QubitReflectTarget>()
                .unwrap_or_else(|| unreachable!("validated invocation checked pinned receiver type"));
        },
        Some(ReceiverKindIr::Typed) => return None,
        None => quote! {
            let (_receiver, arguments) = validated.into_parts();
        },
    };
    let call = if method.receiver.is_some() {
        quote!(<__QubitReflectTarget as #trait_path>::#method_name(receiver, #(#call_arguments),*))
    } else {
        quote!(<__QubitReflectTarget as #trait_path>::#method_name(#(#call_arguments),*))
    };
    let call = if method.qualifiers.is_unsafe {
        quote!(unsafe { #call })
    } else {
        call
    };
    let borrow_origins = std::iter::once(
        method
            .receiver
            .is_some()
            .then(|| quote!(#codegen::invoke::BorrowOrigin::Receiver)),
    )
    .flatten()
    .chain(
        method
            .parameters
            .iter()
            .filter(|parameter| matches!(parameter.ty.kind, TypeKindIr::Reference { .. }))
            .map(|parameter| {
                let index = parameter.index;
                quote!(#codegen::invoke::BorrowOrigin::Parameter(#index))
            }),
    )
    .collect::<Vec<_>>();
    let output = match (&method.qualifiers.is_async, &method.return_type) {
        (true, ReturnTypeIr::Unit) => quote! {
            #codegen::invoke::InvocationOutput::future(async move {
                let mut arguments = arguments.into_vec().into_iter();
                #(#arguments)*
                #call.await;
                #codegen::invoke::InvocationOutput::unit()
            })
        },
        (true, _) => quote! {
            #codegen::invoke::InvocationOutput::future(async move {
                let mut arguments = arguments.into_vec().into_iter();
                #(#arguments)*
                #codegen::invoke::InvocationOutput::owned(#call.await)
            })
        },
        (false, ReturnTypeIr::Unit) => quote! {
            let mut arguments = arguments.into_vec().into_iter();
            #(#arguments)*
            #call;
            #codegen::invoke::InvocationOutput::unit()
        },
        (false, ReturnTypeIr::Type(ty)) if matches!(ty.kind, TypeKindIr::Never) => {
            quote! { let mut arguments = arguments.into_vec().into_iter(); #(#arguments)* match #call {} }
        }
        (false, ReturnTypeIr::Type(ty)) => {
            let output = match ty.kind {
                TypeKindIr::Reference { mutable: true, .. } => {
                    let origin = borrow_origins
                        .first()
                        .cloned()
                        .unwrap_or_else(|| quote!(#codegen::invoke::BorrowOrigin::Receiver));
                    quote!(#codegen::invoke::InvocationOutput::mutable(#call, #origin))
                }
                TypeKindIr::Reference { .. } => {
                    quote!(#codegen::invoke::InvocationOutput::shared_with_origins(
                        #call,
                        ::std::vec![#(#borrow_origins),*].into_boxed_slice(),
                    ))
                }
                _ => quote!(#codegen::invoke::InvocationOutput::owned(#call)),
            };
            quote! { let mut arguments = arguments.into_vec().into_iter(); #(#arguments)* #output }
        }
    };
    let overridden_name = LitStr::new(&method.name.to_string(), method.span);
    let adapter_arguments = if pinned_receiver_mutable.is_some() {
        quote! {
            pinned_receiver: #codegen::invoke::PinnedReceiver<'call>,
            validated: #codegen::invoke::ValidatedInvocation<'call, #mode>,
        }
    } else {
        quote!(validated: #codegen::invoke::ValidatedInvocation<'call, #mode>)
    };
    let adapter_constructor = if let Some(mutable) = pinned_receiver_mutable {
        let constructor = if mutable {
            quote!(mutable)
        } else {
            quote!(shared)
        };
        quote!(#codegen::invoke::PinnedFunctionInvocationAdapter::<#mode, __QubitReflectTarget>::#constructor(#adapter #target_generic_arguments))
    } else {
        quote!(#codegen::invoke::FunctionInvocationAdapter::<#mode>::new(#adapter #target_generic_arguments))
    };
    let adapter_slot = if pinned_receiver_mutable.is_some() {
        quote!(.pinned_local::<__QubitReflectTarget>(adapter))
    } else {
        quote!(.local(adapter))
    };
    Some(quote! {
        if !overridden.iter().any(|name| *name == #overridden_name) {
            fn #adapter<'call, #(#helper_generic_parameters),*>(
                #adapter_arguments
            ) -> #codegen::invoke::InvocationOutput<'call, #mode>
            where
                __QubitReflectTarget: #codegen::descriptor::Reflect + 'static,
                __QubitReflectTarget: #trait_path,
            {
                #receiver
                #output
            }
            let adapter = ::std::boxed::Box::leak(::std::boxed::Box::new(
                #adapter_constructor,
            ));
            let effective = #codegen::invoke::EffectiveMethod::for_type::<__QubitReflectTarget>(
                #spec_provider().method_at(#index),
            )
            .with_adapters(#codegen::invoke::InvocationAdapters::builder()#adapter_slot.build())
            .with_source(#codegen::invoke::MethodImplementationSource::Defaulted);
            methods.push(effective);
        }
    })
}

fn trait_generic_parts(
    parameters: &syn::punctuated::Punctuated<GenericParam, syn::Token![,]>,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut declarations = Vec::new();
    let mut arguments = Vec::new();
    for parameter in parameters {
        match parameter {
            GenericParam::Lifetime(parameter) => {
                let parameter = parameter.clone();
                declarations.push(quote!(#parameter));
                let lifetime = parameter.lifetime.clone();
                arguments.push(quote!(#lifetime));
            }
            GenericParam::Type(parameter) => {
                let mut parameter = parameter.clone();
                parameter.default = None;
                declarations.push(quote!(#parameter));
                let ident = parameter.ident;
                arguments.push(quote!(#ident));
            }
            GenericParam::Const(parameter) => {
                let mut parameter = parameter.clone();
                parameter.default = None;
                declarations.push(quote!(#parameter));
                let ident = parameter.ident;
                arguments.push(quote!(#ident));
            }
        }
    }
    (declarations, arguments)
}

fn ordered_generic_parameters(
    parameters: &syn::punctuated::Punctuated<GenericParam, syn::Token![,]>,
) -> Vec<TokenStream> {
    trait_generic_parts(parameters).0
}
