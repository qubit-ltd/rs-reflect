// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Generic implementation specialization token generation.

use proc_macro2::Group;
use proc_macro2::Ident;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::format_ident;
use quote::quote;

use crate::ir::GenericKindIr;
use crate::ir::ImplDeclarationIr;
use crate::ir::MethodIr;
use crate::ir::PathArgumentIr;
use crate::ir::PathArgumentsIr;
use crate::ir::ReceiverKindIr;
use crate::ir::ReturnTypeIr;
use crate::ir::SpecializationIr;
use crate::ir::SpecializationValueIr;
use crate::ir::TypeIr;
use crate::ir::TypeKindIr;

/// Builds resolver proofs inside the original generic impl environment.
///
/// The helper is then monomorphized with the explicit specialization. This
/// lets rustc resolve aliases and renamed dependencies while ensuring an
/// unconstrained concrete binding is never inspected opportunistically.
pub(super) fn specialization_associated_type_resolver_arms(
    declaration: &ImplDeclarationIr,
    replacements: &[(Ident, TokenStream)],
    facade: &TokenStream,
) -> Vec<TokenStream> {
    let impl_declaration = &declaration.generics.impl_declaration;
    let where_clause = &declaration.generics.where_clause;
    let specialization_arguments: Vec<_> = declaration
        .generics
        .params
        .iter()
        .filter(|parameter| parameter.kind != GenericKindIr::Lifetime)
        .filter_map(|parameter| {
            replacements
                .iter()
                .find(|(name, _)| name == &Ident::new(&parameter.name, parameter.span))
                .map(|(_, value)| value)
        })
        .collect();
    declaration
        .associated_types
        .iter()
        .filter(|associated| associated.generics.params.is_empty())
        .filter_map(|associated| {
            let value = associated.value.as_ref()?;
            let value = &value.tokens;
            let name = syn::LitStr::new(&associated.name.to_string(), associated.span);
            Some(quote! {
                #name => {
                    fn resolve #impl_declaration ()
                        -> Option<#facade::__private::codegen_v1::descriptor::TypeDescriptorResolver>
                        #where_clause
                    {
                        use #facade::__private::codegen_v1::descriptor::ResolveReflectTypeDescriptor as _;
                        let probe = #facade::__private::codegen_v1::descriptor::ReflectArgumentProbe::<#value>::new();
                        (&probe).resolve_reflect_type_descriptor()
                    }
                    resolve::<#(#specialization_arguments),*>()
                }
            })
        })
        .collect()
}

/// Builds replacement tokens for one validated type or const specialization.
pub(super) fn specialization_replacements(
    specialization: &SpecializationIr,
) -> Vec<(Ident, TokenStream)> {
    specialization
        .bindings
        .iter()
        .map(|binding| {
            let tokens = match &binding.value {
                SpecializationValueIr::Type(ty) => ty.tokens.clone(),
                SpecializationValueIr::Const(tokens)
                | SpecializationValueIr::AmbiguousPath(tokens) => tokens.clone(),
            };
            (Ident::new(&binding.name, binding.span), tokens)
        })
        .collect()
}

