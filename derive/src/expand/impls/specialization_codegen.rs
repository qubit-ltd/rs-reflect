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
use quote::ToTokens;
use quote::quote;
use syn::visit_mut::VisitMut;

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
    let codegen = quote!(#facade::__private::codegen_v2);
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
                        -> Option<#facade::__private::codegen_v2::descriptor::TypeDescriptorResolver>
                        #where_clause
                    {
                        use #codegen::descriptor::ResolveReflectTypeDescriptor as _;
                        let probe = #facade::__private::codegen_v2::descriptor::ReflectArgumentProbe::<#value>::new();
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

/// Substitutes generic symbols only where the Rust AST identifies a type or
/// const expression path. Member names, labels, patterns, and unrelated token
/// identifiers are never rewritten.
pub(super) fn substitute_type_syntax(
    tokens: &TokenStream,
    replacements: &[(Ident, TokenStream)],
) -> TokenStream {
    let mut ty: syn::Type = syn::parse2(tokens.clone())
        .expect("validated specialization target must remain valid type syntax");
    GenericSubstituter { replacements }.visit_type_mut(&mut ty);
    ty.into_token_stream()
}

struct GenericSubstituter<'a> {
    replacements: &'a [(Ident, TokenStream)],
}

impl GenericSubstituter<'_> {
    fn replacement(&self, identifier: &Ident) -> Option<&TokenStream> {
        self.replacements
            .iter()
            .find_map(|(name, value)| (name == identifier).then_some(value))
    }
}

