// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Generic adapters for trait methods that provide a default body.

use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::format_ident;
use quote::quote;
use syn::GenericParam;
use syn::ItemTrait;
use syn::LitStr;

use crate::expand::ExpansionContext;
use crate::ir::GenericBoundIr;
use crate::ir::GenericKindIr;
use crate::ir::MethodIr;
use crate::ir::PathArgumentIr;
use crate::ir::PathArgumentsIr;
use crate::ir::PathIr;
use crate::ir::ReceiverKindIr;
use crate::ir::ReturnTypeIr;
use crate::ir::TraitDeclarationIr;
use crate::ir::TypeIr;
use crate::ir::TypeKindIr;

/// Emits a target-generic provider for the executable default methods of a
/// trait.
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
    let mut probe_generic_parameters = retained
        .generics
        .params
        .iter()
        .map(|parameter| quote!(#parameter))
        .collect::<Vec<_>>();
    probe_generic_parameters.insert(
        lifetime_count,
        quote!(__QubitReflectTarget: #codegen::descriptor::Reflect + 'static),
    );
    let probe_generics = quote!(<#(#probe_generic_parameters),*>);
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
    let mut const_marker_items = Vec::new();
    let marker_parameters = retained
        .generics
        .params
        .iter()
        .enumerate()
        .map(|(index, parameter)| match parameter {
            GenericParam::Lifetime(parameter) => {
                let lifetime = &parameter.lifetime;
                quote!(&#lifetime ())
            }
            GenericParam::Type(parameter) => {
                let ident = &parameter.ident;
                quote!(#ident)
            }
            GenericParam::Const(parameter) => {
                let marker = format_ident!("{}_CONST_MARKER_{index}", provider);
                let ident = &parameter.ident;
                let ty = &parameter.ty;
                const_marker_items.push(quote! {
                    #[allow(non_camel_case_types)]
                    #[doc(hidden)]
                    pub struct #marker<const #ident: #ty>;
                });
                quote!(#marker<#ident>)
            }
        })
        .collect::<Vec<_>>();
    let marker_type = quote!(fn(#(#marker_parameters),*) -> __QubitReflectTarget);
    let value_eligibility = format_ident!("{}_THREAD_SAFE_VALUE_ELIGIBILITY", provider);
    let trait_type_parameters = declaration
        .generics
        .params
        .iter()
        .filter(|parameter| parameter.kind == GenericKindIr::Type)
        .map(|parameter| parameter.name.clone())
        .collect::<Vec<_>>();
    let method_context = DefaultMethodContext {
        trait_path: &trait_path,
        helper_generic_parameters: &helper_generic_parameters,
        marker_type: &marker_type,
        target_generic_arguments: &target_generic_arguments,
        spec_provider,
        codegen: &codegen,
        facade,
        context,
        trait_type_parameters: &trait_type_parameters,
    };
    let local_methods = declaration
        .methods
        .iter()
        .enumerate()
        .filter(|(_, method)| method.has_default)
        .filter_map(|(index, method)| emit_default_method(method, index, false, &method_context))
        .collect::<Vec<_>>();
    let thread_safe_adapters = declaration
        .methods
        .iter()
        .enumerate()
        .filter(|(_, method)| method.has_default)
        .map(|(index, method)| {
            (
                index,
                thread_safe_default_adapter(
                    method,
                    index,
                    &ThreadSafeDefaultAdapterContext {
                        trait_path: &trait_path,
                        helper_generic_parameters: &helper_generic_parameters,
                        marker_type: &marker_type,
                        target_generic_arguments: &target_generic_arguments,
                        codegen: &codegen,
                        value_eligibility: &value_eligibility,
                    },
                ),
            )
        })
        .collect::<Vec<_>>();
    let selector = format_ident!("{}_SELECTOR", provider);
    let mut macro_hash = std::collections::hash_map::DefaultHasher::new();
    use std::hash::Hash;
    use std::hash::Hasher;
    let declaration_span = trait_name.span();
    declaration_span.file().hash(&mut macro_hash);
    let declaration_start = declaration_span.start();
    declaration_start.line.hash(&mut macro_hash);
    declaration_start.column.hash(&mut macro_hash);
    retained.to_token_stream().to_string().hash(&mut macro_hash);
    let exported_callsite_macro = format_ident!("__qubit_reflect_trait_defaults_{:016x}", macro_hash.finish(),);
    let callsite_macro = format_ident!("{}_CALLSITE", provider);
    // The callsite macro forwards the concrete lifetime arguments written in
    // the impl's trait path, so lifetime-generic applications can use the
    // same per-application selector as type and const generic applications.
    let selector_item = {
        let method_selections = thread_safe_adapters
            .iter()
            .map(|(index, _)| {
                let method_probe = format_ident!("{}_METHOD_{}_PROBE", provider, index);
                quote! {
                    methods.extend((&&#method_probe::<$($lifetime,)* $target, $($generic_arg)*>::new())
                        .select(overridden)
                        .into_vec());
                }
            })
            .collect::<Vec<_>>();
        let method_selectors = thread_safe_adapters
            .iter()
            .map(|(index, adapter)| {
                let probe = format_ident!("{}_METHOD_{}_PROBE", provider, index);
                let local_method = emit_default_method(&declaration.methods[*index], *index, false, &method_context);
                let thread_safe_method = adapter
                    .as_ref()
                    .and_then(|_| emit_default_method(&declaration.methods[*index], *index, true, &method_context));
                let thread_safe_impl = adapter
                    .as_ref()
                    .zip(thread_safe_method.as_ref())
                    .map(|(adapter, method)| {
                        let bounds = &adapter.bounds;
                        quote! {
                            impl #helper_generics #selector for &&#probe #target_generic_arguments
                            where
                                __QubitReflectTarget: #trait_path,
                                #(#bounds)*
                            {
                                fn select(
                                    self,
                                    overridden: &[&'static str],
                                ) -> ::std::boxed::Box<[#codegen::invoke::EffectiveMethod]> {
                                    let mut methods: ::std::vec::Vec<#codegen::invoke::EffectiveMethod> =
                                        ::std::vec::Vec::new();
                                    #method
                                    methods.into_boxed_slice()
                                }
                            }
                        }
                    });
                let local_impl = quote! {
                    #[allow(non_camel_case_types)]
                    #[doc(hidden)]
                    pub struct #probe #probe_generics (::core::marker::PhantomData<#marker_type>);
                    impl #helper_generics #probe #target_generic_arguments {
                        pub fn new() -> Self {
                            Self(::core::marker::PhantomData)
                        }
                    }

                    impl #helper_generics #selector for &#probe #target_generic_arguments
                    where
                        __QubitReflectTarget: #trait_path,
                    {
                        fn select(
                            self,
                            overridden: &[&'static str],
                        ) -> ::std::boxed::Box<[#codegen::invoke::EffectiveMethod]> {
                            let mut methods: ::std::vec::Vec<#codegen::invoke::EffectiveMethod> =
                                ::std::vec::Vec::new();
                            #local_method
                            methods.into_boxed_slice()
                        }
                    }
                };
                quote!(#local_impl #thread_safe_impl)
            })
            .collect::<Vec<_>>();
        quote! {
            #(#const_marker_items)*

            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            pub trait #value_eligibility<__QubitReflectTarget>: Sized {
                fn into_owned(
                    value: #codegen::invoke::InvocationValue<#codegen::value::ThreadSafe>,
                ) -> ::core::result::Result<
                    Self,
                    #codegen::invoke::InvocationValue<#codegen::value::ThreadSafe>,
                >;
                fn output<'call>(
                    value: Self,
                ) -> #codegen::invoke::InvocationOutput<'call, #codegen::value::ThreadSafe>;
            }
            impl<__QubitReflectTarget, __QubitReflectValue>
                #value_eligibility<__QubitReflectTarget> for __QubitReflectValue
            where
                __QubitReflectValue: #codegen::value::ValueFor<#codegen::value::ThreadSafe>,
            {
                fn into_owned(
                    value: #codegen::invoke::InvocationValue<#codegen::value::ThreadSafe>,
                ) -> ::core::result::Result<
                    Self,
                    #codegen::invoke::InvocationValue<#codegen::value::ThreadSafe>,
                > {
                    value.into_owned::<Self>()
                }
                fn output<'call>(
                    value: Self,
                ) -> #codegen::invoke::InvocationOutput<'call, #codegen::value::ThreadSafe> {
                    #codegen::invoke::InvocationOutput::owned(value)
                }
            }

            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            pub trait #selector {
                fn select(
                    self,
                    overridden: &[&'static str],
                ) -> ::std::boxed::Box<[#codegen::invoke::EffectiveMethod]>;
            }

            #[macro_export]
            macro_rules! #exported_callsite_macro {
                (($module:path), ($facade:ident), $target:ty, [$($lifetime:lifetime),*], [$($generic_arg:tt)*], $overridden:expr) => {{
                    #[allow(unused_imports)]
                    use $module::*;
                    #[allow(unused_variables)]
                    let overridden: &[&'static str] = $overridden;
                    #[allow(unused_mut)]
                    let mut methods: ::std::vec::Vec<$facade::__private::codegen_v4::invoke::EffectiveMethod> =
                        ::std::vec::Vec::new();
                    #(#method_selections)*
                    methods.into_boxed_slice()
                }};
            }
            #[allow(macro_expanded_macro_exports_accessed_by_absolute_paths)]
            pub use #exported_callsite_macro as #callsite_macro;

            #(#method_selectors)*
        }
    };
    quote! {
        #[allow(non_snake_case)]
        #[doc(hidden)]
        pub fn #provider #helper_generics (
            overridden: &[&'static str],
        ) -> ::std::boxed::Box<[#codegen::invoke::EffectiveMethod]>
        where
            __QubitReflectTarget: #trait_path,
        {
            let mut methods: ::std::vec::Vec<#codegen::invoke::EffectiveMethod> = ::std::vec::Vec::new();
            #(#local_methods)*
            methods.into_boxed_slice()
        }

        #selector_item
    }
}

struct DefaultMethodContext<'a> {
    trait_path: &'a TokenStream,
    helper_generic_parameters: &'a [TokenStream],
    marker_type: &'a TokenStream,
    target_generic_arguments: &'a TokenStream,
    spec_provider: &'a Ident,
    codegen: &'a TokenStream,
    facade: &'a TokenStream,
    context: &'a ExpansionContext,
    trait_type_parameters: &'a [String],
}

fn emit_default_method(
    method: &MethodIr,
    index: usize,
    thread_safe_supported: bool,
    context: &DefaultMethodContext<'_>,
) -> Option<TokenStream> {
    let DefaultMethodContext {
        trait_path,
        helper_generic_parameters,
        marker_type,
        target_generic_arguments,
        spec_provider,
        codegen,
        facade,
        context: expansion_context,
        trait_type_parameters,
    } = context;
    let target_type = quote!(__QubitReflectTarget);
    let method_context = crate::expand::invocation::analysis::MethodContext::trait_default(
        &target_type,
        method_uses_unproven_associated_type(method, trait_type_parameters),
    );
    let plan = crate::expand::invocation::analysis::analyze_method(method, method_context)
        .expect("validated trait method analysis is infallible");
    if !plan.is_executable() || !method.generics.params.is_empty() {
        let reasons = crate::expand::invocation::emit::emit_unavailable_reasons(&plan, expansion_context);
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
        .map(|parameter| crate::expand::invocation::emit::argument_binding(parameter, facade, &mode))
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
    let optional_borrow_origin =
        crate::expand::invocation::analysis::optional_shared_borrow_origin(method).map(|origin| match origin {
            crate::expand::invocation::analysis::OptionalBorrowOrigin::Receiver => {
                quote!(#codegen::invoke::BorrowOrigin::Receiver)
            }
            crate::expand::invocation::analysis::OptionalBorrowOrigin::Parameter(index) => {
                quote!(#codegen::invoke::BorrowOrigin::Parameter(#index))
            }
        });
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
                        &[#(#borrow_origins),*],
                    ))
                }
                TypeKindIr::Path(_) if optional_borrow_origin.is_some() => {
                    let origin = optional_borrow_origin
                        .as_ref()
                        .expect("the optional borrowed output has a statically inferred origin");
                    quote!(#codegen::invoke::InvocationOutput::optional_shared_with_origins(
                        #call,
                        &[#origin],
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
    let adapter_path = quote!(<#adapter #target_generic_arguments>::invoke);
    let helper_generics = quote!(<#(#helper_generic_parameters),*>);
    let adapter_constructor = if let Some(mutable) = pinned_receiver_mutable {
        let constructor = if mutable { quote!(mutable) } else { quote!(shared) };
        quote!(#codegen::invoke::PinnedFunctionInvocationAdapter::<#mode, __QubitReflectTarget>::#constructor(#adapter_path))
    } else {
        quote!(#codegen::invoke::FunctionInvocationAdapter::<#mode>::new(#adapter_path))
    };
    let adapter_slot = if pinned_receiver_mutable.is_some() {
        quote!(.pinned_local::<__QubitReflectTarget>(adapter))
    } else if method.qualifiers.is_async || method.qualifiers.is_unsafe {
        quote!(.local(adapter))
    } else {
        quote!(.local(adapter).catching_local(adapter))
    };
    let thread_safe = if thread_safe_supported {
        let provider_name = spec_provider.to_string();
        let trait_provider = provider_name.trim_end_matches("_SPEC");
        let eligibility = format_ident!("{}_DEFAULT_METHODS_THREAD_SAFE_VALUE_ELIGIBILITY", trait_provider,);
        thread_safe_default_adapter(
            method,
            index,
            &ThreadSafeDefaultAdapterContext {
                trait_path,
                helper_generic_parameters,
                marker_type,
                target_generic_arguments,
                codegen,
                value_eligibility: &eligibility,
            },
        )
    } else {
        None
    };
    let adapter_registration = if let Some(thread_safe) = thread_safe {
        let definition = &thread_safe.definition;
        let invoke = &thread_safe.invoke;
        let adapter_type = &thread_safe.adapter_type;
        quote! {
            #definition
            let thread_safe_adapter = ::std::boxed::Box::leak(::std::boxed::Box::new(
                #codegen::invoke::FunctionInvocationAdapter::<#codegen::value::ThreadSafe>::new(
                    <#adapter_type #target_generic_arguments>::#invoke,
                ),
            ));
            let builder = #codegen::invoke::InvocationAdapters::builder()
                .local(adapter)
                .catching_local(adapter)
                .thread_safe(thread_safe_adapter)
                .catching_thread_safe(thread_safe_adapter);
            let effective = #codegen::invoke::EffectiveMethod::for_type::<__QubitReflectTarget>(
                #spec_provider().method_at(#index),
            )
            .with_adapters(builder.build())
            .with_source(#codegen::invoke::MethodImplementationSource::Defaulted);
            methods.push(effective);
        }
    } else {
        quote! {
            let effective = #codegen::invoke::EffectiveMethod::for_type::<__QubitReflectTarget>(
                #spec_provider().method_at(#index),
            )
            .with_adapters(#codegen::invoke::InvocationAdapters::builder()#adapter_slot.build())
            .with_source(#codegen::invoke::MethodImplementationSource::Defaulted);
            methods.push(effective);
        }
    };
    Some(quote! {
        if !overridden.iter().any(|name| *name == #overridden_name) {
            struct #adapter #helper_generics (
                ::core::marker::PhantomData<#marker_type>,
            );
            impl #helper_generics #adapter #target_generic_arguments {
                fn invoke<'call>(
                    #adapter_arguments
                ) -> #codegen::invoke::InvocationOutput<'call, #mode>
                where
                    __QubitReflectTarget: #codegen::descriptor::Reflect + 'static,
                    __QubitReflectTarget: #trait_path,
                {
                    #receiver
                    #output
                }
            }
            let adapter = ::std::boxed::Box::leak(::std::boxed::Box::new(
                #adapter_constructor,
            ));
            #adapter_registration
        }
    })
}

fn method_uses_unproven_associated_type(method: &MethodIr, trait_type_parameters: &[String]) -> bool {
    method
        .receiver
        .as_ref()
        .is_some_and(|receiver| type_uses_unproven_associated_type(&receiver.ty, trait_type_parameters))
        || method
            .parameters
            .iter()
            .any(|parameter| type_uses_unproven_associated_type(&parameter.ty, trait_type_parameters))
        || matches!(
            &method.return_type,
            ReturnTypeIr::Type(ty) if type_uses_unproven_associated_type(ty, trait_type_parameters)
        )
}

fn type_uses_unproven_associated_type(ty: &TypeIr, trait_type_parameters: &[String]) -> bool {
    match &ty.kind {
        TypeKindIr::Path(path) => {
            path_uses_unproven_associated_type(path, trait_type_parameters)
                || path.segments.iter().any(|segment| match &segment.arguments {
                    PathArgumentsIr::None => false,
                    PathArgumentsIr::AngleBracketed(arguments) => arguments.iter().any(|argument| match argument {
                        PathArgumentIr::Type(ty) | PathArgumentIr::AssociatedType { ty, .. } => {
                            type_uses_unproven_associated_type(ty, trait_type_parameters)
                        }
                        PathArgumentIr::Constraint { bounds, .. } => bounds
                            .iter()
                            .any(|bound| bound_uses_unproven_associated_type(bound, trait_type_parameters)),
                        PathArgumentIr::Lifetime(_)
                        | PathArgumentIr::Const(_)
                        | PathArgumentIr::AssociatedConst { .. }
                        | PathArgumentIr::Other(_) => false,
                    }),
                    PathArgumentsIr::Parenthesized { inputs, output } => {
                        inputs
                            .iter()
                            .any(|ty| type_uses_unproven_associated_type(ty, trait_type_parameters))
                            || output
                                .as_ref()
                                .is_some_and(|ty| type_uses_unproven_associated_type(ty, trait_type_parameters))
                    }
                })
        }
        TypeKindIr::Reference { element, .. } | TypeKindIr::Slice(element) | TypeKindIr::Pointer { element, .. } => {
            type_uses_unproven_associated_type(element, trait_type_parameters)
        }
        TypeKindIr::Tuple(elements) => elements
            .iter()
            .any(|element| type_uses_unproven_associated_type(element, trait_type_parameters)),
        TypeKindIr::Array { element, .. } => type_uses_unproven_associated_type(element, trait_type_parameters),
        TypeKindIr::BareFunction { inputs, output, .. } => {
            inputs
                .iter()
                .any(|input| type_uses_unproven_associated_type(input, trait_type_parameters))
                || output
                    .as_ref()
                    .is_some_and(|output| type_uses_unproven_associated_type(output, trait_type_parameters))
        }
        TypeKindIr::TraitObject { bounds, .. } | TypeKindIr::ImplTrait { bounds } => bounds
            .iter()
            .any(|bound| bound_uses_unproven_associated_type(bound, trait_type_parameters)),
        TypeKindIr::Never | TypeKindIr::Infer | TypeKindIr::Macro | TypeKindIr::Other => false,
    }
}

fn path_uses_unproven_associated_type(path: &PathIr, trait_type_parameters: &[String]) -> bool {
    let direct_projection = path.qualified_self.is_none()
        && path.segments.len() > 1
        && path
            .segments
            .first()
            .is_some_and(|segment| is_trait_type_parameter(&segment.name, trait_type_parameters));
    let qualified_projection = path.qualified_self.as_ref().is_some_and(|qualified_self| {
        path.segments.len() > qualified_self.position
            && type_references_trait_parameter(&qualified_self.ty, trait_type_parameters)
    });
    direct_projection
        || qualified_projection
        || path.segments.iter().any(|segment| match &segment.arguments {
            PathArgumentsIr::None => false,
            PathArgumentsIr::AngleBracketed(arguments) => arguments.iter().any(|argument| match argument {
                PathArgumentIr::Type(ty) | PathArgumentIr::AssociatedType { ty, .. } => {
                    type_uses_unproven_associated_type(ty, trait_type_parameters)
                }
                PathArgumentIr::Constraint { bounds, .. } => bounds
                    .iter()
                    .any(|bound| bound_uses_unproven_associated_type(bound, trait_type_parameters)),
                PathArgumentIr::Lifetime(_)
                | PathArgumentIr::Const(_)
                | PathArgumentIr::AssociatedConst { .. }
                | PathArgumentIr::Other(_) => false,
            }),
            PathArgumentsIr::Parenthesized { inputs, output } => {
                inputs
                    .iter()
                    .any(|ty| type_uses_unproven_associated_type(ty, trait_type_parameters))
                    || output
                        .as_ref()
                        .is_some_and(|ty| type_uses_unproven_associated_type(ty, trait_type_parameters))
            }
        })
}

fn type_references_trait_parameter(ty: &TypeIr, trait_type_parameters: &[String]) -> bool {
    match &ty.kind {
        TypeKindIr::Path(path) => {
            path.qualified_self.as_ref().is_some_and(|qualified_self| {
                type_references_trait_parameter(&qualified_self.ty, trait_type_parameters)
            }) || path
                .segments
                .first()
                .is_some_and(|segment| is_trait_type_parameter(&segment.name, trait_type_parameters))
                || path.segments.iter().any(|segment| match &segment.arguments {
                    PathArgumentsIr::None => false,
                    PathArgumentsIr::AngleBracketed(arguments) => arguments.iter().any(|argument| match argument {
                        PathArgumentIr::Type(ty) | PathArgumentIr::AssociatedType { ty, .. } => {
                            type_references_trait_parameter(ty, trait_type_parameters)
                        }
                        _ => false,
                    }),
                    PathArgumentsIr::Parenthesized { inputs, output } => {
                        inputs
                            .iter()
                            .any(|ty| type_references_trait_parameter(ty, trait_type_parameters))
                            || output
                                .as_ref()
                                .is_some_and(|ty| type_references_trait_parameter(ty, trait_type_parameters))
                    }
                })
        }
        TypeKindIr::Reference { element, .. } | TypeKindIr::Slice(element) | TypeKindIr::Pointer { element, .. } => {
            type_references_trait_parameter(element, trait_type_parameters)
        }
        TypeKindIr::Tuple(elements) => elements
            .iter()
            .any(|element| type_references_trait_parameter(element, trait_type_parameters)),
        TypeKindIr::Array { element, .. } => type_references_trait_parameter(element, trait_type_parameters),
        TypeKindIr::BareFunction { inputs, output, .. } => {
            inputs
                .iter()
                .any(|input| type_references_trait_parameter(input, trait_type_parameters))
                || output
                    .as_ref()
                    .is_some_and(|output| type_references_trait_parameter(output, trait_type_parameters))
        }
        TypeKindIr::TraitObject { bounds, .. } | TypeKindIr::ImplTrait { bounds } => bounds
            .iter()
            .any(|bound| bound_uses_unproven_associated_type(bound, trait_type_parameters)),
        TypeKindIr::Never | TypeKindIr::Infer | TypeKindIr::Macro | TypeKindIr::Other => false,
    }
}

fn bound_uses_unproven_associated_type(bound: &GenericBoundIr, trait_type_parameters: &[String]) -> bool {
    match bound {
        GenericBoundIr::Trait { path, .. } => path_uses_unproven_associated_type(path, trait_type_parameters),
        GenericBoundIr::Lifetime(_) | GenericBoundIr::Other(_) => false,
    }
}

fn is_trait_type_parameter(name: &str, trait_type_parameters: &[String]) -> bool {
    name == "Self" || trait_type_parameters.iter().any(|parameter| parameter == name)
}

/// Generates a thread-safe adapter only for the narrow, fully owned signature
/// subset whose value-mode bounds can be proven by stable Rust.
#[must_use]
struct ThreadSafeDefaultAdapter {
    definition: TokenStream,
    adapter_type: Ident,
    invoke: Ident,
    bounds: Vec<TokenStream>,
}

struct ThreadSafeDefaultAdapterContext<'a> {
    trait_path: &'a TokenStream,
    helper_generic_parameters: &'a [TokenStream],
    marker_type: &'a TokenStream,
    target_generic_arguments: &'a TokenStream,
    codegen: &'a TokenStream,
    value_eligibility: &'a Ident,
}

fn thread_safe_default_adapter(
    method: &MethodIr,
    index: usize,
    context: &ThreadSafeDefaultAdapterContext<'_>,
) -> Option<ThreadSafeDefaultAdapter> {
    let ThreadSafeDefaultAdapterContext {
        trait_path,
        helper_generic_parameters,
        marker_type,
        target_generic_arguments,
        codegen,
        value_eligibility,
    } = context;
    if method.qualifiers.is_async
        || method.qualifiers.is_unsafe
        || !method.generics.params.is_empty()
        || crate::expand::invocation::analysis::is_optional_borrow_return(&method.return_type)
        || crate::expand::invocation::analysis::return_contains_non_static_lifetime(&method.return_type)
        || method.parameters.iter().any(|parameter| {
            !crate::expand::invocation::analysis::supports_invocation_parameter(&parameter.ty)
                || type_mentions_self(&parameter.ty.tokens)
        })
    {
        return None;
    }
    let output_type = match &method.return_type {
        ReturnTypeIr::Unit => None,
        ReturnTypeIr::Type(ty)
            if matches!(
                ty.kind,
                TypeKindIr::Path(_)
                    | TypeKindIr::Tuple(_)
                    | TypeKindIr::Array { .. }
                    | TypeKindIr::Pointer { .. }
                    | TypeKindIr::BareFunction { .. }
            ) && !type_mentions_self(&ty.tokens) =>
        {
            Some(ty.tokens.clone())
        }
        ReturnTypeIr::Type(_) => return None,
    };
    let receiver_kind = match method.receiver.as_ref() {
        None => None,
        Some(receiver) => match receiver.kind {
            ReceiverKindIr::Value | ReceiverKindIr::SharedReference | ReceiverKindIr::MutableReference => {
                Some(receiver.kind)
            }
            ReceiverKindIr::Typed => return None,
        },
    };

    let value_mode = quote!(#codegen::value::ThreadSafe);
    let adapter_type = format_ident!("__qubit_reflect_default_thread_safe_adapter_{index}");
    let invoke = format_ident!("__qubit_reflect_default_thread_safe_invoke_{index}");
    let method_name = &method.name;
    let arguments = method
        .parameters
        .iter()
        .map(|parameter| format_ident!("__qubit_reflect_argument_{}", parameter.index))
        .collect::<Vec<_>>();
    let receiver_setup = match receiver_kind {
        Some(ReceiverKindIr::Value) => quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver = receiver.expect("validated invocation has a receiver")
                .into_owned::<__QubitReflectTarget>()
                .unwrap_or_else(|_| unreachable!("validated invocation checked receiver type and mode"));
        },
        Some(ReceiverKindIr::SharedReference) => quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver = receiver.expect("validated invocation has a receiver")
                .into_ref::<__QubitReflectTarget>()
                .unwrap_or_else(|_| unreachable!("validated invocation checked receiver type and mode"));
        },
        Some(ReceiverKindIr::MutableReference) => quote! {
            let (receiver, arguments) = validated.into_parts();
            let receiver = receiver.expect("validated invocation has a receiver")
                .into_mut::<__QubitReflectTarget>()
                .unwrap_or_else(|_| unreachable!("validated invocation checked receiver type and mode"));
        },
        None => quote! {
            let (_receiver, arguments) = validated.into_parts();
        },
        Some(ReceiverKindIr::Typed) => return None,
    };
    let call_receiver = receiver_kind.is_some().then(|| quote!(receiver,));
    let invoke_args = arguments.iter().map(|argument| quote!(#argument));
    let call = quote!(
        <__QubitReflectTarget as #trait_path>::#method_name(#call_receiver #(#invoke_args),*)
    );
    let bindings = arguments
        .iter()
        .zip(method.parameters.iter())
        .map(|(argument, parameter)| {
            let ty = &parameter.ty.tokens;
            match &parameter.ty.kind {
                TypeKindIr::Reference {
                    mutable: false,
                    element,
                    ..
                } => {
                    let element_type = &element.tokens;
                    quote! {
                        let #argument: #ty = arguments.next()
                            .expect("validated invocation contains each declared argument")
                            .into_ref::<#element_type>()
                            .unwrap_or_else(|_| unreachable!("validated invocation checked argument type and mode"));
                    }
                }
                TypeKindIr::Reference {
                    mutable: true, element, ..
                } => {
                    let element_type = &element.tokens;
                    quote! {
                        let #argument: #ty = arguments.next()
                            .expect("validated invocation contains each declared argument")
                            .into_mut::<#element_type>()
                            .unwrap_or_else(|_| unreachable!("validated invocation checked argument type and mode"));
                    }
                }
                _ => quote! {
                    let #argument: #ty = <#ty as #value_eligibility<__QubitReflectTarget>>::into_owned(
                        arguments.next().expect("validated invocation contains each declared argument"),
                    )
                    .unwrap_or_else(|_| unreachable!("validated invocation checked argument type and mode"));
                },
            }
        });
    let result = if let Some(output_type) = output_type.as_ref() {
        quote!(<#output_type as #value_eligibility<__QubitReflectTarget>>::output(#call))
    } else {
        quote! { #call; #codegen::invoke::InvocationOutput::unit() }
    };
    let target_bound = match receiver_kind {
        Some(ReceiverKindIr::Value) => quote!(
            __QubitReflectTarget: #codegen::value::ValueFor<#value_mode>,
        ),
        Some(ReceiverKindIr::SharedReference) => quote!(
            __QubitReflectTarget: #codegen::value::ValueForRef<#value_mode>,
        ),
        Some(ReceiverKindIr::MutableReference) => quote!(
            __QubitReflectTarget: #codegen::value::ValueForMut<#value_mode>,
        ),
        None => quote!(),
        Some(ReceiverKindIr::Typed) => return None,
    };
    let parameter_bounds = method
        .parameters
        .iter()
        .map(|parameter| match &parameter.ty.kind {
            TypeKindIr::Reference {
                mutable: false,
                element,
                ..
            } => {
                let element_type = &element.tokens;
                quote!(#element_type: #codegen::value::ValueForRef<#value_mode>,)
            }
            TypeKindIr::Reference {
                mutable: true, element, ..
            } => {
                let element_type = &element.tokens;
                quote!(#element_type: #codegen::value::ValueForMut<#value_mode>,)
            }
            _ => {
                let ty = &parameter.ty.tokens;
                quote!(#ty: #value_eligibility<__QubitReflectTarget>,)
            }
        })
        .collect::<Vec<_>>();
    let output_bound = output_type
        .as_ref()
        .map(|ty| quote!(#ty: #value_eligibility<__QubitReflectTarget>,))
        .into_iter()
        .collect::<Vec<_>>();
    let mut bounds = vec![quote!(__QubitReflectTarget: #trait_path,)];
    if !target_bound.is_empty() {
        bounds.push(target_bound);
    }
    bounds.extend(parameter_bounds.iter().cloned());
    bounds.extend(output_bound.iter().cloned());
    let thread_safe_bounds = quote! { #(#bounds)* };
    let helper_generics = quote!(<#(#helper_generic_parameters),*>);
    let definition = quote! {
        struct #adapter_type #helper_generics (
            ::core::marker::PhantomData<#marker_type>,
        );
        impl #helper_generics #adapter_type #target_generic_arguments {
            fn #invoke<'call>(
                validated: #codegen::invoke::ValidatedInvocation<'call, #value_mode>,
            ) -> #codegen::invoke::InvocationOutput<'call, #value_mode>
            where
                #thread_safe_bounds
            {
                #receiver_setup
                let mut arguments = arguments.into_vec().into_iter();
                #(#bindings)*
                #result
            }
        }
    };
    Some(ThreadSafeDefaultAdapter {
        definition,
        adapter_type,
        invoke,
        bounds,
    })
}

fn type_mentions_self(ty: &TokenStream) -> bool {
    ty.to_string()
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .any(|identifier| identifier == "Self")
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
