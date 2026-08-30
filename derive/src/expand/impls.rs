//! Expansion of distributed registration fragments for `#[reflect_impl]`.

use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::format_ident;
use quote::quote;

use crate::ir::HelperName;
use crate::ir::HelperValueIr;
use crate::ir::GenericKindIr;
use crate::ir::ImplDeclarationIr;
use crate::ir::MethodIr;
use crate::ir::ParameterIr;
use crate::ir::ParameterPatternKindIr;
use crate::ir::PathArgumentIr;
use crate::ir::PathArgumentsIr;
use crate::ir::ReceiverKindIr;
use crate::ir::ReturnTypeIr;
use crate::ir::SpecializationIr;
use crate::ir::SpecializationValueIr;
use crate::ir::TypeIr;
use crate::ir::TypeKindIr;
use crate::ir::VisibilityIr;

/// Expands an impl unchanged and submits a lazily-built implementation
/// fragment.
///
/// The descriptor graph is deliberately constructed during registry
/// initialization, not from an inventory constructor. This keeps user code out
/// of linker startup and uses the T12 registration protocol exclusively.
pub(crate) fn expand_impl(declaration: ImplDeclarationIr) -> TokenStream {
    if !declaration.generics.params.is_empty() {
        return expand_generic_impl_specializations(declaration);
    }
    expand_concrete_impl(declaration, quote!(::std::vec![]))
}

