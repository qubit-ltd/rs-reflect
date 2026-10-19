//! Call-site binding of one explicitly monomorphized method.

use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::format_ident;
use quote::quote;
use syn::ExprPath;
use syn::Result;
use syn::Token;
use syn::Type;
use syn::TypeFnPtr;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse2;
use syn::spanned::Spanned;

use crate::expand::context::ExpansionContext;
use crate::expand::expression_codegen::path_arguments;
use crate::expand::expression_codegen::path_type_expression;
use crate::expand::generic_environment::GenericEnvironment;

struct BindingInput {
    method: ExprPath,
    signature: TypeFnPtr,
    asynchronous: bool,
}

impl Parse for BindingInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let method = input.parse()?;
        input.parse::<Token![as]>()?;
        let asynchronous = input.peek(Token![async]);
        if asynchronous {
            input.parse::<Token![async]>()?;
        }
        let ty = input.parse::<Type>()?;
        let Type::FnPtr(signature) = ty else {
            return Err(syn::Error::new_spanned(
                ty,
                "expected a function pointer signature such as `fn(&T) -> U`",
            ));
        };
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after the method signature"));
        }
        Ok(Self {
            method,
            signature,
            asynchronous,
        })
    }
}

/// Expands `bind_method!(path as fn(signature))` to a concrete binding value.
pub(crate) fn expand(input: TokenStream) -> Result<TokenStream> {
    let context = ExpansionContext::from_attributes(&[])?;
    expand_with_context(input, &context)
}