impl VisitMut for GenericSubstituter<'_> {
    fn visit_type_mut(&mut self, ty: &mut syn::Type) {
        if let syn::Type::Path(path) = ty
            && path.qself.is_none()
            && path.path.leading_colon.is_none()
            && !path.path.segments.is_empty()
            && matches!(path.path.segments[0].arguments, syn::PathArguments::None)
            && let Some(replacement) = self.replacement(&path.path.segments[0].ident)
        {
            if path.path.segments.len() > 1 {
                let tail = syn::Path {
                    leading_colon: None,
                    segments: path.path.segments.iter().skip(1).cloned().collect(),
                };
                if let Ok(mut replacement) = syn::parse2::<syn::Type>(quote!(#replacement :: #tail))
                {
                    syn::visit_mut::visit_type_mut(self, &mut replacement);
                    *ty = replacement;
                    return;
                }
            }
            let Ok(replacement) = syn::parse2::<syn::Type>(replacement.clone()) else {
                return;
            };
            if path.path.segments.len() == 1 {
                *ty = replacement;
                return;
            }
            if let syn::Type::Path(replacement) = replacement
                && replacement.qself.is_none()
            {
                let tail = path.path.segments.iter().skip(1).cloned();
                let mut segments = replacement.path.segments;
                segments.extend(tail);
                path.path.leading_colon = replacement.path.leading_colon;
                path.path.segments = segments;
                syn::visit_mut::visit_type_path_mut(self, path);
            }
            return;
        }
        syn::visit_mut::visit_type_mut(self, ty);
    }

    fn visit_expr_mut(&mut self, expression: &mut syn::Expr) {
        if let syn::Expr::Path(path) = expression
            && path.qself.is_none()
            && path.path.leading_colon.is_none()
            && !path.path.segments.is_empty()
            && matches!(path.path.segments[0].arguments, syn::PathArguments::None)
            && let Some(replacement) = self.replacement(&path.path.segments[0].ident)
        {
            if path.path.segments.len() > 1 {
                let tail = syn::Path {
                    leading_colon: None,
                    segments: path.path.segments.iter().skip(1).cloned().collect(),
                };
                if let Ok(mut replacement) = syn::parse2::<syn::Expr>(quote!(#replacement :: #tail))
                {
                    syn::visit_mut::visit_expr_mut(self, &mut replacement);
                    *expression = replacement;
                    return;
                }
            }
            let Ok(replacement) = syn::parse2::<syn::Expr>(replacement.clone()) else {
                return;
            };
            if path.path.segments.len() == 1 {
                *expression = replacement;
                return;
            }
            if let syn::Expr::Path(replacement) = replacement
                && replacement.qself.is_none()
            {
                let tail = path.path.segments.iter().skip(1).cloned();
                let mut segments = replacement.path.segments;
                segments.extend(tail);
                path.path.leading_colon = replacement.path.leading_colon;
                path.path.segments = segments;
                syn::visit_mut::visit_expr_path_mut(self, path);
            }
            return;
        }
        syn::visit_mut::visit_expr_mut(self, expression);
    }

    fn visit_generic_argument_mut(&mut self, argument: &mut syn::GenericArgument) {
        if let syn::GenericArgument::Type(syn::Type::Path(path)) = argument
            && path.qself.is_none()
            && path.path.leading_colon.is_none()
            && path.path.segments.len() == 1
            && matches!(path.path.segments[0].arguments, syn::PathArguments::None)
            && let Some(replacement) = self.replacement(&path.path.segments[0].ident)
            && let Ok(expression) = syn::parse2::<syn::Expr>(replacement.clone())
            && syn::parse2::<syn::Type>(replacement.clone()).is_err()
        {
            *argument = syn::GenericArgument::Const(expression);
            return;
        }
        syn::visit_mut::visit_generic_argument_mut(self, argument);
    }
}

/// Replaces impl generic references in method signature types while retaining
/// their structural IR for descriptor rendering.
pub(super) fn substitute_impl_method_types(
    declaration: &mut ImplDeclarationIr,
    replacements: &[(Ident, TokenStream)],
) {
    for method in &mut declaration.methods {
        let replacements = replacements
            .iter()
            .filter(|(name, _)| {
                !method
                    .generics
                    .params
                    .iter()
                    .any(|parameter| name == parameter.name.as_str())
            })
            .cloned()
            .collect::<Vec<_>>();
        if let Some(receiver) = &mut method.receiver {
            substitute_type_tokens(&mut receiver.ty, &replacements);
        }
        for parameter in &mut method.parameters {
            substitute_type_tokens(&mut parameter.ty, &replacements);
        }
        if let ReturnTypeIr::Type(ty) = &mut method.return_type {
            substitute_type_tokens(ty, &replacements);
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
    let tokens = substitute_type_syntax(&ty.tokens, replacements);
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
    reparse_substituted_type(ty, replacements);
}

/// Rebuilds a trait path after substituting bound impl generic parameters.
pub(super) fn substitute_trait_path_tokens(
    path: &mut crate::ir::PathIr,
    replacements: &[(Ident, TokenStream)],
) {
    let mut parsed: syn::Path = syn::parse2(path.tokens.clone())
        .expect("validated specialization must retain valid trait path syntax");
    GenericSubstituter { replacements }.visit_path_mut(&mut parsed);
    *path = crate::parse::convert_path(&parsed);
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
    Some(substitute_type_syntax(
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
    let environment =
        crate::expand::generic_environment::GenericEnvironment::from_generics(generics);
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
                let expression =
                    crate::expand::traits::type_expression(ty, &environment, facade);
                Some(quote!(#facade::__private::codegen_v2::expression::GenericArgument::Type(#expression)))
            }
            (GenericKindIr::Type, SpecializationValueIr::AmbiguousPath(tokens)) => Some(quote!(
                #facade::__private::codegen_v2::expression::GenericArgument::Type(
                    #facade::__private::codegen_v2::expression::TypeExpression::Concrete(
                        #facade::__private::codegen_v2::expression::concrete(
                            vec![stringify!(#tokens).into()].into_boxed_slice(),
                            vec![].into_boxed_slice(),
                            #facade::__private::codegen_v2::expression::DiagnosticText::from(stringify!(#tokens)),
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
                    #facade::__private::codegen_v2::expression::GenericArgument::Const(
                        #facade::__private::codegen_v2::expression::ConstGenericArgument::new(
                            #facade::__private::codegen_v2::expression::TypeExpression::Concrete(
                                #facade::__private::codegen_v2::expression::concrete(
                                    vec![#declared_type_literal.into()].into_boxed_slice(),
                                    vec![].into_boxed_slice(),
                                    #facade::__private::codegen_v2::expression::DiagnosticText::from(#declared_type_literal),
                                ),
                            ),
                            #facade::__private::codegen_v2::expression::const_path([
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

/// Materializes one method specialization as concrete method IR and call
/// arguments. The ordinary invocation analyzer and emitter can then make the
/// same availability decision as they do for a non-generic method.
pub(super) fn specialized_method(
    method: &MethodIr,
    specialization: &SpecializationIr,
) -> Option<(MethodIr, Vec<TokenStream>)> {
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
    let replacements = specialization_replacements(specialization);
    let mut concrete = method.clone();
    if let Some(receiver) = &mut concrete.receiver {
        substitute_type_tokens(&mut receiver.ty, &replacements);
    }
    for parameter in &mut concrete.parameters {
        substitute_type_tokens(&mut parameter.ty, &replacements);
    }
    if let ReturnTypeIr::Type(ty) = &mut concrete.return_type {
        substitute_type_tokens(ty, &replacements);
    }
    concrete
        .generics
        .params
        .retain(|parameter| parameter.kind == GenericKindIr::Lifetime);
    concrete.specializations.clear();
    Some((concrete, generic_arguments))
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