/// Retains a generic impl and emits one concrete registration fragment for
/// every explicit specialization.
///
/// Blanket and constrained impls have no finite runtime `TypeId` set, so an
/// impl without `specialize(...)` deliberately remains descriptor-only.
fn expand_generic_impl_specializations(declaration: ImplDeclarationIr) -> TokenStream {
    let retained = declaration.retained_tokens.clone();
    let Some(facade) = facade_path() else {
        return retained;
    };
    if declaration.specializations.is_empty() {
        return retained;
    }
    let fragments = declaration.specializations.iter().map(|specialization| {
        let specialization_arguments = specialization_arguments(specialization, &declaration.generics, &facade);
        let arguments = quote!(#specialization_arguments.into_vec());
        let replacements = specialization_replacements(specialization);
        let mut concrete = declaration.clone();
        concrete.retained_tokens = TokenStream::new();
        concrete.specializations.clear();
        substitute_type_tokens(&mut concrete.target_type, &replacements);
        if let Some(trait_path) = &mut concrete.trait_path {
            trait_path.tokens = substitute_tokens(&trait_path.tokens, &replacements);
            trait_path.source = trait_path.tokens.to_string();
        }
        substitute_impl_method_types(&mut concrete, &replacements);
        expand_concrete_impl(concrete, arguments)
    });
    quote!(#retained #(#fragments)*)
}

/// Expands one concrete implementation registration fragment.
fn expand_concrete_impl(declaration: ImplDeclarationIr, impl_arguments: TokenStream) -> TokenStream {
    let Some(facade) = facade_path() else {
        // Parser-only consumers of the standalone derive crate intentionally
        // have no runtime facade. Retaining the validated Rust impl preserves
        // its business semantics without fabricating a registration dependency.
        return declaration.retained_tokens;
    };
    let retained = declaration.retained_tokens;
    let target = declaration.target_type.tokens;
    let impl_generic_definition = super::traits::generic_definition(&declaration.generics, declaration.span, &facade);
    let trait_path = declaration.trait_path.as_ref().map(|path| path.tokens.clone());
    let has_trait = trait_path.is_some();
    let external_id = declaration
        .attributes
        .iter()
        .find_map(|attribute| match &attribute.value {
            HelperValueIr::ExternalTraitId(value) => Some(value.as_str()),
            _ => None,
        });
    let fingerprint = fingerprint(&format!("{}{}", retained, target));
    let location = declaration.span.start();
    let line = location.line as u32;
    let column = location.column as u32;
    let module = format_ident!("__qubit_reflect_impl_{fingerprint:x}_{line}_{column}");
    let target_source = declaration.target_type.source;
    let invocation_adapter_definitions = declaration
        .methods
        .iter()
        .filter(|method| {
            !method
                .attributes
                .iter()
                .any(|attribute| attribute.name == HelperName::Skip)
        })
        .enumerate()
        .filter_map(|(index, method)| {
            let typed_owned_receiver = method
                .receiver
                .as_ref()
                .and_then(|receiver| typed_owned_receiver_type(receiver, &target));
            let typed_pinned_receiver = method
                .receiver
                .as_ref()
                .and_then(typed_pinned_receiver_mutable);
            let typed_extension_receiver = method
                .receiver
                .as_ref()
                .and_then(|receiver| typed_extension_receiver_type(receiver, &target));
            let supported_receiver = matches!(
                method.receiver.as_ref().map(|receiver| receiver.kind),
                None | Some(ReceiverKindIr::Value)
                    | Some(ReceiverKindIr::SharedReference)
                    | Some(ReceiverKindIr::MutableReference)
            ) || typed_owned_receiver.is_some()
                || typed_pinned_receiver.is_some()
                || typed_extension_receiver.is_some();
            let supported_parameters = method
                .parameters
                .iter()
                .all(|parameter| supports_invocation_parameter(&parameter.ty));
            let is_safe_invocation = !has_trait
                && supported_receiver
                && supported_parameters
                && method.generics.params.is_empty()
                && !method.qualifiers.is_unsafe
                && method.qualifiers.abi.is_none()
                && !method.qualifiers.is_variadic
                && (!return_contains_non_static_lifetime(&method.return_type)
                    || is_supported_shared_borrow_return(&method.return_type)
                    || is_supported_mutable_borrow_return(method))
                && (!method.qualifiers.is_async || !is_borrow_return(&method.return_type))
                && !method
                    .attributes
                    .iter()
                    .any(|attribute| attribute.name == HelperName::NoInvoke)
                && matches!(
                    method.return_type,
                    ReturnTypeIr::Unit
                        | ReturnTypeIr::Type(crate::ir::TypeIr {
                            kind: TypeKindIr::Path(_) | TypeKindIr::Reference { .. },
                            ..
                        })
                );
            let pinned_receiver_mutable = method
                .receiver
                .as_ref()
                .and_then(typed_pinned_receiver_mutable);
            if let Some(pinned_mutable) = pinned_receiver_mutable {
                let is_safe_pinned_invocation = !has_trait
                    && supported_parameters
                    && method.generics.params.is_empty()
                    && !method.qualifiers.is_async
                    && !method.qualifiers.is_unsafe
                    && method.qualifiers.abi.is_none()
                    && !method.qualifiers.is_variadic
                    && !is_borrow_return(&method.return_type)
                    && !method
                        .attributes
                        .iter()
                        .any(|attribute| attribute.name == HelperName::NoInvoke)
                    && matches!(
                        method.return_type,
                        ReturnTypeIr::Unit
                            | ReturnTypeIr::Type(crate::ir::TypeIr {
                                kind: TypeKindIr::Path(_),
                                ..
                            })
                    );
                if is_safe_pinned_invocation {
                    let method_name = &method.name;
                    let adapter_name = format_ident!("__qubit_reflect_invoke_pinned_{index}");
                    let adapter_token_name = format_ident!("__QUBIT_REFLECT_PINNED_ADAPTER_{index}");
                    let descriptor_name = format_ident!("__QUBIT_REFLECT_INVOCATION_ADAPTER_{index}");
                    let parameter_expectations: Vec<_> = method
                        .parameters
                        .iter()
                        .map(|parameter| invocation_argument_expectation(parameter, &facade))
                        .collect();
                    let mode = quote!(#facade::value::Local);
                    let argument_bindings: Vec<_> = method
                        .parameters
                        .iter()
                        .map(|parameter| invocation_argument_binding(parameter, &facade, &mode))
                        .collect();
                    let call_arguments: Vec<_> = method
                        .parameters
                        .iter()
                        .map(|parameter| format_ident!("__qubit_reflect_argument_{}", parameter.index))
                        .collect();
                    let invocation_type = if pinned_mutable {
                        quote!(#facade::invoke::PinnedMutInvocation<'call, #target, #facade::value::Local>)
                    } else {
                        quote!(#facade::invoke::PinnedRefInvocation<'call, #target, #facade::value::Local>)
                    };
                    let failure_type = if pinned_mutable {
                        quote!(#facade::invoke::PinnedMutInvocationFailure<'call, #target, #facade::value::Local>)
                    } else {
                        quote!(#facade::invoke::PinnedRefInvocationFailure<'call, #target, #facade::value::Local>)
                    };
                    let adapter_type = if pinned_mutable {
                        quote!(#facade::invoke::PinnedMutAdapter<#target, #facade::value::Local>)
                    } else {
                        quote!(#facade::invoke::PinnedRefAdapter<#target, #facade::value::Local>)
                    };
                    let constructor = if pinned_mutable {
                        quote!(#facade::descriptor::InvocationAdapter::pinned_mut_local(&#adapter_token_name))
                    } else {
                        quote!(#facade::descriptor::InvocationAdapter::pinned_ref_local(&#adapter_token_name))
                    };
                    let output = match method.return_type {
                        ReturnTypeIr::Unit => quote! {
                            <#target>::#method_name(receiver, #(#call_arguments),*);
                            #facade::invoke::InvocationOutput::Unit
                        },
                        ReturnTypeIr::Type(_) => quote! {
                            #facade::invoke::InvocationOutput::Owned(
                                #facade::value::DynamicOwned::<#facade::value::Local>::new(
                                    <#target>::#method_name(receiver, #(#call_arguments),*),
                                ),
                            )
                        },
                    };
                    return Some(quote! {
                        fn #adapter_name<'call>(
                            invocation: #invocation_type,
                        ) -> ::core::result::Result<
                            #facade::invoke::InvocationOutput<'call, #facade::value::Local>,
                            #failure_type,
                        > {
                            let identity = #facade::identity::MemberId::new(
                                #target_source, "method", #index, fragment_identity(),
                            );
                            let validated = invocation.validate(&identity, &[#(#parameter_expectations),*])?;
                            let (receiver, arguments) = validated.into_parts();
                            let mut arguments = arguments.into_vec().into_iter();
                            #(#argument_bindings)*
                            Ok(#output)
                        }

                        static #adapter_token_name: #adapter_type = #adapter_name;
                        static #descriptor_name: #facade::descriptor::InvocationAdapter = #constructor;
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
            let adapter_name = format_ident!("__qubit_reflect_invoke_{index}");
            let catching_adapter_name = format_ident!("__qubit_reflect_invoke_catching_{index}");
            let descriptor_name = format_ident!("__QUBIT_REFLECT_INVOCATION_ADAPTER_{index}");
            let thread_safe = method
                .attributes
                .iter()
                .any(|attribute| attribute.name == HelperName::ThreadSafe);
            let catching_supported = method
                .attributes
                .iter()
                .any(|attribute| attribute.name == HelperName::CatchUnwind)
                && !thread_safe
                && !method.qualifiers.is_async
                && !is_borrow_return(&method.return_type);
            let mode = if thread_safe {
                quote!(#facade::value::ThreadSafe)
            } else {
                quote!(#facade::value::Local)
            };
            let adapter_constructor = if thread_safe {
                quote!(#facade::descriptor::InvocationAdapter::thread_safe(#adapter_name))
            } else if catching_supported {
                quote!(#facade::descriptor::InvocationAdapter::local_with_catching(
                    #adapter_name,
                    #catching_adapter_name,
                ))
            } else {
                quote!(#facade::descriptor::InvocationAdapter::local(#adapter_name))
            };
            let adapter_definition = if catching_supported {
                quote! {
                    #[cfg(panic = "unwind")]
                    static #descriptor_name: #facade::descriptor::InvocationAdapter =
                        #adapter_constructor;
                    #[cfg(panic = "abort")]
                    static #descriptor_name: #facade::descriptor::InvocationAdapter =
                        #facade::descriptor::InvocationAdapter::local(#adapter_name);
                }
            } else {
                quote! {
                    static #descriptor_name: #facade::descriptor::InvocationAdapter =
                        #adapter_constructor;
                }
            };
            let receiver_expectation = if matches!(
                method.receiver.as_ref().map(|receiver| receiver.kind),
                Some(ReceiverKindIr::Value)
            ) {
                quote!(#facade::invoke::ReceiverExpectation::owned::<#target>())
            } else if let Some(receiver_type) = &typed_owned_receiver {
                quote!(#facade::invoke::ReceiverExpectation::owned::<#receiver_type>())
            } else if matches!(
                method.receiver.as_ref().map(|receiver| receiver.kind),
                Some(ReceiverKindIr::MutableReference)
            ) {
                quote!(#facade::invoke::ReceiverExpectation::borrowed_mut::<#target>())
            } else if method.receiver.is_some() {
                quote!(#facade::invoke::ReceiverExpectation::borrowed::<#target>())
            } else {
                quote!(#facade::invoke::ReceiverExpectation::none())
            };
            let receiver_binding = if let Some(receiver_type) = &typed_extension_receiver {
                quote! {
                    let (receiver, arguments) = validated.into_parts();
                    let receiver = match receiver {
                        Some(receiver) => receiver,
                        None => return Err(#facade::invoke::Invocation::associated(arguments).reject(
                            &identity,
                            #facade::invoke::InvocationErrorKind::ReceiverAdapterRejected {
                                expected_name: ::std::any::type_name::<#receiver_type>(),
                            },
                        )),
                    };
                    let adapter = match <#target as #facade::Reflect>::type_descriptor()
                        .get_capability(#facade::invoke::receiver_adapter_key::<#receiver_type, #mode>())
                    {
                        Some(adapter) => adapter,
                        None => return Err(#facade::invoke::Invocation::new(Some(receiver), arguments).reject(
                            &identity,
                            #facade::invoke::InvocationErrorKind::ReceiverAdapterUnavailable {
                                expected_name: ::std::any::type_name::<#receiver_type>(),
                            },
                        )),
                    };
                    let receiver: #receiver_type = match adapter(receiver) {
                        Ok(receiver) => receiver,
                        Err(receiver) => return Err(#facade::invoke::Invocation::new(Some(receiver), arguments).reject(
                            &identity,
                            #facade::invoke::InvocationErrorKind::ReceiverAdapterRejected {
                                expected_name: ::std::any::type_name::<#receiver_type>(),
                            },
                        )),
                    };
                }
            } else if matches!(
                method.receiver.as_ref().map(|receiver| receiver.kind),
                Some(ReceiverKindIr::Value)
            ) {
                quote! {
                    let (receiver, arguments) = validated.into_parts();
                    let receiver: #target = match receiver {
                        Some(#facade::invoke::InvocationReceiver::Owned(value)) =>
                            #facade::value::DynamicOwned::<#mode>::downcast::<#target>(value)
                                .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                        _ => unreachable!("validation checked receiver mode"),
                    };
                }
            } else if let Some(receiver_type) = &typed_owned_receiver {
                quote! {
                    let (receiver, arguments) = validated.into_parts();
                    let receiver: #receiver_type = match receiver {
                        Some(#facade::invoke::InvocationReceiver::Owned(value)) =>
                            #facade::value::DynamicOwned::<#mode>::downcast::<#receiver_type>(value)
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
                        Some(#facade::invoke::InvocationReceiver::Mut(value)) =>
                            #facade::value::DynamicMut::<#mode>::downcast::<#target>(value)
                                .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                        _ => unreachable!("validation checked receiver mode"),
                    };
                }
            } else if method.receiver.is_some() {
                quote! {
                    let (receiver, arguments) = validated.into_parts();
                    let receiver: &#target = match receiver {
                        Some(#facade::invoke::InvocationReceiver::Ref(value)) =>
                            #facade::value::DynamicRef::<#mode>::downcast::<#target>(value)
                                .unwrap_or_else(|_| unreachable!("validation checked receiver type")),
                        Some(#facade::invoke::InvocationReceiver::Mut(value)) => {
                            let value = #facade::value::DynamicMut::<#mode>::downcast::<#target>(value)
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
                .map(|parameter| invocation_argument_expectation(parameter, &facade))
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
                .map(|parameter| invocation_argument_binding(parameter, &facade, &mode))
                .collect();
            let call_arguments: Vec<_> = method
                .parameters
                .iter()
                .map(|parameter| format_ident!("__qubit_reflect_argument_{}", parameter.index))
                .collect();
            let argument_bindings = &argument_bindings;
            let parameter_expectations = &parameter_expectations;
            let call_arguments = &call_arguments;
            let call = if method.receiver.is_some() {
                quote!(<#target>::#method_name(receiver, #(#call_arguments),*))
            } else {
                quote!(<#target>::#method_name(#(#call_arguments),*))
            };
            let borrow_origins = std::iter::once(method.receiver.is_some().then(|| quote!(#facade::invoke::BorrowOrigin::Receiver)))
                .flatten()
                .chain(
                    method
                        .parameters
                        .iter()
                        .filter(|parameter| matches!(parameter.ty.kind, TypeKindIr::Reference { .. }))
                        .map(|parameter| {
                        let index = parameter.index;
                        quote!(#facade::invoke::BorrowOrigin::Parameter(#index))
                    }),
                );
            let output = match (method.qualifiers.is_async, &method.return_type) {
                (false, ReturnTypeIr::Unit) => quote! {
                    #receiver_binding
                    let mut arguments = arguments.into_vec().into_iter();
                    #(#argument_bindings)*
                    #call;
                    #facade::invoke::InvocationOutput::Unit
                },
                (false, ReturnTypeIr::Type(TypeIr { kind: TypeKindIr::Reference { mutable: false, .. }, .. })) => quote! {
                    #receiver_binding
                    let mut arguments = arguments.into_vec().into_iter();
                    #(#argument_bindings)*
                    #facade::invoke::InvocationOutput::Ref {
                        value: #facade::value::DynamicRef::<#mode>::new(#call),
                        origins: ::std::boxed::Box::new([#(#borrow_origins),*]),
                    }
                },
                (false, ReturnTypeIr::Type(TypeIr { kind: TypeKindIr::Reference { mutable: true, .. }, .. })) => quote! {
                    #receiver_binding
                    let mut arguments = arguments.into_vec().into_iter();
                    #(#argument_bindings)*
                    #facade::invoke::InvocationOutput::Mut {
                        value: #facade::value::DynamicMut::<#mode>::new(#call),
                        origin: #facade::invoke::BorrowOrigin::Receiver,
                    }
                },
                (false, ReturnTypeIr::Type(_)) => quote! {
                    #receiver_binding
                    let mut arguments = arguments.into_vec().into_iter();
                    #(#argument_bindings)*
                    #facade::invoke::InvocationOutput::Owned(
                        #facade::value::DynamicOwned::<#mode>::new(#call),
                    )
                },
                (true, ReturnTypeIr::Unit) => quote! {
                    #receiver_binding
                    let mut arguments = arguments.into_vec().into_iter();
                    #(#argument_bindings)*
                    #facade::invoke::InvocationOutput::Future(
                        #facade::invoke::ReflectedFuture::<#mode>::new(async move {
                            #call.await;
                            #facade::invoke::InvocationOutput::Unit
                        }),
                    )
                },
                (true, ReturnTypeIr::Type(_)) => quote! {
                    #receiver_binding
                    let mut arguments = arguments.into_vec().into_iter();
                    #(#argument_bindings)*
                    #facade::invoke::InvocationOutput::Future(
                        #facade::invoke::ReflectedFuture::<#mode>::new(async move {
                            #facade::invoke::InvocationOutput::Owned(
                                #facade::value::DynamicOwned::<#mode>::new(#call.await),
                            )
                        }),
                    )
                },
            };
            let catching_definition = if catching_supported {
                let catching_call = match method.return_type {
                    ReturnTypeIr::Unit => quote! {
                        #call;
                        #facade::invoke::InvocationOutput::Unit
                    },
                    ReturnTypeIr::Type(_) => quote! {
                        #facade::invoke::InvocationOutput::Owned(
                            #facade::value::DynamicOwned::<#facade::value::Local>::new(
                                #call,
                            ),
                        )
                    },
                };
                quote! {
                    fn #catching_adapter_name<'call>(
                        invocation: #facade::invoke::Invocation<'call, #facade::value::Local>,
                    ) -> ::core::result::Result<
                        ::core::result::Result<
                            #facade::invoke::InvocationOutput<'call, #facade::value::Local>,
                            #facade::invoke::InvocationPanic,
                        >,
                        #facade::invoke::InvocationFailure<'call, #facade::value::Local>,
                    > {
                        let identity = #facade::identity::MemberId::new(
                            #target_source, "method", #index, fragment_identity(),
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
                            Err(payload) => Ok(Err(#facade::invoke::InvocationPanic::new(identity, payload))),
                        }
                    }
                }
            } else {
                TokenStream::new()
            };
            Some(quote! {
                fn #adapter_name<'call>(
                    invocation: #facade::invoke::Invocation<'call, #mode>,
                ) -> ::core::result::Result<
                    #facade::invoke::InvocationOutput<'call, #mode>,
                    #facade::invoke::InvocationFailure<'call, #mode>,
                > {
                    let identity = #facade::identity::MemberId::new(
                        #target_source, "method", #index, fragment_identity(),
                    );
                    let validated = #invocation_validation;
                    Ok({ #output })
                }

                #catching_definition

                #adapter_definition
            })
        });
    let invocation_adapter_entries = declaration
        .methods
        .iter()
        .filter(|method| {
            !method
                .attributes
                .iter()
                .any(|attribute| attribute.name == HelperName::Skip)
        })
        .enumerate()
        .map(|(index, method)| {
            let supported_parameters = method
                .parameters
                .iter()
                .all(|parameter| supports_invocation_parameter(&parameter.ty));
            let typed_owned_receiver = method
                .receiver
                .as_ref()
                .and_then(|receiver| typed_owned_receiver_type(receiver, &target));
            let typed_pinned_receiver = method
                .receiver
                .as_ref()
                .and_then(typed_pinned_receiver_mutable);
            let typed_extension_receiver = method
                .receiver
                .as_ref()
                .and_then(|receiver| typed_extension_receiver_type(receiver, &target));
            if typed_pinned_receiver.is_some() {
                let is_safe_pinned_invocation = !has_trait
                    && supported_parameters
                    && method.generics.params.is_empty()
                    && !method.qualifiers.is_async
                    && !method.qualifiers.is_unsafe
                    && method.qualifiers.abi.is_none()
                    && !method.qualifiers.is_variadic
                    && !is_borrow_return(&method.return_type)
                    && !method
                        .attributes
                        .iter()
                        .any(|attribute| attribute.name == HelperName::NoInvoke)
                    && matches!(
                        method.return_type,
                        ReturnTypeIr::Unit
                            | ReturnTypeIr::Type(crate::ir::TypeIr {
                                kind: TypeKindIr::Path(_),
                                ..
                            })
                    );
                if is_safe_pinned_invocation {
                    let descriptor_name = format_ident!("__QUBIT_REFLECT_INVOCATION_ADAPTER_{index}");
                    return quote!(Some(&#descriptor_name));
                }
                return quote!(None);
            }
            let supported_receiver = matches!(
                method.receiver.as_ref().map(|receiver| receiver.kind),
                None | Some(ReceiverKindIr::Value)
                    | Some(ReceiverKindIr::SharedReference)
                    | Some(ReceiverKindIr::MutableReference)
            ) || typed_owned_receiver.is_some();
            let supported_receiver = supported_receiver || typed_extension_receiver.is_some();
            let is_safe_invocation = !has_trait
                && supported_receiver
                && supported_parameters
                && method.generics.params.is_empty()
                && !method.qualifiers.is_unsafe
                && method.qualifiers.abi.is_none()
                && !method.qualifiers.is_variadic
                && (!return_contains_non_static_lifetime(&method.return_type)
                    || is_supported_shared_borrow_return(&method.return_type)
                    || is_supported_mutable_borrow_return(method))
                && (!method.qualifiers.is_async || !is_borrow_return(&method.return_type))
                && !method
                    .attributes
                    .iter()
                    .any(|attribute| attribute.name == HelperName::NoInvoke)
                && matches!(
                    method.return_type,
                    ReturnTypeIr::Unit
                        | ReturnTypeIr::Type(crate::ir::TypeIr {
                            kind: TypeKindIr::Path(_) | TypeKindIr::Reference { .. },
                            ..
                        })
                );
            if is_safe_invocation {
                let descriptor_name = format_ident!("__QUBIT_REFLECT_INVOCATION_ADAPTER_{index}");
                if let Some(receiver_type) = typed_extension_receiver {
                    let mode = if method
                        .attributes
                        .iter()
                        .any(|attribute| attribute.name == HelperName::ThreadSafe)
                    {
                        quote!(#facade::value::ThreadSafe)
                    } else {
                        quote!(#facade::value::Local)
                    };
                    quote!(
                        if <#target as #facade::Reflect>::type_descriptor()
                            .get_capability(#facade::invoke::receiver_adapter_key::<#receiver_type, #mode>())
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
    let generic_specialization_adapter_definitions = declaration
        .methods
        .iter()
        .filter(|method| !method.attributes.iter().any(|attribute| attribute.name == HelperName::Skip))
        .enumerate()
        .flat_map(|(method_index, method)| {
            let target = target.clone();
            let target_source = target_source.clone();
            let facade = facade.clone();
            method.specializations.iter().enumerate().filter_map(move |(specialization_index, specialization)| {
                simple_generic_specialization_adapter(
                    method,
                    specialization,
                    &target,
                    &target_source,
                    &facade,
                    method_index,
                    specialization_index,
                )
            })
        });
    let method_specialization_entries = declaration
        .methods
        .iter()
        .filter(|method| !method.attributes.iter().any(|attribute| attribute.name == HelperName::Skip))
        .enumerate()
        .map(|(method_index, method)| {
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
        .filter(|method| !method.attributes.iter().any(|attribute| attribute.name == HelperName::Skip))
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
                ReceiverKindIr::Value => quote!(Some(#facade::descriptor::ReceiverDescriptor::Owned)),
                ReceiverKindIr::SharedReference => quote!(Some(#facade::descriptor::ReceiverDescriptor::Shared)),
                ReceiverKindIr::MutableReference => quote!(Some(#facade::descriptor::ReceiverDescriptor::Mutable)),
                ReceiverKindIr::Typed => {
                    let value = syn::LitStr::new(&receiver.declaration.to_string(), receiver.span);
                    quote!(Some(#facade::descriptor::ReceiverDescriptor::Explicit(#value)))
                }
            },
            None => quote!(None),
        };
        let parameters = method.parameters.iter().map(|parameter| {
            let name = parameter.name.as_deref().map(|name| syn::LitStr::new(name, parameter.span));
            let name = match name { Some(value) => quote!(Some(#value)), None => quote!(None) };
            let pattern = match parameter.pattern.kind {
                ParameterPatternKindIr::Identifier => quote!(#facade::descriptor::ParameterPatternDescriptor::Identifier),
                ParameterPatternKindIr::Wildcard => quote!(#facade::descriptor::ParameterPatternDescriptor::Wildcard),
                ParameterPatternKindIr::Destructure => {
                    let source = syn::LitStr::new(&parameter.pattern.source, parameter.span);
                    quote!(#facade::descriptor::ParameterPatternDescriptor::Destructure(#source.into()))
                }
            };
            let passing = match &parameter.ty.kind {
                TypeKindIr::Reference { mutable: true, .. } => quote!(#facade::descriptor::ParameterPassingMode::MutableBorrow),
                TypeKindIr::Reference { .. } => quote!(#facade::descriptor::ParameterPassingMode::SharedBorrow),
                _ => quote!(#facade::descriptor::ParameterPassingMode::Owned),
            };
            let ty = super::traits::type_expression(&parameter.ty, &facade);
            let parameter_index = parameter.index;
            quote!(#facade::descriptor::ParameterDescriptor::new(#parameter_index, #name, #pattern, #passing, #ty, None))
        });
        let return_value = match &method.return_type {
            ReturnTypeIr::Unit => quote!(#facade::descriptor::ReturnDescriptor::unit()),
            ReturnTypeIr::Type(ty) => {
                let expression = super::traits::type_expression(ty, &facade);
                let kind = match &ty.kind {
                    TypeKindIr::Never => quote!(#facade::descriptor::ReturnKind::Never),
                    TypeKindIr::Reference { .. } => quote!(#facade::descriptor::ReturnKind::Reference),
                    TypeKindIr::ImplTrait { .. } => quote!(#facade::descriptor::ReturnKind::Opaque),
                    _ => quote!(#facade::descriptor::ReturnKind::Concrete),
                };
                quote!(#facade::descriptor::ReturnDescriptor::new(#kind, Some(#expression), None))
            }
        };
        let visibility = match &method.visibility {
            VisibilityIr::Public => quote!(#facade::descriptor::MethodVisibility::Declared(#facade::identity::Visibility::Public)),
            VisibilityIr::Crate => quote!(#facade::descriptor::MethodVisibility::Declared(#facade::identity::Visibility::Crate)),
            VisibilityIr::Super => quote!(#facade::descriptor::MethodVisibility::Declared(#facade::identity::Visibility::Super)),
            VisibilityIr::Restricted(path) => { let path = syn::LitStr::new(&path.source, method.span); quote!(#facade::descriptor::MethodVisibility::Declared(#facade::identity::Visibility::Restricted(#path.into()))) },
            VisibilityIr::SelfValue | VisibilityIr::Inherited => quote!(#facade::descriptor::MethodVisibility::Declared(#facade::identity::Visibility::Private)),
        };
        let generic_definition = super::traits::generic_definition(&method.generics, method.span, &facade);
        let qualifiers = &method.qualifiers;
        let is_async = qualifiers.is_async;
        let is_unsafe = qualifiers.is_unsafe;
        let is_const = qualifiers.is_const;
        let is_variadic = qualifiers.is_variadic;
        let abi = qualifiers.abi.as_deref().map(|abi| syn::LitStr::new(abi, method.span));
        let abi = match abi { Some(value) => quote!(Some(#facade::expression::FunctionAbi::Other(#value.into()))), None => quote!(None) };
        quote! {
            #facade::descriptor::MethodDescriptor::builder(
                #facade::identity::MemberId::new(
                    #target_source,
                    "method",
                    #index,
                    fragment_identity(),
                ),
                #rust_name,
                #query_name,
                #facade::descriptor::MethodDeclarationOwner::Impl(definition),
            )
            .visibility(#visibility)
            .receiver(#receiver)
            .parameters(vec![#(#parameters),*])
            .return_value(#return_value)
            .qualifiers(#facade::descriptor::MethodQualifiers {
                is_async: #is_async, is_unsafe: #is_unsafe, is_const: #is_const,
                abi: #abi, is_variadic: #is_variadic,
            })
            .generic_definition(&#generic_definition)
            .build()
        }
    });
    let external_method_entries = declaration
        .methods
        .iter()
        .filter(|method| !method.attributes.iter().any(|attribute| attribute.name == HelperName::Skip))
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
                    ReceiverKindIr::Value => quote!(Some(#facade::descriptor::ReceiverDescriptor::Owned)),
                    ReceiverKindIr::SharedReference => quote!(Some(#facade::descriptor::ReceiverDescriptor::Shared)),
                    ReceiverKindIr::MutableReference => quote!(Some(#facade::descriptor::ReceiverDescriptor::Mutable)),
                    ReceiverKindIr::Typed => { let value = syn::LitStr::new(&receiver.declaration.to_string(), receiver.span); quote!(Some(#facade::descriptor::ReceiverDescriptor::Explicit(#value))) }
                },
                None => quote!(None),
            };
            let parameters = method.parameters.iter().map(|parameter| {
                let name = parameter.name.as_deref().map(|name| syn::LitStr::new(name, parameter.span));
                let name = match name { Some(value) => quote!(Some(#value)), None => quote!(None) };
                let pattern = match parameter.pattern.kind {
                    ParameterPatternKindIr::Identifier => quote!(#facade::descriptor::ParameterPatternDescriptor::Identifier),
                    ParameterPatternKindIr::Wildcard => quote!(#facade::descriptor::ParameterPatternDescriptor::Wildcard),
                    ParameterPatternKindIr::Destructure => { let source = syn::LitStr::new(&parameter.pattern.source, parameter.span); quote!(#facade::descriptor::ParameterPatternDescriptor::Destructure(#source.into())) }
                };
                let passing = match &parameter.ty.kind { TypeKindIr::Reference { mutable: true, .. } => quote!(#facade::descriptor::ParameterPassingMode::MutableBorrow), TypeKindIr::Reference { .. } => quote!(#facade::descriptor::ParameterPassingMode::SharedBorrow), _ => quote!(#facade::descriptor::ParameterPassingMode::Owned) };
                let ty = super::traits::type_expression(&parameter.ty, &facade);
                let parameter_index = parameter.index;
                quote!(#facade::descriptor::ParameterDescriptor::new(#parameter_index, #name, #pattern, #passing, #ty, None))
            });
            let return_value = match &method.return_type {
                ReturnTypeIr::Unit => quote!(#facade::descriptor::ReturnDescriptor::unit()),
                ReturnTypeIr::Type(ty) => { let expression = super::traits::type_expression(ty, &facade); let kind = match &ty.kind { TypeKindIr::Never => quote!(#facade::descriptor::ReturnKind::Never), TypeKindIr::Reference { .. } => quote!(#facade::descriptor::ReturnKind::Reference), TypeKindIr::ImplTrait { .. } => quote!(#facade::descriptor::ReturnKind::Opaque), _ => quote!(#facade::descriptor::ReturnKind::Concrete) }; quote!(#facade::descriptor::ReturnDescriptor::new(#kind, Some(#expression), None)) }
            };
            let generic_definition = super::traits::generic_definition(&method.generics, method.span, &facade);
            let qualifiers = &method.qualifiers;
            let is_async = qualifiers.is_async;
            let is_unsafe = qualifiers.is_unsafe;
            let is_const = qualifiers.is_const;
            let is_variadic = qualifiers.is_variadic;
            let abi = qualifiers.abi.as_deref().map(|abi| syn::LitStr::new(abi, method.span));
            let abi = match abi { Some(value) => quote!(Some(#facade::expression::FunctionAbi::Other(#value.into()))), None => quote!(None) };
            quote! {
                #facade::descriptor::MethodDescriptor::builder(
                    #facade::identity::MemberId::new(
                        #target_source, "external-method", #index, fragment_identity(),
                    ),
                    #rust_name, #query_name,
                    #facade::descriptor::MethodDeclarationOwner::Trait(external),
                )
                .visibility(#facade::descriptor::MethodVisibility::InheritedFromTrait)
                .receiver(#receiver)
                .parameters(vec![#(#parameters),*])
                .return_value(#return_value)
                .qualifiers(#facade::descriptor::MethodQualifiers { is_async: #is_async, is_unsafe: #is_unsafe, is_const: #is_const, abi: #abi, is_variadic: #is_variadic })
                .generic_definition(&#generic_definition)
                .build()
            }
        });
    let kind = if has_trait {
        quote!(#facade::descriptor::ImplKind::Trait)
    } else {
        quote!(#facade::descriptor::ImplKind::Inherent)
    };
    let trait_setup = match (trait_path, external_id) {
        (None, _) => quote!((None, None)),
        (Some(path), Some(id)) => quote!({
            let external: &'static #facade::descriptor::TraitDefinitionDescriptor =
                ::std::boxed::Box::leak(::std::boxed::Box::new(
                #facade::descriptor::TraitDefinitionDescriptor::new(
                    #facade::descriptor::TraitId::External(
                        #facade::identity::ExternalTraitId::new(#id)
                            .expect("validated external trait ID"),
                    ),
                    stringify!(#path),
                    stringify!(#path),
                    stringify!(#path),
                    #facade::descriptor::TraitCompleteness::ExternalIncomplete,
                    ::std::boxed::Box::leak(::std::boxed::Box::new(
                        #facade::expression::GenericDefinitionDescriptor {
                            parameters: ::std::boxed::Box::new([]),
                            predicates: ::std::boxed::Box::new([]),
                            diagnostic: ::core::default::Default::default(),
                        },
                    )),
                ),
            ));
            let applied: &'static #facade::descriptor::TraitDescriptor =
                ::std::boxed::Box::leak(::std::boxed::Box::new(
                {
                    let methods: &'static [#facade::descriptor::MethodDescriptor] =
                        ::std::boxed::Box::leak(::std::vec![#(#external_method_entries),*].into_boxed_slice());
                #facade::descriptor::TraitDescriptor::builder(external)
                    .methods(methods)
                    .build()
                    .expect("external trait application is valid")
                }
            ));
            (Some(external), Some(applied))
        }),
        (Some(path), None) => quote!({
            let payload = <#target as #path>::__qubit_reflect_trait_payload();
            (Some(payload.definition()), Some(payload.applied()))
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
                fn external_trait_runtime_identity() -> #facade::__private::registration::RuntimeIdentity {
                    #facade::__private::registration::RuntimeIdentity::Trait(
                        #facade::descriptor::TraitId::External(
                            #facade::identity::ExternalTraitId::new(#id)
                                .expect("validated external trait ID"),
                        ),
                    )
                }

                fn external_trait_payload() -> #facade::__private::registration::FragmentPayload {
                    static DESCRIPTOR: ::std::sync::OnceLock<#facade::descriptor::TraitDefinitionDescriptor> =
                        ::std::sync::OnceLock::new();
                    let descriptor = DESCRIPTOR.get_or_init(|| #facade::descriptor::TraitDefinitionDescriptor::new(
                        #facade::descriptor::TraitId::External(
                            #facade::identity::ExternalTraitId::new(#id)
                                .expect("validated external trait ID"),
                        ),
                        stringify!(#path), stringify!(#path), stringify!(#path),
                        #facade::descriptor::TraitCompleteness::ExternalIncomplete,
                        ::std::boxed::Box::leak(::std::boxed::Box::new(
                            #facade::expression::GenericDefinitionDescriptor {
                                parameters: ::std::boxed::Box::new([]),
                                predicates: ::std::boxed::Box::new([]),
                                diagnostic: ::core::default::Default::default(),
                            },
                        )),
                    ));
                    #facade::__private::registration::FragmentPayload::Trait(descriptor)
                }

                #facade::__private::inventory::submit! {
                    #facade::__private::registration::RegistrationFragment::new(
                        #facade::__private::registration::FragmentKind::Trait,
                        #facade::__private::registration::StaticFragmentIdentity::new(
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
    quote! {
        #retained

        #[doc(hidden)]
        mod #module {
            use super::*;

            #(#invocation_adapter_definitions)*
            #(#generic_specialization_adapter_definitions)*

            fn fragment_identity() -> #facade::identity::FragmentIdentity {
                #facade::identity::FragmentIdentity::new(
                    env!("CARGO_PKG_NAME"), module_path!(), #line, #column,
                    "impl", #fingerprint,
                )
            }

            fn runtime_identity() -> #facade::__private::registration::RuntimeIdentity {
                #facade::__private::registration::RuntimeIdentity::Impl(
                    ::std::any::TypeId::of::<#target>(),
                )
            }

            fn payload() -> #facade::__private::registration::FragmentPayload {
                static DESCRIPTOR: ::std::sync::OnceLock<#facade::descriptor::ImplDescriptor> =
                    ::std::sync::OnceLock::new();
                let descriptor = DESCRIPTOR.get_or_init(|| {
                    let (trait_definition, implemented_trait): (
                        Option<&'static #facade::descriptor::TraitDefinitionDescriptor>,
                        Option<&'static #facade::descriptor::TraitDescriptor>,
                    ) = #trait_setup;
                    let definition = ::std::boxed::Box::leak(::std::boxed::Box::new(
                        #facade::descriptor::ImplDefinitionDescriptor::new(
                            fragment_identity(),
                            #facade::expression::TypeExpression::Concrete(
                                #facade::expression::ConcreteTypeExpression {
                                    path: vec![#target_source.into()].into_boxed_slice(),
                                    arguments: ::std::boxed::Box::new([]),
                                    diagnostic: #facade::expression::DiagnosticText::from(#target_source),
                                },
                            ),
                            #kind,
                            #trait_definition,
                            ::std::boxed::Box::leak(::std::boxed::Box::new(
                                #impl_generic_definition,
                            )),
                        ).expect("generated impl definition is consistent"),
                    ));
                    let methods: &'static [#facade::descriptor::MethodDescriptor] =
                        ::std::boxed::Box::leak(::std::vec![#(#method_entries),*].into_boxed_slice());
                    let method_instances = if let Some(trait_descriptor) = implemented_trait {
                        trait_descriptor.methods().iter().map(|declaration| {
                            match methods.iter().find(|method| method.query_name() == declaration.query_name()) {
                                Some(method) => #facade::descriptor::MethodInstanceDescriptor::new(
                                    declaration, Some(method),
                                    #facade::descriptor::MethodImplementationSource::Overridden,
                                    None,
                                    ::std::boxed::Box::new([
                                        #facade::descriptor::InvocationUnavailableReason::DisabledByPolicy,
                                    ]),
                                ).expect("generated trait method instance is consistent"),
                                None if declaration.has_default() => #facade::descriptor::MethodInstanceDescriptor::new(
                                    declaration, None,
                                    #facade::descriptor::MethodImplementationSource::Defaulted,
                                    None,
                                    ::std::boxed::Box::new([
                                        #facade::descriptor::InvocationUnavailableReason::DisabledByPolicy,
                                    ]),
                                ).expect("generated default trait method instance is consistent"),
                                None => #facade::descriptor::MethodInstanceDescriptor::new(
                                    declaration, None,
                                    #facade::descriptor::MethodImplementationSource::Required,
                                    None,
                                    ::std::boxed::Box::new([
                                        #facade::descriptor::InvocationUnavailableReason::DisabledByPolicy,
                                    ]),
                                ).expect("generated required trait method instance is consistent"),
                            }
                        }).collect()
                    } else {
                        let adapters: &[Option<&'static #facade::descriptor::InvocationAdapter>] =
                            &[#(#invocation_adapter_entries),*];
                        let specializations: ::std::vec::Vec<::std::vec::Vec<(
                            ::std::boxed::Box<[#facade::expression::GenericArgument]>,
                            Option<&'static #facade::descriptor::InvocationAdapter>,
                        )>> = ::std::vec![#(#method_specialization_entries),*];
                        methods.iter().zip(adapters.iter().copied()).enumerate().flat_map(|(index, (method, adapter))| {
                            if !specializations[index].is_empty() {
                                return specializations[index].iter().cloned().map(|(arguments, adapter)| {
                                    let unavailable_reasons: ::std::boxed::Box<[
                                        #facade::descriptor::InvocationUnavailableReason
                                    ]> = if adapter.is_some() {
                                        ::std::vec![].into_boxed_slice()
                                    } else {
                                        ::std::vec![
                                            #facade::descriptor::InvocationUnavailableReason::UnsupportedSpecialization,
                                        ].into_boxed_slice()
                                    };
                                    #facade::descriptor::MethodInstanceDescriptor::with_arguments(
                                        method,
                                        None,
                                        #facade::descriptor::MethodImplementationSource::Declared,
                                        adapter,
                                        arguments,
                                        unavailable_reasons,
                                    ).expect("generated generic method specialization is consistent")
                                }).collect::<::std::vec::Vec<_>>();
                            }
                            let unavailable_reasons: ::std::boxed::Box<[
                                #facade::descriptor::InvocationUnavailableReason
                            ]> = if adapter.is_some() {
                                ::std::vec![] .into_boxed_slice()
                            } else {
                                let reason = if method.qualifiers().is_unsafe {
                                    #facade::descriptor::InvocationUnavailableReason::UnsafeMethod
                                } else if method.qualifiers().abi.is_some() {
                                    #facade::descriptor::InvocationUnavailableReason::UnsupportedAbi
                                } else if method.qualifiers().is_variadic {
                                    #facade::descriptor::InvocationUnavailableReason::Variadic
                                } else if matches!(method.receiver(), Some(#facade::descriptor::ReceiverDescriptor::Explicit(_))) {
                                    #facade::descriptor::InvocationUnavailableReason::UnsupportedReceiver
                                } else if !method.generic_definition().parameters.is_empty() {
                                    #facade::descriptor::InvocationUnavailableReason::UnspecializedGeneric
                                } else {
                                    #facade::descriptor::InvocationUnavailableReason::DisabledByPolicy
                                };
                                ::std::vec![reason].into_boxed_slice()
                            };
                            ::std::vec![#facade::descriptor::MethodInstanceDescriptor::new(
                                method,
                                None,
                                #facade::descriptor::MethodImplementationSource::Declared,
                                adapter,
                                unavailable_reasons,
                            ).expect("generated inherent method instance is consistent")]
                        }).collect::<::std::vec::Vec<_>>()
                    };
                    let mut builder = #facade::descriptor::ImplDescriptor::builder(
                        definition,
                        || <#target as #facade::Reflect>::type_descriptor(),
                    ).methods(methods).method_instances(method_instances).arguments(#impl_arguments);
                    if let Some(trait_descriptor) = #implemented_trait {
                        builder = builder.implemented_trait(trait_descriptor);
                    }
                    builder.build().expect("generated impl descriptor is consistent")
                });
                #facade::__private::registration::FragmentPayload::Impl(descriptor)
            }

            #facade::__private::inventory::submit! {
                #facade::__private::registration::RegistrationFragment::new(
                    #facade::__private::registration::FragmentKind::Impl,
                    #facade::__private::registration::StaticFragmentIdentity::new(
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

/// Builds replacement tokens for one validated type or const specialization.
fn specialization_replacements(specialization: &SpecializationIr) -> Vec<(Ident, TokenStream)> {
    specialization
        .bindings
        .iter()
        .map(|binding| {
            let tokens = match &binding.value {
                SpecializationValueIr::Type(ty) => ty.tokens.clone(),
                SpecializationValueIr::Const(tokens) | SpecializationValueIr::AmbiguousPath(tokens) => tokens.clone(),
            };
            (Ident::new(&binding.name, binding.span), tokens)
        })
        .collect()
}

/// Replaces generic identifiers recursively while retaining source grouping.
fn substitute_tokens(tokens: &TokenStream, replacements: &[(Ident, TokenStream)]) -> TokenStream {
    tokens
        .clone()
        .into_iter()
        .flat_map(|tree| match tree {
            TokenTree::Ident(ident) => replacements
                .iter()
                .find(|(name, _)| *name == ident)
                .map(|(_, tokens)| tokens.clone())
                .unwrap_or_else(|| TokenStream::from(TokenTree::Ident(ident))),
            TokenTree::Group(group) => {
                let mut replacement = proc_macro2::Group::new(
                    group.delimiter(),
                    substitute_tokens(&group.stream(), replacements),
                );
                replacement.set_span(group.span());
                TokenStream::from(TokenTree::Group(replacement))
            }
            other => TokenStream::from(other),
        })
        .collect()
}

/// Replaces impl generic references in method signature types while retaining
/// their structural IR for descriptor rendering.
fn substitute_impl_method_types(declaration: &mut ImplDeclarationIr, replacements: &[(Ident, TokenStream)]) {
    for method in &mut declaration.methods {
        if let Some(receiver) = &mut method.receiver {
            substitute_type_tokens(&mut receiver.ty, replacements);
        }
        for parameter in &mut method.parameters {
            substitute_type_tokens(&mut parameter.ty, replacements);
        }
        if let ReturnTypeIr::Type(ty) = &mut method.return_type {
            substitute_type_tokens(ty, replacements);
        }
    }
}

/// Recursively substitutes one type and every nested type retained by its IR.
fn substitute_type_tokens(ty: &mut TypeIr, replacements: &[(Ident, TokenStream)]) {
    ty.tokens = substitute_tokens(&ty.tokens, replacements);
    ty.source = ty.tokens.to_string();
    match &mut ty.kind {
        TypeKindIr::Path(path) => substitute_path_tokens(path, replacements),
        TypeKindIr::Reference { element, .. }
        | TypeKindIr::Slice(element)
        | TypeKindIr::Pointer { element, .. } => substitute_type_tokens(element, replacements),
        TypeKindIr::Tuple(elements) => {
            for element in elements {
                substitute_type_tokens(element, replacements);
            }
        }
        TypeKindIr::Array { element, length } => {
            substitute_type_tokens(element, replacements);
            *length = substitute_tokens(length, replacements);
        }
        TypeKindIr::BareFunction { inputs, output, .. } => {
            for input in inputs {
                substitute_type_tokens(input, replacements);
            }
            if let Some(output) = output {
                substitute_type_tokens(output, replacements);
            }
        }
        TypeKindIr::TraitObject { .. }
        | TypeKindIr::ImplTrait { .. }
        | TypeKindIr::Never
        | TypeKindIr::Infer
        | TypeKindIr::Macro
        | TypeKindIr::Other => {}
    }
}

/// Substitutes nested type and const tokens in one path IR.
fn substitute_path_tokens(path: &mut crate::ir::PathIr, replacements: &[(Ident, TokenStream)]) {
    path.tokens = substitute_tokens(&path.tokens, replacements);
    path.source = path.tokens.to_string();
    if let Some(qualified_self) = &mut path.qualified_self {
        substitute_type_tokens(&mut qualified_self.ty, replacements);
    }
    for segment in &mut path.segments {
        match &mut segment.arguments {
            PathArgumentsIr::None => {}
            PathArgumentsIr::AngleBracketed(arguments) => {
                for argument in arguments {
                    match argument {
                        PathArgumentIr::Type(ty) | PathArgumentIr::AssociatedType { ty, .. } => {
                            substitute_type_tokens(ty, replacements);
                        }
                        PathArgumentIr::Const(tokens)
                        | PathArgumentIr::AssociatedConst { value: tokens, .. } => {
                            *tokens = substitute_tokens(tokens, replacements);
                        }
                        PathArgumentIr::Lifetime(_) | PathArgumentIr::Constraint { .. } | PathArgumentIr::Other(_) => {}
                    }
                }
            }
            PathArgumentsIr::Parenthesized { inputs, output } => {
                for input in inputs {
                    substitute_type_tokens(input, replacements);
                }
                if let Some(output) = output {
                    substitute_type_tokens(output, replacements);
                }
            }
        }
    }
}

/// Returns whether a parameter can cross the safe dynamic invocation boundary.
fn supports_invocation_parameter(ty: &TypeIr) -> bool {
    match &ty.kind {
        TypeKindIr::Reference { element, .. } => supports_owned_dynamic_type(element),
        _ => supports_owned_dynamic_type(ty),
    }
}

/// Returns the exact owned container type for the standard explicit receivers
/// that can cross the `Any` boundary without losing ownership or pinning.
fn typed_owned_receiver_type(receiver: &crate::ir::ReceiverIr, target: &TokenStream) -> Option<TokenStream> {
    if receiver.kind != ReceiverKindIr::Typed {
        return None;
    }
    let TypeKindIr::Path(path) = &receiver.ty.kind else {
        return None;
    };
    let segment = path.segments.last()?;
    let PathArgumentsIr::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    match (segment.name.as_str(), arguments.as_slice()) {
        ("Box", [PathArgumentIr::Type(argument)]) if is_self_type(argument) => {
            Some(quote!(::std::boxed::Box<#target>))
        }
        ("Rc", [PathArgumentIr::Type(argument)]) if is_self_type(argument) => {
            Some(quote!(::std::rc::Rc<#target>))
        }
        ("Arc", [PathArgumentIr::Type(argument)]) if is_self_type(argument) => {
            Some(quote!(::std::sync::Arc<#target>))
        }
        ("Pin", [PathArgumentIr::Type(argument)]) if is_box_self_type(argument) => {
            Some(quote!(::std::pin::Pin<::std::boxed::Box<#target>>))
        }
        _ => None,
    }
}

/// Returns a non-core explicit receiver type that requires a registered
/// `ReceiverAdapter` capability for dynamic invocation.
fn typed_extension_receiver_type(
    receiver: &crate::ir::ReceiverIr,
    target: &TokenStream,
) -> Option<TokenStream> {
    if receiver.kind != ReceiverKindIr::Typed
        || typed_pinned_receiver_mutable(receiver).is_some()
        || typed_owned_receiver_type(receiver, &quote!(Self)).is_some()
    {
        return None;
    }
    let self_identifier = Ident::new("Self", receiver.span);
    Some(substitute_tokens(
        &receiver.ty.tokens,
        &[(self_identifier, target.clone())],
    ))
}

/// Returns whether a typed receiver is `Pin<&mut Self>` (`true`) or
/// `Pin<&Self>` (`false`).
fn typed_pinned_receiver_mutable(receiver: &crate::ir::ReceiverIr) -> Option<bool> {
    if receiver.kind != ReceiverKindIr::Typed {
        return None;
    }
    let TypeKindIr::Path(path) = &receiver.ty.kind else {
        return None;
    };
    let segment = path.segments.last()?;
    let PathArgumentsIr::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let [PathArgumentIr::Type(argument)] = arguments.as_slice() else {
        return None;
    };
    let TypeKindIr::Reference { mutable, element, .. } = &argument.kind else {
        return None;
    };
    (segment.name == "Pin" && is_self_type(element)).then_some(*mutable)
}

/// Returns whether `ty` is the receiver's unqualified `Self` type.
fn is_self_type(ty: &TypeIr) -> bool {
    matches!(&ty.kind, TypeKindIr::Path(path) if path.segments.len() == 1 && path.segments[0].name == "Self")
}

/// Returns whether `ty` is the standard `Box<Self>` receiver container.
fn is_box_self_type(ty: &TypeIr) -> bool {
    let TypeKindIr::Path(path) = &ty.kind else {
        return false;
    };
    let Some(segment) = path.segments.last() else {
        return false;
    };
    matches!(
        (&*segment.name, &segment.arguments),
        ("Box", PathArgumentsIr::AngleBracketed(arguments))
            if matches!(arguments.as_slice(), [PathArgumentIr::Type(argument)] if is_self_type(argument))
    )
}

/// Emits concrete arguments for one validated method specialization.
fn specialization_arguments(
    specialization: &SpecializationIr,
    generics: &crate::ir::GenericsIr,
    facade: &TokenStream,
) -> TokenStream {
    let arguments = generics.params.iter().filter_map(|parameter| {
        if parameter.kind == GenericKindIr::Lifetime {
            return None;
        }
        let binding = specialization.bindings.iter().find(|binding| binding.name == parameter.name)?;
        match (parameter.kind, &binding.value) {
            (GenericKindIr::Type, SpecializationValueIr::Type(ty)) => {
                let expression = super::traits::type_expression(ty, facade);
                Some(quote!(#facade::expression::GenericArgument::Type(#expression)))
            }
            (GenericKindIr::Type, SpecializationValueIr::AmbiguousPath(tokens)) => Some(quote!(
                #facade::expression::GenericArgument::Type(
                    #facade::expression::TypeExpression::Concrete(
                        #facade::expression::ConcreteTypeExpression {
                            path: ::std::boxed::Box::new([stringify!(#tokens).into()]),
                            arguments: ::std::boxed::Box::new([]),
                            diagnostic: #facade::expression::DiagnosticText::from(stringify!(#tokens)),
                        },
                    ),
                )
            )),
            (GenericKindIr::Const, value) => {
                let tokens = match value {
                    SpecializationValueIr::Const(tokens) | SpecializationValueIr::AmbiguousPath(tokens) => tokens,
                    SpecializationValueIr::Type(_) => return None,
                };
                let declared_type = parameter.const_type.as_ref()?.source.as_str();
                let declared_type_literal = syn::LitStr::new(declared_type, parameter.span);
                Some(quote!(
                    #facade::expression::GenericArgument::Const(
                        #facade::expression::ConstGenericArgument {
                            declared_type: ::std::boxed::Box::new(
                                #facade::expression::TypeExpression::Concrete(
                                    #facade::expression::ConcreteTypeExpression {
                                        path: ::std::boxed::Box::new([#declared_type_literal.into()]),
                                        arguments: ::std::boxed::Box::new([]),
                                        diagnostic: #facade::expression::DiagnosticText::from(#declared_type_literal),
                                    },
                                ),
                            ),
                            value: #facade::expression::ConstExpression::Path(
                                ::std::boxed::Box::new([stringify!(#tokens).into()]),
                            ),
                            normalized_diagnostic: stringify!(#tokens).into(),
                        },
                    )
                ))
            }
            _ => None,
        }
    });
    quote!(::std::vec![#(#arguments),*].into_boxed_slice())
}

/// Generates a local adapter for the safely erasable subset of an explicitly
/// registered generic associated function. More complex signatures remain
/// registered as specializations but explicitly unavailable.
fn simple_generic_specialization_adapter(
    method: &MethodIr,
    specialization: &SpecializationIr,
    target: &TokenStream,
    target_source: &str,
    facade: &TokenStream,
    method_index: usize,
    specialization_index: usize,
) -> Option<TokenStream> {
    if method.receiver.is_some()
        || method.qualifiers.is_async
        || method.qualifiers.is_unsafe
        || method.qualifiers.abi.is_some()
        || method.qualifiers.is_variadic
        || method.attributes.iter().any(|attribute| attribute.name == HelperName::NoInvoke)
        || method.generics.params.iter().any(|parameter| parameter.kind == GenericKindIr::Lifetime)
    {
        return None;
    }
    let generic_arguments: Vec<_> = method
        .generics
        .params
        .iter()
        .map(|parameter| match parameter.kind {
            GenericKindIr::Type => specialization_type_argument(specialization, &parameter.name),
            GenericKindIr::Const => specialization_const_argument(specialization, &parameter.name),
            GenericKindIr::Lifetime => None,
        })
        .collect::<Option<_>>()?;
    let parameter_types: Vec<_> = method
        .parameters
        .iter()
        .map(|parameter| specialize_direct_type_parameter(&parameter.ty, &method.generics, specialization))
        .collect::<Option<_>>()?;
    let return_type = match &method.return_type {
        ReturnTypeIr::Unit => None,
        ReturnTypeIr::Type(ty) => Some(specialize_direct_type_parameter(ty, &method.generics, specialization)?),
    };
    let argument_expectations = parameter_types
        .iter()
        .map(|ty| quote!(#facade::invoke::ArgumentExpectation::owned::<#ty>()));
    let argument_bindings = parameter_types.iter().enumerate().map(|(index, ty)| {
        let argument = format_ident!("__qubit_reflect_specialized_argument_{index}");
        quote! {
            let #argument: #ty = match arguments.next().expect("validation checked argument count") {
                #facade::invoke::InvocationArg::Owned(value) =>
                    #facade::value::DynamicOwned::<#facade::value::Local>::downcast::<#ty>(value)
                        .unwrap_or_else(|_| unreachable!("validation checked argument type")),
                _ => unreachable!("validation checked argument mode"),
            };
        }
    });
    let call_arguments = (0..parameter_types.len()).map(|index| format_ident!("__qubit_reflect_specialized_argument_{index}"));
    let method_name = &method.name;
    let adapter_name = format_ident!("__qubit_reflect_invoke_specialization_{method_index}_{specialization_index}");
    let descriptor_name = format_ident!(
        "__QUBIT_REFLECT_GENERIC_SPECIALIZATION_ADAPTER_{method_index}_{specialization_index}"
    );
    let output = match return_type {
        None => quote! {
            <#target>::#method_name::<#(#generic_arguments),*>(#(#call_arguments),*);
            #facade::invoke::InvocationOutput::Unit
        },
        Some(_) => quote! {
            #facade::invoke::InvocationOutput::Owned(
                #facade::value::DynamicOwned::<#facade::value::Local>::new(
                    <#target>::#method_name::<#(#generic_arguments),*>(#(#call_arguments),*),
                ),
            )
        },
    };
    Some(quote! {
        fn #adapter_name<'call>(
            invocation: #facade::invoke::Invocation<'call, #facade::value::Local>,
        ) -> ::core::result::Result<
            #facade::invoke::InvocationOutput<'call, #facade::value::Local>,
            #facade::invoke::InvocationFailure<'call, #facade::value::Local>,
        > {
            let identity = #facade::identity::MemberId::new(
                #target_source,
                "method-specialization",
                #method_index,
                fragment_identity(),
            );
            let validated = invocation.validate(
                &identity,
                #facade::invoke::ReceiverExpectation::none(),
                &[#(#argument_expectations),*],
            )?;
            let (_receiver, arguments) = validated.into_parts();
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            Ok(#output)
        }

        static #descriptor_name: #facade::descriptor::InvocationAdapter =
            #facade::descriptor::InvocationAdapter::local(#adapter_name);
    })
}

/// Resolves one named type argument from a validated specialization.
fn specialization_type_argument(specialization: &SpecializationIr, name: &str) -> Option<TokenStream> {
    match &specialization.bindings.iter().find(|binding| binding.name == name)?.value {
        SpecializationValueIr::Type(ty) => Some(ty.tokens.clone()),
        SpecializationValueIr::AmbiguousPath(tokens) => Some(tokens.clone()),
        SpecializationValueIr::Const(_) => None,
    }
}

/// Resolves one named const argument from a validated specialization.
fn specialization_const_argument(specialization: &SpecializationIr, name: &str) -> Option<TokenStream> {
    match &specialization.bindings.iter().find(|binding| binding.name == name)?.value {
        SpecializationValueIr::Const(tokens) | SpecializationValueIr::AmbiguousPath(tokens) => Some(tokens.clone()),
        SpecializationValueIr::Type(_) => None,
    }
}

/// Replaces a direct generic type parameter with its registered concrete type.
fn specialize_direct_type_parameter(
    ty: &TypeIr,
    generics: &crate::ir::GenericsIr,
    specialization: &SpecializationIr,
) -> Option<TokenStream> {
    let TypeKindIr::Path(path) = &ty.kind else {
        return None;
    };
    if path.segments.len() != 1 || !matches!(path.segments[0].arguments, PathArgumentsIr::None) {
        return Some(ty.tokens.clone());
    }
    let name = &path.segments[0].name;
    if generics.params.iter().any(|parameter| parameter.kind == GenericKindIr::Type && parameter.name == *name) {
        specialization_type_argument(specialization, name)
    } else {
        Some(ty.tokens.clone())
    }
}

/// Returns whether an owned dynamic value can safely represent `ty`.
fn supports_owned_dynamic_type(ty: &TypeIr) -> bool {
    matches!(
        ty.kind,
        TypeKindIr::Path(_)
            | TypeKindIr::Tuple(_)
            | TypeKindIr::Array { .. }
            | TypeKindIr::Pointer { .. }
            | TypeKindIr::BareFunction { .. }
    )
}

/// Expands the exact runtime expectation for one positional parameter.
fn invocation_argument_expectation(parameter: &ParameterIr, facade: &TokenStream) -> TokenStream {
    match &parameter.ty.kind {
        TypeKindIr::Reference {
            mutable: true, element, ..
        } => {
            let element = &element.tokens;
            quote!(#facade::invoke::ArgumentExpectation::borrowed_mut::<#element>())
        }
        TypeKindIr::Reference { element, .. } => {
            let element = &element.tokens;
            quote!(#facade::invoke::ArgumentExpectation::borrowed::<#element>())
        }
        _ => {
            let ty = &parameter.ty.tokens;
            quote!(#facade::invoke::ArgumentExpectation::owned::<#ty>())
        }
    }
}

/// Expands one post-validation extraction while preserving borrowed lifetimes.
fn invocation_argument_binding(parameter: &ParameterIr, facade: &TokenStream, mode: &TokenStream) -> TokenStream {
    let argument = format_ident!("__qubit_reflect_argument_{}", parameter.index);
    match &parameter.ty.kind {
        TypeKindIr::Reference {
            mutable: true, element, ..
        } => {
            let element = &element.tokens;
            quote! {
                let #argument = match arguments
                    .next()
                    .expect("validation checked argument count")
                {
                    #facade::invoke::InvocationArg::Mut(value) =>
                        #facade::value::DynamicMut::<#mode>::downcast::<#element>(value)
                            .unwrap_or_else(|_| unreachable!("validation checked argument type")),
                    _ => unreachable!("validation checked argument mode"),
                };
            }
        }
        TypeKindIr::Reference { element, .. } => {
            let element = &element.tokens;
            quote! {
                let #argument = match arguments
                    .next()
                    .expect("validation checked argument count")
                {
                    #facade::invoke::InvocationArg::Ref(value) =>
                        #facade::value::DynamicRef::<#mode>::downcast::<#element>(value)
                            .unwrap_or_else(|_| unreachable!("validation checked argument type")),
                    #facade::invoke::InvocationArg::Mut(value) => {
                        let value = #facade::value::DynamicMut::<#mode>::downcast::<#element>(value)
                            .unwrap_or_else(|_| unreachable!("validation checked argument type"));
                        &*value
                    }
                    _ => unreachable!("validation checked argument mode"),
                };
            }
        }
        _ => {
            let ty = &parameter.ty.tokens;
            quote! {
                let #argument: #ty = match arguments
                    .next()
                    .expect("validation checked argument count")
                {
                    #facade::invoke::InvocationArg::Owned(value) =>
                        #facade::value::DynamicOwned::<#mode>::downcast::<#ty>(value)
                            .unwrap_or_else(|_| unreachable!("validation checked argument type")),
                    _ => unreachable!("validation checked argument mode"),
                };
            }
        }
    }
}

/// Resolves the facade path without coupling the proc-macro crate to runtime.
fn facade_path() -> Option<TokenStream> {
    match proc_macro_crate::crate_name("qubit-reflect") {
        Ok(proc_macro_crate::FoundCrate::Itself) => Some(quote!(crate)),
        Ok(proc_macro_crate::FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            Some(quote!(::#ident))
        }
        Err(_) => None,
    }
}

/// Computes a deterministic content fingerprint for fragment identity.
fn fingerprint(value: &str) -> u64 {
    value.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

/// Returns whether a return declaration carries a borrow that cannot cross the
/// owned dynamic-value boundary.
fn return_contains_non_static_lifetime(return_type: &ReturnTypeIr) -> bool {
    matches!(return_type, ReturnTypeIr::Type(ty) if type_contains_non_static_lifetime(ty))
}

/// Returns whether a borrowed output can retain the invocation call lifetime.
fn is_supported_shared_borrow_return(return_type: &ReturnTypeIr) -> bool {
    matches!(
        return_type,
        ReturnTypeIr::Type(TypeIr {
            kind: TypeKindIr::Reference { mutable: false, .. },
            ..
        })
    )
}

/// Returns whether this declaration can safely identify a unique mutable-borrow origin.
fn is_supported_mutable_borrow_return(method: &MethodIr) -> bool {
    matches!(
        method.receiver.as_ref().map(|receiver| receiver.kind),
        Some(ReceiverKindIr::MutableReference)
    ) && !method
        .parameters
        .iter()
        .any(|parameter| matches!(parameter.ty.kind, TypeKindIr::Reference { mutable: true, .. }))
        && matches!(
            method.return_type,
            ReturnTypeIr::Type(TypeIr {
                kind: TypeKindIr::Reference { mutable: true, .. },
                ..
            })
        )
}

/// Returns whether the declaration returns any borrow rather than an owned value.
fn is_borrow_return(return_type: &ReturnTypeIr) -> bool {
    matches!(
        return_type,
        ReturnTypeIr::Type(TypeIr {
            kind: TypeKindIr::Reference { .. },
            ..
        })
    )
}

/// Recursively detects path arguments whose lifetime is not explicitly static.
fn type_contains_non_static_lifetime(ty: &TypeIr) -> bool {
    match &ty.kind {
        TypeKindIr::Path(path) => {
            path.qualified_self
                .as_ref()
                .is_some_and(|qualified| type_contains_non_static_lifetime(&qualified.ty))
                || path.segments.iter().any(|segment| match &segment.arguments {
                    PathArgumentsIr::None => false,
                    PathArgumentsIr::AngleBracketed(arguments) => arguments.iter().any(|argument| match argument {
                        PathArgumentIr::Lifetime(lifetime) => lifetime != "'static",
                        PathArgumentIr::Type(ty) => type_contains_non_static_lifetime(ty),
                        PathArgumentIr::AssociatedType { ty, .. } => type_contains_non_static_lifetime(ty),
                        PathArgumentIr::Const(_)
                        | PathArgumentIr::AssociatedConst { .. }
                        | PathArgumentIr::Constraint { .. }
                        | PathArgumentIr::Other(_) => false,
                    }),
                    PathArgumentsIr::Parenthesized { inputs, output } => {
                        inputs.iter().any(type_contains_non_static_lifetime)
                            || output.as_deref().is_some_and(type_contains_non_static_lifetime)
                    }
                })
        }
        TypeKindIr::Reference { .. } => true,
        TypeKindIr::Tuple(items) => items.iter().any(type_contains_non_static_lifetime),
        TypeKindIr::Slice(element) | TypeKindIr::Array { element, .. } | TypeKindIr::Pointer { element, .. } => {
            type_contains_non_static_lifetime(element)
        }
        TypeKindIr::BareFunction {
            lifetimes,
            inputs,
            output,
            ..
        } => {
            lifetimes.iter().any(|lifetime| lifetime != "'static")
                || inputs.iter().any(type_contains_non_static_lifetime)
                || output.as_deref().is_some_and(type_contains_non_static_lifetime)
        }
        TypeKindIr::TraitObject { .. }
        | TypeKindIr::ImplTrait { .. }
        | TypeKindIr::Never
        | TypeKindIr::Infer
        | TypeKindIr::Macro
        | TypeKindIr::Other => false,
    }
}
