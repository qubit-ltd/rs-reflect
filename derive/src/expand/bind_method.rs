//! Call-site binding of one explicitly monomorphized method.

use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::format_ident;
use quote::quote;
use syn::Expr;
use syn::ExprPath;
use syn::Lit;
use syn::Result;
use syn::Token;
use syn::Type;
use syn::TypeFnPtr;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse2;
use syn::spanned::Spanned;

use crate::expand::context::ExpansionContext;
use crate::expand::generic_environment::GenericEnvironment;

struct BindingInput {
    method: ExprPath,
    signature: TypeFnPtr,
    asynchronous: bool,
    thread_safe: bool,
}

impl Parse for BindingInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let method = input.parse()?;
        input.parse::<Token![as]>()?;
        let thread_safe = input.peek(syn::Ident)
            && input.fork().parse::<syn::Ident>()?.to_string() == "thread_safe";
        if thread_safe {
            let mode = input.parse::<syn::Ident>()?;
            if mode != "thread_safe" {
                return Err(syn::Error::new_spanned(
                    mode,
                    "expected `thread_safe` adapter mode",
                ));
            }
        }
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
            thread_safe,
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
    let thread_safe = input.thread_safe;
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
    let method_ident = method_path
        .path
        .segments
        .last()
        .expect("method path is non-empty")
        .ident
        .clone();
    let specialization_arguments = match &method_path
        .path
        .segments
        .last()
        .expect("method path is non-empty")
        .arguments
    {
        syn::PathArguments::AngleBracketed(arguments) => arguments
            .args
            .iter()
            .map(|argument| specialization_argument(argument, &facade))
            .collect::<Result<Vec<_>>>()?,
        _ => Vec::new(),
    };
    let is_generic = !specialization_arguments.is_empty();
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
    let target = quote!(#target_type);
    let typed_method = if asynchronous {
        quote!(let __typed_method = #method_path;)
    } else {
        let function_type = quote!(#signature);
        quote!(let __typed_method: #function_type = #method_path;)
    };
    let receiver_kind = receiver_ty.as_ref().map(|receiver| {
        if is_pin_mut_type(receiver) {
            quote!(PinnedMutable)
        } else if is_pin_shared_type(receiver) {
            quote!(PinnedShared)
        } else if is_mut_reference(receiver) {
            quote!(Mutable)
        } else if is_reference(receiver) {
            quote!(Shared)
        } else {
            quote!(Owned)
        }
    });
    let receiver_metadata = receiver_kind.map_or_else(
        || quote!(None),
        |kind| quote!(Some(#facade::__private::codegen_v4::invoke::ReceiverDescriptor::for_type::<#target>(
            #facade::__private::codegen_v4::invoke::ReceiverKind::#kind,
        ))),
    );
    let parameters: Vec<_> = input_types
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let ty_ir = crate::parse::convert_type(ty);
            let expression =
                super::traits::type_expression(&ty_ir, &GenericEnvironment::new(), &facade);
            let passing = if is_mut_reference(ty) {
                quote!(Mutable)
            } else if is_reference(ty) {
                quote!(Shared)
            } else {
                quote!(Owned)
            };
            quote!(#facade::__private::codegen_v4::invoke::ParameterMetadata::new(
                #index,
                #facade::__private::codegen_v4::invoke::ParameterPattern::Identifier(""),
                #facade::__private::codegen_v4::invoke::PassingMode::#passing,
                #expression,
            ))
        })
        .collect();
    let (return_metadata, return_kind) = match &signature.output {
        syn::ReturnType::Default => (
            quote!(#facade::__private::codegen_v4::invoke::ReturnMetadata::unit()),
            "Unit",
        ),
        syn::ReturnType::Type(_, ty) if matches!(**ty, Type::Tuple(ref tuple) if tuple.elems.is_empty()) => {
            (
                quote!(#facade::__private::codegen_v4::invoke::ReturnMetadata::unit()),
                "Unit",
            )
        }
        syn::ReturnType::Type(_, ty) => {
            let ty_ir = crate::parse::convert_type(ty);
            let expression =
                super::traits::type_expression(&ty_ir, &GenericEnvironment::new(), &facade);
            let (kind, kind_name) = if asynchronous {
                (quote!(Future), "Future")
            } else if is_mut_reference(ty) {
                (quote!(Mutable), "Mutable")
            } else if is_reference(ty) {
                (quote!(Shared), "Shared")
            } else if matches!(**ty, Type::Never(_)) {
                (quote!(Never), "Never")
            } else {
                (quote!(Owned), "Owned")
            };
            (
                quote!(#facade::__private::codegen_v4::invoke::ReturnMetadata::new(
                    #facade::__private::codegen_v4::invoke::ReturnKind::#kind,
                    #expression,
                )),
                kind_name,
            )
        }
    };
    let generic_definition = quote!(#facade::__private::codegen_v4::expression::GenericDefinitionMetadata::new(&[], &[]));
    let method_name = syn::LitStr::new(&method_ident.to_string(), method_ident.span());
    let implementation = syn::LitStr::new(
        &target_type.to_token_stream().to_string(),
        method_path.span(),
    );
    let namespace = if let Some(qself) = &method_path.qself {
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
        let trait_name =
            syn::LitStr::new(&trait_path.to_token_stream().to_string(), trait_path.span());
        quote!(#facade::__private::codegen_v4::invoke::MethodNamespaceMetadata::Trait {
            trait_: #trait_name,
            implementation: #implementation,
        })
    } else {
        quote!(#facade::__private::codegen_v4::invoke::MethodNamespaceMetadata::Inherent {
            implementation: #implementation,
        })
    };
    let qualifiers = quote!(#facade::__private::codegen_v4::invoke::MethodQualifiers::new(false, #asynchronous, false, false));
    let metadata = quote! {
        fn __qubit_bind_method_metadata() -> &'static #facade::__private::codegen_v4::invoke::MethodMetadata {
            static VALUE: ::std::sync::OnceLock<#facade::__private::codegen_v4::invoke::MethodMetadata> = ::std::sync::OnceLock::new();
            VALUE.get_or_init(|| {
                static GENERICS: #facade::__private::codegen_v4::expression::GenericDefinitionMetadata = #generic_definition;
                let parameters = ::std::boxed::Box::leak(
                    ::std::vec![#(#parameters),*].into_boxed_slice(),
                );
                let output = ::std::boxed::Box::leak(::std::boxed::Box::new(#return_metadata));
                #facade::__private::codegen_v4::invoke::MethodMetadata::from_spec(
                    #facade::__private::codegen_v4::invoke::MethodMetadataSpec::new(
                        #method_name,
                        #namespace,
                        #facade::__private::codegen_v4::invoke::MethodVisibility::Public,
                        #receiver_metadata,
                        parameters,
                        output,
                        #qualifiers,
                        &GENERICS,
                        #is_generic,
                        false,
                    ),
                )
            })
        }
    };
    let argument_names: Vec<_> = (0..input_types.len())
        .map(|index| format_ident!("__arg_{index}"))
        .collect();
    let argument_bindings: Vec<_> = input_types
        .iter()
        .enumerate()
        .map(|(index, ty)| binding(ty, index, &facade))
        .collect();
    let receiver_binding = receiver_ty.as_ref().map_or_else(
        || quote!(let (_receiver, __arguments) = __validated.into_parts();),
        |receiver| {
            if is_pin_mut_type(receiver) {
                quote! {
                    let (_receiver, __arguments) = __validated.into_parts();
                    let __receiver = __pinned_receiver.downcast_mut::<#target>()
                        .unwrap_or_else(|| unreachable!("validated invocation checked pinned receiver"));
                }
            } else if is_pin_shared_type(receiver) {
                quote! {
                    let (_receiver, __arguments) = __validated.into_parts();
                    let __receiver = __pinned_receiver.downcast_ref::<#target>()
                        .unwrap_or_else(|| unreachable!("validated invocation checked pinned receiver"));
                }
            } else if is_mut_reference(receiver) {
                quote! {
                    let (__receiver, __arguments) = __validated.into_parts();
                    let __receiver = __receiver.expect("validated invocation has a receiver")
                        .into_mut::<#target>()
                        .unwrap_or_else(|_| unreachable!("validated invocation checked receiver"));
                }
            } else if is_reference(receiver) {
                quote! {
                    let (__receiver, __arguments) = __validated.into_parts();
                    let __receiver = __receiver.expect("validated invocation has a receiver");
                    let __receiver = match __receiver.into_ref::<#target>() {
                        Ok(value) => value,
                        Err(receiver) => receiver.into_mut::<#target>()
                            .map(|value| &*value)
                            .unwrap_or_else(|_| unreachable!("validated invocation checked receiver")),
                    };
                }
            } else {
                quote! {
                    let (__receiver, __arguments) = __validated.into_parts();
                    let __receiver = __receiver.expect("validated invocation has a receiver")
                        .into_owned::<#target>()
                        .unwrap_or_else(|_| unreachable!("validated invocation checked receiver"));
                }
            }
        },
    );
    let call_arguments: Vec<_> = receiver_ty
        .as_ref()
        .map(|_| quote!(__receiver))
        .into_iter()
        .chain(argument_names.iter().map(|name| quote!(#name)))
        .collect();
    let call = quote!(let __call = __typed_method(#(#call_arguments),*););
    let output = match return_kind {
        "Unit" => {
            quote!({ __call; #facade::__private::codegen_v4::invoke::InvocationOutput::unit() })
        }
        "Shared" => quote!(#facade::__private::codegen_v4::invoke::InvocationOutput::shared(
            __call,
            #facade::__private::codegen_v4::invoke::BorrowOrigin::Receiver,
        )),
        "Mutable" => quote!(#facade::__private::codegen_v4::invoke::InvocationOutput::mutable(
            __call,
            #facade::__private::codegen_v4::invoke::BorrowOrigin::Receiver,
        )),
        "Future" => {
            quote!(#facade::__private::codegen_v4::invoke::InvocationOutput::future(async move {
                #facade::__private::codegen_v4::invoke::InvocationOutput::owned(__call.await)
            }))
        }
        "Never" => quote!(match __call {}),
        _ => quote!(#facade::__private::codegen_v4::invoke::InvocationOutput::owned(__call)),
    };
    let mode = if thread_safe {
        quote!(#facade::__private::codegen_v4::value::ThreadSafe)
    } else {
        quote!(#facade::__private::codegen_v4::value::Local)
    };
    let adapter_fn = format_ident!("__qubit_bind_method_adapter");
    let pinned = receiver_ty
        .as_ref()
        .is_some_and(|ty| is_pin_mut_type(ty) || is_pin_shared_type(ty));
    let adapter = if pinned {
        let constructor = if receiver_ty.as_ref().is_some_and(is_pin_mut_type) {
            quote!(mutable)
        } else {
            quote!(shared)
        };
        quote! {
            static __QUBIT_BIND_METHOD_ADAPTER: #facade::__private::codegen_v4::invoke::PinnedFunctionInvocationAdapter<#mode, #target> =
                #facade::__private::codegen_v4::invoke::PinnedFunctionInvocationAdapter::<#mode, #target>::#constructor(#adapter_fn);
        }
    } else {
        quote! {
            static __QUBIT_BIND_METHOD_ADAPTER: #facade::__private::codegen_v4::invoke::FunctionInvocationAdapter<#mode> =
                #facade::__private::codegen_v4::invoke::FunctionInvocationAdapter::<#mode>::new(#adapter_fn);
        }
    };
    let adapter_args = if pinned {
        quote!(
            __pinned_receiver: #facade::__private::codegen_v4::invoke::PinnedReceiver<'call>,
            __validated: #facade::__private::codegen_v4::invoke::ValidatedInvocation<'call, #mode>,
        )
    } else {
        quote!(__validated: #facade::__private::codegen_v4::invoke::ValidatedInvocation<'call, #mode>)
    };
    let adapters = if pinned {
        if receiver_ty.as_ref().is_some_and(is_pin_mut_type) {
            if thread_safe {
                quote!(#facade::__private::codegen_v4::invoke::InvocationAdapters::builder().pinned_thread_safe::<#target>(&__QUBIT_BIND_METHOD_ADAPTER).build())
            } else {
                quote!(#facade::__private::codegen_v4::invoke::InvocationAdapters::builder().pinned_local::<#target>(&__QUBIT_BIND_METHOD_ADAPTER).build())
            }
        } else {
            if thread_safe {
                quote!(#facade::__private::codegen_v4::invoke::InvocationAdapters::builder().pinned_thread_safe::<#target>(&__QUBIT_BIND_METHOD_ADAPTER).build())
            } else {
                quote!(#facade::__private::codegen_v4::invoke::InvocationAdapters::builder().pinned_local::<#target>(&__QUBIT_BIND_METHOD_ADAPTER).build())
            }
        }
    } else if thread_safe {
        quote!(#facade::__private::codegen_v4::invoke::InvocationAdapters::builder().thread_safe(&__QUBIT_BIND_METHOD_ADAPTER).build())
    } else {
        quote!(#facade::__private::codegen_v4::invoke::InvocationAdapters::builder().local(&__QUBIT_BIND_METHOD_ADAPTER).build())
    };
    let binding = if is_generic {
        let specialization_argument_count = specialization_arguments.len();
        quote! {
            static __QUBIT_BIND_METHOD_SPECIALIZATION_ARGUMENTS:
                [#facade::__private::codegen_v4::invoke::MethodSpecializationArgument; #specialization_argument_count] =
                [#(#specialization_arguments),*];
            static __QUBIT_BIND_METHOD_SPECIALIZATIONS:
                [#facade::__private::codegen_v4::invoke::MethodSpecializationSpec; 1] = [
                #facade::__private::codegen_v4::invoke::MethodSpecializationSpec::new(
                    &__QUBIT_BIND_METHOD_SPECIALIZATION_ARGUMENTS,
                    #adapters,
                ),
            ];
            #facade::__private::codegen_v4::registration::bind_method::<#target>(
                __qubit_bind_method_metadata(),
                #adapters,
            )
            .with_specializations(&__QUBIT_BIND_METHOD_SPECIALIZATIONS)
        }
    } else {
        quote!(#facade::__private::codegen_v4::registration::bind_method::<#target>(__qubit_bind_method_metadata(), #adapters))
    };
    let shim = quote! {
        #metadata
        fn #adapter_fn<'call>(#adapter_args) -> #facade::__private::codegen_v4::invoke::InvocationOutput<'call, #mode> {
            #typed_method
            #receiver_binding
            let mut arguments = __arguments.into_vec().into_iter();
            #(#argument_bindings)*
            #call
            #output
        }
        #adapter
    };
    Ok(crate::expand::invocation::emit::rewrite_codegen_paths(
        quote! {{ #shim #binding }},
    ))
}

fn specialization_argument(
    argument: &syn::GenericArgument,
    facade: &TokenStream,
) -> Result<TokenStream> {
    let path = quote!(#facade::__private::codegen_v4::invoke::MethodSpecializationArgument);
    let token = match argument {
        syn::GenericArgument::Type(ty) => quote!(#path::type_argument::<#ty>()),
        syn::GenericArgument::Lifetime(lifetime) => {
            let lifetime =
                syn::LitStr::new(&lifetime.to_token_stream().to_string(), lifetime.span());
            quote!(#path::lifetime(#lifetime))
        }
        syn::GenericArgument::Const(expression) => {
            const_specialization_argument(expression, &path)?
        }
        other => {
            return Err(syn::Error::new_spanned(
                other,
                "method specialization accepts only type, lifetime, and scalar const arguments",
            ));
        }
    };
    Ok(token)
}

fn const_specialization_argument(expression: &Expr, path: &TokenStream) -> Result<TokenStream> {
    let (negative, literal) = match expression {
        Expr::Lit(literal) => (false, &literal.lit),
        Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Neg(_)) => {
            let Expr::Lit(literal) = unary.expr.as_ref() else {
                return Err(syn::Error::new_spanned(
                    expression,
                    "method specialization const argument must be a scalar literal",
                ));
            };
            (true, &literal.lit)
        }
        _ => {
            return Err(syn::Error::new_spanned(
                expression,
                "method specialization const argument must be a typed scalar literal",
            ));
        }
    };
    let token = match literal {
        Lit::Bool(value) => {
            if negative {
                return Err(syn::Error::new_spanned(
                    expression,
                    "boolean const cannot be negative",
                ));
            }
            let value = value.value;
            quote!(#path::const_bool(#value))
        }
        Lit::Char(value) => {
            if negative {
                return Err(syn::Error::new_spanned(
                    expression,
                    "character const cannot be negative",
                ));
            }
            let value = value.value();
            quote!(#path::const_char(#value))
        }
        Lit::Int(value) => {
            let suffix = value.suffix();
            let raw_digits = value.base10_digits();
            let (negative, magnitude_digits) = raw_digits
                .strip_prefix('-')
                .map_or((negative, raw_digits), |digits| (true, digits));
            if suffix.is_empty() {
                let magnitude = magnitude_digits.parse::<u128>().map_err(|error| {
                    syn::Error::new_spanned(
                        expression,
                        format!(
                            "integer const specialization `{raw_digits}` is out of range: {error}"
                        ),
                    )
                })?;
                quote!(#path::const_integer_literal(#negative, #magnitude))
            } else {
                let constructor = match suffix {
                    "i8" => quote!(i8),
                    "i16" => quote!(i16),
                    "i32" => quote!(i32),
                    "i64" => quote!(i64),
                    "i128" => quote!(i128),
                    "isize" => quote!(isize),
                    "u8" if !negative => quote!(u8),
                    "u16" if !negative => quote!(u16),
                    "u32" if !negative => quote!(u32),
                    "u64" if !negative => quote!(u64),
                    "u128" if !negative => quote!(u128),
                    "usize" if !negative => quote!(usize),
                    _ => {
                        return Err(syn::Error::new_spanned(
                            expression,
                            "unsupported integer const specialization type",
                        ));
                    }
                };
                quote!(#path::#constructor(#expression))
            }
        }
        _ => {
            return Err(syn::Error::new_spanned(
                expression,
                "method specialization const argument must be a bool, char, or typed integer literal",
            ));
        }
    };
    Ok(token)
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

fn is_pin_shared_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(path)
    if path.path.segments.last().is_some_and(|segment| segment.ident == "Pin")
        && path.path.segments.last().is_some_and(|segment| matches!(&segment.arguments,
            syn::PathArguments::AngleBracketed(arguments)
                if matches!(arguments.args.first(), Some(syn::GenericArgument::Type(Type::Reference(reference))) if reference.mutability.is_none())
        )))
}

fn is_mut_reference(ty: &Type) -> bool {
    matches!(ty, Type::Reference(reference) if reference.mutability.is_some())
}

fn is_reference(ty: &Type) -> bool {
    matches!(ty, Type::Reference(_))
}

fn binding(ty: &Type, index: usize, _facade: &TokenStream) -> TokenStream {
    let value = format_ident!("__arg_{index}");
    match ty {
        Type::Reference(reference) if reference.mutability.is_some() => {
            let element = &reference.elem;
            quote! {
                let #value: &mut #element = arguments.next()
                    .expect("validated invocation contains each declared argument")
                    .into_mut::<#element>()
                    .unwrap_or_else(|_| unreachable!("validated invocation checked argument type and mode"));
            }
        }
        Type::Reference(reference) => {
            let element = &reference.elem;
            quote! {
                let #value = arguments.next()
                    .expect("validated invocation contains each declared argument");
                let #value: &#element = match #value.into_ref::<#element>() {
                    Ok(value) => value,
                    Err(value) => match value.into_mut::<#element>() {
                        Ok(value) => &*value,
                        Err(_) => unreachable!("validated invocation checked argument type and mode"),
                    },
                };
            }
        }
        _ => quote! {
            let #value: #ty = arguments.next()
                .expect("validated invocation contains each declared argument")
                .into_owned::<#ty>()
                .unwrap_or_else(|_| unreachable!("validated invocation checked argument type and mode"));
        },
    }
}