/// Replaces generic identifiers recursively while retaining source grouping.
pub(super) fn substitute_tokens(
    tokens: &TokenStream,
    replacements: &[(Ident, TokenStream)],
) -> TokenStream {
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
                let mut replacement = Group::new(
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
pub(super) fn substitute_impl_method_types(
    declaration: &mut ImplDeclarationIr,
    replacements: &[(Ident, TokenStream)],
) {
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

/// Replaces impl generic references in associated-item binding types.
pub(super) fn substitute_impl_associated_item_types(
    declaration: &mut ImplDeclarationIr,
    replacements: &[(Ident, TokenStream)],
) {
    for associated in &mut declaration.associated_types {
        if let Some(value) = &mut associated.value {
            reparse_substituted_type(value, replacements);
        }
    }
    for associated in &mut declaration.associated_consts {
        reparse_substituted_type(&mut associated.ty, replacements);
    }
}

/// Rebuilds structural type IR after specialization changes a root path.
fn reparse_substituted_type(ty: &mut TypeIr, replacements: &[(Ident, TokenStream)]) {
    let tokens = substitute_tokens(&ty.tokens, replacements);
    let parsed: syn::Type = syn::parse2(tokens)
        .expect("validated specialization must retain valid associated-item type syntax");
    *ty = crate::parse::convert_type(&parsed);
}

/// Applies the runtime-root lifetime policy to one specialized impl.
pub(super) fn substitute_impl_lifetimes(
    declaration: &mut ImplDeclarationIr,
    lifetime_names: &[&str],
) {
    substitute_type_lifetimes(&mut declaration.target_type, lifetime_names);
    if let Some(trait_path) = &mut declaration.trait_path {
        substitute_path_lifetimes(trait_path, lifetime_names);
    }
    for method in &mut declaration.methods {
        if let Some(receiver) = &mut method.receiver {
            substitute_type_lifetimes(&mut receiver.ty, lifetime_names);
            receiver.declaration =
                substitute_lifetime_tokens(&receiver.declaration, lifetime_names);
        }
        for parameter in &mut method.parameters {
            substitute_type_lifetimes(&mut parameter.ty, lifetime_names);
        }
        if let ReturnTypeIr::Type(ty) = &mut method.return_type {
            substitute_type_lifetimes(ty, lifetime_names);
        }
    }
    for associated in &mut declaration.associated_types {
        if let Some(value) = &mut associated.value {
            substitute_type_lifetimes(value, lifetime_names);
        }
    }
    for associated in &mut declaration.associated_consts {
        substitute_type_lifetimes(&mut associated.ty, lifetime_names);
    }
}

/// Substitutes impl lifetimes with `'static` in one type IR.
fn substitute_type_lifetimes(ty: &mut TypeIr, lifetime_names: &[&str]) {
    ty.tokens = substitute_lifetime_tokens(&ty.tokens, lifetime_names);
    ty.source = ty.tokens.to_string();
    match &mut ty.kind {
        TypeKindIr::Path(path) => substitute_path_lifetimes(path, lifetime_names),
        TypeKindIr::Reference {
            lifetime, element, ..
        } => {
            if lifetime
                .as_deref()
                .is_some_and(|lifetime| lifetime_names.contains(&lifetime.trim_start_matches('\'')))
            {
                *lifetime = Some("'static".to_owned());
            }
            substitute_type_lifetimes(element, lifetime_names);
        }
        TypeKindIr::Slice(element) | TypeKindIr::Pointer { element, .. } => {
            substitute_type_lifetimes(element, lifetime_names);
        }
        TypeKindIr::Tuple(elements) => {
            for element in elements {
                substitute_type_lifetimes(element, lifetime_names);
            }
        }
        TypeKindIr::Array { element, length } => {
            substitute_type_lifetimes(element, lifetime_names);
            *length = substitute_lifetime_tokens(length, lifetime_names);
        }
        TypeKindIr::BareFunction { inputs, output, .. } => {
            for input in inputs {
                substitute_type_lifetimes(input, lifetime_names);
            }
            if let Some(output) = output {
                substitute_type_lifetimes(output, lifetime_names);
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

/// Substitutes impl lifetimes in one path and its structured arguments.
fn substitute_path_lifetimes(path: &mut crate::ir::PathIr, lifetime_names: &[&str]) {
    path.tokens = substitute_lifetime_tokens(&path.tokens, lifetime_names);
    path.source = path.tokens.to_string();
    if let Some(qualified_self) = &mut path.qualified_self {
        substitute_type_lifetimes(&mut qualified_self.ty, lifetime_names);
    }
    for segment in &mut path.segments {
        match &mut segment.arguments {
            PathArgumentsIr::None => {}
            PathArgumentsIr::AngleBracketed(arguments) => {
                for argument in arguments {
                    match argument {
                        PathArgumentIr::Lifetime(lifetime)
                            if lifetime_names.contains(&lifetime.trim_start_matches('\'')) =>
                        {
                            *lifetime = "'static".to_owned();
                        }
                        PathArgumentIr::Type(ty) | PathArgumentIr::AssociatedType { ty, .. } => {
                            substitute_type_lifetimes(ty, lifetime_names);
                        }
                        PathArgumentIr::Const(tokens)
                        | PathArgumentIr::AssociatedConst { value: tokens, .. }
                        | PathArgumentIr::Other(tokens) => {
                            *tokens = substitute_lifetime_tokens(tokens, lifetime_names);
                        }
                        PathArgumentIr::Lifetime(_) | PathArgumentIr::Constraint { .. } => {}
                    }
                }
            }
            PathArgumentsIr::Parenthesized { inputs, output } => {
                for input in inputs {
                    substitute_type_lifetimes(input, lifetime_names);
                }
                if let Some(output) = output {
                    substitute_type_lifetimes(output, lifetime_names);
                }
            }
        }
    }
}

/// Rewrites lifetime token pairs while preserving group delimiters and spans.
fn substitute_lifetime_tokens(tokens: &TokenStream, lifetime_names: &[&str]) -> TokenStream {
    let trees: Vec<_> = tokens.clone().into_iter().collect();
    let mut output = TokenStream::new();
    let mut index = 0;
    while index < trees.len() {
        if let TokenTree::Punct(punctuation) = &trees[index]
            && punctuation.as_char() == '\''
            && let Some(TokenTree::Ident(identifier)) = trees.get(index + 1)
            && lifetime_names.contains(&identifier.to_string().as_str())
        {
            output.extend(quote!('static));
            index += 2;
            continue;
        }
        match &trees[index] {
            TokenTree::Group(group) => {
                let mut replacement = Group::new(
                    group.delimiter(),
                    substitute_lifetime_tokens(&group.stream(), lifetime_names),
                );
                replacement.set_span(group.span());
                output.extend([TokenTree::Group(replacement)]);
            }
            tree => output.extend([tree.clone()]),
        }
        index += 1;
    }
    output
}

/// Recursively substitutes one type and every nested type retained by its IR.
pub(super) fn substitute_type_tokens(ty: &mut TypeIr, replacements: &[(Ident, TokenStream)]) {
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
                        PathArgumentIr::Lifetime(_)
                        | PathArgumentIr::Constraint { .. }
                        | PathArgumentIr::Other(_) => {}
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

/// Returns a non-core explicit receiver type that requires a registered
/// `ReceiverAdapter` capability for dynamic invocation.
pub(super) fn typed_extension_receiver_type(
    receiver: &crate::ir::ReceiverIr,
    target: &TokenStream,
) -> Option<TokenStream> {
    if receiver.kind != ReceiverKindIr::Typed
        || crate::expand::invocation::analysis::typed_pinned_receiver_mutable(receiver).is_some()
        || crate::expand::invocation::analysis::typed_owned_receiver_type(receiver, &quote!(Self))
            .is_some()
    {
        return None;
    }
    let self_identifier = Ident::new("Self", receiver.span);
    Some(substitute_tokens(
        &receiver.ty.tokens,
        &[(self_identifier, target.clone())],
    ))
}

/// Emits concrete arguments for one validated method specialization.
pub(super) fn specialization_arguments(
    specialization: &SpecializationIr,
    generics: &crate::ir::GenericsIr,
    facade: &TokenStream,
) -> TokenStream {
    let arguments = generics.params.iter().filter_map(|parameter| {
        if parameter.kind == GenericKindIr::Lifetime {
            return None;
        }
        let binding = specialization
            .bindings
            .iter()
            .find(|binding| binding.name == parameter.name)?;
        match (parameter.kind, &binding.value) {
            (GenericKindIr::Type, SpecializationValueIr::Type(ty)) => {
                let expression = crate::expand::traits::type_expression(ty, facade);
                Some(quote!(#facade::__private::codegen_v1::expression::GenericArgument::Type(#expression)))
            }
            (GenericKindIr::Type, SpecializationValueIr::AmbiguousPath(tokens)) => Some(quote!(
                #facade::__private::codegen_v1::expression::GenericArgument::Type(
                    #facade::__private::codegen_v1::expression::TypeExpression::Concrete(
                        #facade::__private::codegen_v1::expression::concrete(
                            vec![stringify!(#tokens).into()].into_boxed_slice(),
                            vec![].into_boxed_slice(),
                            #facade::__private::codegen_v1::expression::DiagnosticText::from(stringify!(#tokens)),
                        ),
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
                    #facade::__private::codegen_v1::expression::GenericArgument::Const(
                        #facade::__private::codegen_v1::expression::ConstGenericArgument::new(
                            #facade::__private::codegen_v1::expression::TypeExpression::Concrete(
                                #facade::__private::codegen_v1::expression::concrete(
                                    vec![#declared_type_literal.into()].into_boxed_slice(),
                                    vec![].into_boxed_slice(),
                                    #facade::__private::codegen_v1::expression::DiagnosticText::from(#declared_type_literal),
                                ),
                            ),
                            #facade::__private::codegen_v1::expression::const_path([
                                stringify!(#tokens),
                            ]),
                            stringify!(#tokens),
                        ),
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
pub(super) fn simple_generic_specialization_adapter(
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
        || super::invocation_disabled_by_policy(method)
        || method
            .generics
            .params
            .iter()
            .any(|parameter| parameter.kind == GenericKindIr::Lifetime)
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
        .map(|parameter| specialize_type_tokens(&parameter.ty, &method.generics, specialization))
        .collect::<Option<_>>()?;
    let return_type = match &method.return_type {
        ReturnTypeIr::Unit => None,
        ReturnTypeIr::Type(ty) => Some(specialize_type_tokens(
            ty,
            &method.generics,
            specialization,
        )?),
    };
    let argument_expectations = parameter_types
        .iter()
        .map(|ty| quote!(#facade::__private::codegen_v1::invoke::ArgumentExpectation::owned::<#ty>()));
    let argument_bindings = parameter_types.iter().enumerate().map(|(index, ty)| {
        let argument = format_ident!("__qubit_reflect_specialized_argument_{index}");
        quote! {
            let #argument: #ty = match arguments.next().expect("validation checked argument count") {
                #facade::__private::codegen_v1::invoke::InvocationArg::Owned(value) =>
                    #facade::__private::codegen_v1::value::DynamicOwned::<#facade::__private::codegen_v1::value::Local>::downcast::<#ty>(value)
                        .unwrap_or_else(|_| unreachable!("validation checked argument type")),
                _ => unreachable!("validation checked argument mode"),
            };
        }
    });
    let call_arguments = (0..parameter_types.len())
        .map(|index| format_ident!("__qubit_reflect_specialized_argument_{index}"));
    let method_name = &method.name;
    let adapter_name = format_ident!(
        "__qubit_reflect_invoke_specialization_{method_index}_{specialization_index}"
    );
    let descriptor_name = format_ident!(
        "__QUBIT_REFLECT_GENERIC_SPECIALIZATION_ADAPTER_{method_index}_{specialization_index}"
    );
    let output = match return_type {
        None => quote! {
            <#target>::#method_name::<#(#generic_arguments),*>(#(#call_arguments),*);
            #facade::__private::codegen_v1::invoke::InvocationOutput::Unit
        },
        Some(_) => quote! {
            #facade::__private::codegen_v1::invoke::InvocationOutput::Owned(
                #facade::__private::codegen_v1::value::DynamicOwned::<#facade::__private::codegen_v1::value::Local>::new(
                    <#target>::#method_name::<#(#generic_arguments),*>(#(#call_arguments),*),
                ),
            )
        },
    };
    Some(quote! {
        fn #adapter_name<'call>(
            invocation: #facade::__private::codegen_v1::invoke::Invocation<'call, #facade::__private::codegen_v1::value::Local>,
        ) -> ::core::result::Result<
            #facade::__private::codegen_v1::invoke::InvocationOutput<'call, #facade::__private::codegen_v1::value::Local>,
            #facade::__private::codegen_v1::invoke::InvocationFailure<'call, #facade::__private::codegen_v1::value::Local>,
        > {
            let identity = #facade::__private::codegen_v1::identity::MemberId::new(
                #target_source,
                "method-specialization",
                #method_index,
                fragment_identity(),
            );
            let validated = invocation.validate(
                &identity,
                #facade::__private::codegen_v1::invoke::ReceiverExpectation::none(),
                &[#(#argument_expectations),*],
            )?;
            let (_receiver, arguments) = validated.into_parts();
            let mut arguments = arguments.into_vec().into_iter();
            #(#argument_bindings)*
            Ok(#output)
        }

        static #descriptor_name: #facade::__private::codegen_v1::descriptor::InvocationAdapter =
            #facade::__private::codegen_v1::descriptor::InvocationAdapter::local(#adapter_name);
    })
}

/// Resolves one named type argument from a validated specialization.
fn specialization_type_argument(
    specialization: &SpecializationIr,
    name: &str,
) -> Option<TokenStream> {
    match &specialization
        .bindings
        .iter()
        .find(|binding| binding.name == name)?
        .value
    {
        SpecializationValueIr::Type(ty) => Some(ty.tokens.clone()),
        SpecializationValueIr::AmbiguousPath(tokens) => Some(tokens.clone()),
        SpecializationValueIr::Const(_) => None,
    }
}

/// Resolves one named const argument from a validated specialization.
fn specialization_const_argument(
    specialization: &SpecializationIr,
    name: &str,
) -> Option<TokenStream> {
    match &specialization
        .bindings
        .iter()
        .find(|binding| binding.name == name)?
        .value
    {
        SpecializationValueIr::Const(tokens) | SpecializationValueIr::AmbiguousPath(tokens) => {
            Some(tokens.clone())
        }
        SpecializationValueIr::Type(_) => None,
    }
}

/// Recursively replaces method-generic type and const parameters in one type.
fn specialize_type_tokens(
    ty: &TypeIr,
    generics: &crate::ir::GenericsIr,
    specialization: &SpecializationIr,
) -> Option<TokenStream> {
    let mut replacements = Vec::new();
    for parameter in &generics.params {
        let tokens = match parameter.kind {
            GenericKindIr::Type => specialization_type_argument(specialization, &parameter.name),
            GenericKindIr::Const => specialization_const_argument(specialization, &parameter.name),
            GenericKindIr::Lifetime => None,
        }?;
        replacements.push((Ident::new(&parameter.name, parameter.span), tokens));
    }
    Some(substitute_tokens(&ty.tokens, &replacements))
}