/// Expands a binding using a pre-resolved runtime facade.
pub(crate) fn expand_with_context(
    input: TokenStream,
    context: &ExpansionContext,
) -> Result<TokenStream> {
    let input: BindingInput = parse2(input)?;
    let asynchronous = input.asynchronous;
    if input.signature.unsafety.is_some()
        || input.signature.abi.is_some()
        || input.signature.variadic.is_some()
    {
        return Err(syn::Error::new_spanned(
            input.signature,
            "bind_method! requires a safe, Rust ABI, non-variadic function signature",
        ));
    }
    let facade = context.facade().clone();
    let method_path = input.method;
    let signature = input.signature;
    let function_type = quote!(#signature);
    let method_ident = method_path
        .path
        .segments
        .last()
        .expect("method path is non-empty")
        .ident
        .clone();
    let target_type = if let Some(qself) = &method_path.qself {
        (*qself.ty).clone()
    } else {
        let mut prefix = method_path.path.clone();
        prefix.segments.pop();
        if prefix.segments.trailing_punct() {
            prefix.segments.pop_punct();
        }
        if prefix.segments.is_empty() {
            return Err(syn::Error::new_spanned(
                method_path,
                "method path must include a concrete target type",
            ));
        }
        for segment in &mut prefix.segments {
            if let syn::PathArguments::AngleBracketed(arguments) = &mut segment.arguments {
                arguments.colon2_token = None;
            }
        }
        Type::Path(syn::TypePath {
            qself: None,
            path: prefix,
            attrs: Vec::new(),
        })
    };
    let inputs = &signature.inputs;
    let first_is_receiver = inputs.first().is_some_and(|arg| {
        receiver_target(&arg.ty).is_some_and(|candidate| {
            candidate.to_token_stream().to_string() == target_type.to_token_stream().to_string()
        })
    });
    let mut input_types: Vec<Type> = inputs.iter().map(|arg| arg.ty.clone()).collect();
    let receiver_ty = if first_is_receiver {
        Some(input_types.remove(0))
    } else {
        None
    };
    let pinned_mut_receiver = receiver_ty.as_ref().is_some_and(is_pin_mut_type);
    let target_ir = crate::parse::convert_type(&target_type);
    let target_application =
        super::traits::type_expression(&target_ir, &GenericEnvironment::new(), &facade);
    let target = quote!(#target_type);
    let expectations = input_types.iter().map(|ty| expectation(ty, &facade));
    let bindings = input_types
        .iter()
        .enumerate()
        .map(|(index, ty)| binding(ty, index, &facade));
    let args: Vec<_> = (0..input_types.len())
        .map(|index| format_ident!("__arg_{index}"))
        .collect();
    let receiver_expectation = receiver_ty.as_ref().map_or_else(
        || quote!(#facade::__private::codegen_v4::invoke::ReceiverExpectation::none()),
        |ty| expectation_receiver(ty, &facade),
    );
    let receiver_binding = if pinned_mut_receiver {
        quote! {
            let (__receiver, __arguments) = __validated.into_parts();
            let __receiver: ::core::pin::Pin<&mut #target> = __receiver;
        }
    } else {
        receiver_ty.as_ref().map_or_else(
            || quote!(let (_receiver, __arguments) = __validated.into_parts();),
            |ty| receiver_binding(ty, &target, &facade),
        )
    };
    let result = signature.output.clone();
    let output = match result {
        syn::ReturnType::Default => {
            quote!({ __call; Ok(#facade::__private::codegen_v4::invoke::InvocationOutput::Unit) })
        }
        syn::ReturnType::Type(_, ty) if matches!(*ty, Type::Tuple(ref tuple) if tuple.elems.is_empty()) =>
        {
            quote!({ __call; Ok(#facade::__private::codegen_v4::invoke::InvocationOutput::Unit) })
        }
        syn::ReturnType::Type(_, _ty) if asynchronous => quote! {
            Ok(#facade::__private::codegen_v4::invoke::InvocationOutput::Future(
                #facade::__private::codegen_v4::invoke::ReflectedFuture::<#facade::__private::codegen_v4::value::Local>::new(async move {
                    let __value = __call.await;
                    #facade::__private::codegen_v4::invoke::InvocationOutput::Owned(
                        #facade::__private::codegen_v4::value::DynamicOwned::<#facade::__private::codegen_v4::value::Local>::new(__value),
                    )
                }),
            ))
        },
        syn::ReturnType::Type(_, ty) if matches!(*ty, Type::Reference(ref reference) if reference.mutability.is_none()) =>
        {
            let mut origins = Vec::new();
            if receiver_ty
                .as_ref()
                .is_some_and(|receiver| matches!(receiver, Type::Reference(_)))
            {
                origins
                    .push(quote!(#facade::__private::codegen_v4::invoke::BorrowOrigin::Receiver));
            }
            for (index, argument) in input_types.iter().enumerate() {
                if matches!(argument, Type::Reference(_)) {
                    origins.push(quote!(#facade::__private::codegen_v4::invoke::BorrowOrigin::Parameter(#index)));
                }
            }
            quote! {
                Ok(#facade::__private::codegen_v4::invoke::InvocationOutput::Ref {
                    value: #facade::__private::codegen_v4::value::DynamicRef::<#facade::__private::codegen_v4::value::Local>::new(__call),
                    origins: ::std::vec![#(#origins),*].into_boxed_slice(),
                })
            }
        }
        syn::ReturnType::Type(_, _ty) => quote! {
            Ok(#facade::__private::codegen_v4::invoke::InvocationOutput::Owned(
                #facade::__private::codegen_v4::value::DynamicOwned::<#facade::__private::codegen_v4::value::Local>::new(__call),
            ))
        },
    };
    let call = if let Some(receiver) = &receiver_ty {
        quote!(let __receiver: #receiver = __receiver; let __call = __typed_method(__receiver, #(#args),*);)
    } else {
        quote!(let __call = __typed_method(#(#args),*);)
    };
    let typed_method = if asynchronous {
        quote!(let __typed_method = #method_path;)
    } else {
        quote!(let __typed_method: #function_type = #method_path;)
    };
    let invocation_type = if pinned_mut_receiver {
        quote!(#facade::__private::codegen_v4::invoke::PinnedMutInvocation<'call, #target, #facade::__private::codegen_v4::value::Local>)
    } else {
        quote!(#facade::__private::codegen_v4::invoke::Invocation<'call, #facade::__private::codegen_v4::value::Local>)
    };
    let failure_type = if pinned_mut_receiver {
        quote!(#facade::__private::codegen_v4::invoke::PinnedMutInvocationFailure<'call, #target, #facade::__private::codegen_v4::value::Local>)
    } else {
        quote!(#facade::__private::codegen_v4::invoke::InvocationFailure<'call, #facade::__private::codegen_v4::value::Local>)
    };
    let validate = if pinned_mut_receiver {
        quote!(let __validated = __invocation.validate(&__identity, &[#(#expectations),*])?;)
    } else {
        quote!(let __validated = __invocation.validate(&__identity, #receiver_expectation, &[#(#expectations),*])?;)
    };
    let adapter = if pinned_mut_receiver {
        quote!({
            static ENTRY: #facade::__private::codegen_v4::invoke::PinnedMutAdapter<#target, #facade::__private::codegen_v4::value::Local> = __qubit_bind_method_adapter;
            #facade::__private::codegen_v4::descriptor::InvocationAdapter::pinned_mut_local::<#target>(&ENTRY)
        })
    } else {
        quote!(#facade::__private::codegen_v4::descriptor::InvocationAdapter::local(__qubit_bind_method_adapter))
    };
    let shim = quote! {
        fn __qubit_bind_method_adapter<'registry, 'call>(
            __registry: &'registry #facade::__private::codegen_v4::registry::ReflectRegistry,
            __invocation: #invocation_type,
        ) -> ::core::result::Result<
            #facade::__private::codegen_v4::invoke::InvocationOutput<'call, #facade::__private::codegen_v4::value::Local>,
            #failure_type,
        > {
            #typed_method
            let __identity = #facade::__private::codegen_v4::identity::MemberId::new(
                ::std::any::type_name::<#target>(), "method", 0,
                #facade::__private::codegen_v4::identity::FragmentIdentity::new(env!("CARGO_PKG_NAME"), module_path!(), line!(), column!(), "method-binding", 0),
            );
            #validate
            #receiver_binding
            let mut arguments = __arguments.into_vec().into_iter();
            #(#bindings)*
            let _ = __registry;
            #call
            #output
        }
        static __QUBIT_BIND_METHOD_ADAPTER: #facade::__private::codegen_v4::descriptor::InvocationAdapter = #adapter;
    };
    let method_ir = crate::parse::convert_path(&method_path.path);
    let method_arguments = method_ir
        .segments
        .last()
        .map(|segment| {
            path_arguments(
                &segment.arguments,
                &GenericEnvironment::new(),
                &facade,
                method_path.span(),
            )
        })
        .unwrap_or_default();
    let binding_expr = if let Some(qself) = &method_path.qself {
        let trait_path = syn::Path {
            leading_colon: None,
            segments: method_path
                .path
                .segments
                .iter()
                .take(qself.position)
                .cloned()
                .collect(),
        };
        let trait_ir = crate::parse::convert_path(&trait_path);
        let trait_expression = path_type_expression(&trait_ir, &GenericEnvironment::new(), &facade);
        quote! {
            #facade::__private::codegen_v4::registry::MethodBinding::trait_method(
                #facade::__private::codegen_v4::TypeDescriptor::of::<#target>(),
                <#target as #trait_path>::__qubit_reflect_trait_payload().definition().trait_id().clone(),
                #trait_expression,
                stringify!(#method_ident), vec![#(#method_arguments),*].into_boxed_slice(),
                &__QUBIT_BIND_METHOD_ADAPTER,
            ).with_target_application(#target_application)
        }
    } else {
        quote! {
            #facade::__private::codegen_v4::registry::MethodBinding::inherent(
                #facade::__private::codegen_v4::TypeDescriptor::of::<#target>(),
                stringify!(#method_ident), vec![#(#method_arguments),*].into_boxed_slice(),
                &__QUBIT_BIND_METHOD_ADAPTER,
            ).with_target_application(#target_application)
        }
    };
    Ok(crate::expand::invocation::emit::rewrite_codegen_paths(
        quote! {{ #shim #binding_expr }},
    ))
}

fn expectation(ty: &Type, facade: &TokenStream) -> TokenStream {
    match ty {
        Type::Reference(reference) if reference.mutability.is_some() => {
            let element = &reference.elem;
            quote!(#facade::__private::codegen_v4::invoke::ArgumentExpectation::borrowed_mut::<#element>())
        }
        Type::Reference(reference) => {
            let element = &reference.elem;
            quote!(#facade::__private::codegen_v4::invoke::ArgumentExpectation::borrowed::<#element>())
        }
        _ => quote!(#facade::__private::codegen_v4::invoke::ArgumentExpectation::owned::<#ty>()),
    }
}

fn receiver_target(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Reference(reference) => Some(&reference.elem),
        Type::Path(path)
            if path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "Pin") =>
        {
            let syn::PathArguments::AngleBracketed(arguments) =
                &path.path.segments.last()?.arguments
            else {
                return None;
            };
            match arguments.args.first()? {
                syn::GenericArgument::Type(Type::Reference(reference)) => Some(&reference.elem),
                _ => None,
            }
        }
        other => Some(other),
    }
}

fn is_pin_mut_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(path)
    if path.path.segments.last().is_some_and(|segment| segment.ident == "Pin")
        && path.path.segments.last().is_some_and(|segment| matches!(&segment.arguments,
            syn::PathArguments::AngleBracketed(arguments)
                if matches!(arguments.args.first(), Some(syn::GenericArgument::Type(Type::Reference(reference))) if reference.mutability.is_some())
        )))
}

fn expectation_receiver(ty: &Type, facade: &TokenStream) -> TokenStream {
    match ty {
        Type::Reference(reference) if reference.mutability.is_some() => {
            let element = &reference.elem;
            quote!(#facade::__private::codegen_v4::invoke::ReceiverExpectation::borrowed_mut::<#element>())
        }
        Type::Reference(reference) => {
            let element = &reference.elem;
            quote!(#facade::__private::codegen_v4::invoke::ReceiverExpectation::borrowed::<#element>())
        }
        _ => quote!(#facade::__private::codegen_v4::invoke::ReceiverExpectation::owned::<#ty>()),
    }
}

fn receiver_binding(ty: &Type, target: &TokenStream, facade: &TokenStream) -> TokenStream {
    match ty {
        Type::Reference(reference) if reference.mutability.is_some() => quote! {
            let (__receiver, __arguments) = __validated.into_parts();
            let __receiver: &mut #target = match __receiver {
                Some(#facade::__private::codegen_v4::invoke::InvocationReceiver::Mut(value)) =>
                    #facade::__private::codegen_v4::value::DynamicMut::<#facade::__private::codegen_v4::value::Local>::downcast::<#target>(value).unwrap_or_else(|_| unreachable!()),
                _ => unreachable!(),
            };
        },
        Type::Reference(_) => quote! {
            let (__receiver, __arguments) = __validated.into_parts();
            let __receiver: &#target = match __receiver {
                Some(#facade::__private::codegen_v4::invoke::InvocationReceiver::Ref(value)) =>
                    #facade::__private::codegen_v4::value::DynamicRef::<#facade::__private::codegen_v4::value::Local>::downcast::<#target>(value).unwrap_or_else(|_| unreachable!()),
                Some(#facade::__private::codegen_v4::invoke::InvocationReceiver::Mut(value)) => {
                    let value = #facade::__private::codegen_v4::value::DynamicMut::<#facade::__private::codegen_v4::value::Local>::downcast::<#target>(value).unwrap_or_else(|_| unreachable!()); &*value
                }
                _ => unreachable!(),
            };
        },
        _ => quote! {
            let (__receiver, __arguments) = __validated.into_parts();
            let __receiver: #target = match __receiver {
                Some(#facade::__private::codegen_v4::invoke::InvocationReceiver::Owned(value)) =>
                    #facade::__private::codegen_v4::value::DynamicOwned::<#facade::__private::codegen_v4::value::Local>::downcast::<#target>(value).unwrap_or_else(|_| unreachable!()),
                _ => unreachable!(),
            };
        },
    }
}

fn binding(ty: &Type, index: usize, facade: &TokenStream) -> TokenStream {
    let value = format_ident!("__arg_{index}");
    match ty {
        Type::Reference(reference) if reference.mutability.is_some() => {
            let element = &reference.elem;
            quote! { let #value: &mut #element = match arguments.next().expect("validated argument") {
                #facade::__private::codegen_v4::invoke::InvocationArg::Mut(value) =>
                    #facade::__private::codegen_v4::value::DynamicMut::<#facade::__private::codegen_v4::value::Local>::downcast::<#element>(value).unwrap_or_else(|_| unreachable!()),
                _ => unreachable!(),
            }; }
        }
        Type::Reference(reference) => {
            let element = &reference.elem;
            quote! { let #value: &#element = match arguments.next().expect("validated argument") {
                #facade::__private::codegen_v4::invoke::InvocationArg::Ref(value) => #facade::__private::codegen_v4::value::DynamicRef::<#facade::__private::codegen_v4::value::Local>::downcast::<#element>(value).unwrap_or_else(|_| unreachable!()),
                #facade::__private::codegen_v4::invoke::InvocationArg::Mut(value) => { let value = #facade::__private::codegen_v4::value::DynamicMut::<#facade::__private::codegen_v4::value::Local>::downcast::<#element>(value).unwrap_or_else(|_| unreachable!()); &*value },
                _ => unreachable!(),
            }; }
        }
        _ => quote! { let #value: #ty = match arguments.next().expect("validated argument") {
            #facade::__private::codegen_v4::invoke::InvocationArg::Owned(value) => #facade::__private::codegen_v4::value::DynamicOwned::<#facade::__private::codegen_v4::value::Local>::downcast::<#ty>(value).unwrap_or_else(|_| unreachable!()),
            _ => unreachable!(),
        }; },
    }
}
