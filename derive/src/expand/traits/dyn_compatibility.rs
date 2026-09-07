// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Conservative dyn-compatibility analysis helpers.

use super::{SynPathArguments, SynWherePredicate, replace_declared_lifetimes_with_static};
use crate::ir::{HelperName, HelperValueIr, TraitDeclarationIr};
use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote};
use syn::LitStr;
use syn::{
    FnArg, GenericArgument, GenericParam, ItemTrait, Path, Receiver, TraitBoundModifier, TraitItem,
    TraitItemFn, TraitItemType, Type, TypeParamBound, WhereClause,
};
use syn::{parse_quote, parse2};

/// Returns inherited associated types explicitly proven for a dyn root.
pub(super) fn dyn_inherited_associated_types(
    declaration: &TraitDeclarationIr,
) -> impl Iterator<Item = &crate::ir::PathIr> {
    declaration
        .attributes
        .iter()
        .filter_map(|attribute| match &attribute.value {
            HelperValueIr::DynCompatible(paths) => Some(paths.iter()),
            _ => None,
        })
        .flatten()
}

/// Adds explicit inherited bindings to one reflected dyn supertrait path.
pub(super) fn dyn_reflected_supertrait_path(
    path: &crate::ir::PathIr,
    declaration: &TraitDeclarationIr,
) -> TokenStream {
    let mut syntax: Path = parse2(path.tokens.clone())
        .expect("validated reflected supertrait paths must parse as Rust paths");
    for inherited in dyn_inherited_associated_types(declaration)
        .filter(|inherited| inherited_belongs_to_supertrait(inherited, path))
    {
        let name = Ident::new(
            &inherited
                .segments
                .last()
                .expect("validated inherited associated path has an item")
                .name,
            declaration.span,
        );
        let parameter = format_ident!("__QubitReflectAssociated{}", name);
        let segment = syntax
            .segments
            .last_mut()
            .expect("validated supertrait path has a segment");
        match &mut segment.arguments {
            SynPathArguments::None => {
                segment.arguments = SynPathArguments::AngleBracketed(parse_quote!(
                    <#name = #parameter>
                ));
            }
            SynPathArguments::AngleBracketed(arguments) => {
                arguments.args.push(parse_quote!(#name = #parameter));
            }
            SynPathArguments::Parenthesized(_) => {}
        }
    }
    let syntax = if syntax.leading_colon.is_some() {
        quote!(#syntax)
    } else {
        quote!(super::#syntax)
    };
    replace_declared_lifetimes_with_static(syntax, declaration)
}

/// Builds inherited associated bindings for an external supertrait identity.
pub(super) fn dyn_inherited_arguments_for_supertrait(
    path: &crate::ir::PathIr,
    declaration: &TraitDeclarationIr,
    facade: &TokenStream,
) -> Vec<TokenStream> {
    dyn_inherited_associated_types(declaration)
        .filter(|inherited| inherited_belongs_to_supertrait(inherited, path))
        .map(|inherited| {
            let name = inherited
                .segments
                .last()
                .expect("validated inherited associated path has an item")
                .name
                .clone();
            let name_literal = LitStr::new(&name, declaration.span);
            let parameter = format_ident!("__QubitReflectAssociated{name}");
            quote!(#facade::__private::codegen_v3::expression::associated_type(
                #name_literal,
                #facade::__private::codegen_v3::expression::TypeExpression::Concrete(
                    #facade::__private::codegen_v3::expression::concrete(
                        vec![std::any::type_name::<#parameter>().into()].into_boxed_slice(),
                        vec![].into_boxed_slice(),
                        #facade::__private::codegen_v3::expression::DiagnosticText::from(std::any::type_name::<#parameter>()),
                    ),
                ),
            ))
        })
        .collect()
}

/// Returns whether `Supertrait::Item` names an item on this direct bound.
pub(super) fn inherited_belongs_to_supertrait(
    inherited: &crate::ir::PathIr,
    supertrait: &crate::ir::PathIr,
) -> bool {
    inherited.segments.len() == supertrait.segments.len() + 1
        && inherited
            .segments
            .iter()
            .zip(&supertrait.segments)
            .all(|(left, right)| left.name == right.name)
}

/// Returns whether the declaration is syntactically proven to admit a bare
/// `'static` trait object.
///
/// Generic traits are deliberately excluded because the requirements do not
/// define which concrete application a declaration-level macro should choose.
/// Supertraits are limited to standard traits whose dyn compatibility is known
/// without inspecting another macro expansion.
pub(super) fn is_provably_dyn_compatible(
    item: &ItemTrait,
    declaration: &TraitDeclarationIr,
) -> bool {
    if declaration
        .attributes
        .iter()
        .any(|attribute| attribute.name == HelperName::DynCompatible)
    {
        return true;
    }
    if where_clause_requires_sized_self(item.generics.where_clause.as_ref())
        || item
            .generics
            .where_clause
            .as_ref()
            .is_some_and(|clause| tokens_contain_unprojected_self(clause.to_token_stream()))
        || !item.supertraits.iter().all(is_known_dyn_compatible_bound)
    {
        return false;
    }
    item.items.iter().all(|trait_item| match trait_item {
        TraitItem::Fn(method) => method_is_dyn_dispatchable(method),
        TraitItem::Type(associated) => {
            associated.generics.params.is_empty()
                || where_clause_requires_sized_self(associated.generics.where_clause.as_ref())
        }
        TraitItem::Const(_) => false,
        _ => false,
    })
}

/// Returns whether one supertrait bound is known locally to preserve dyn
/// compatibility.
pub(super) fn is_known_dyn_compatible_bound(bound: &TypeParamBound) -> bool {
    match bound {
        TypeParamBound::Lifetime(_) => true,
        TypeParamBound::Trait(bound) => {
            if !matches!(bound.modifier, TraitBoundModifier::None)
                || tokens_contain_self(bound.to_token_stream())
            {
                return false;
            }
            let path = bound.path.to_token_stream().to_string().replace(' ', "");
            if matches!(
                path.as_str(),
                "Sized"
                    | "std::marker::Sized"
                    | "core::marker::Sized"
                    | "::std::marker::Sized"
                    | "::core::marker::Sized"
            ) {
                return false;
            }
            matches!(
                path.as_str(),
                "::std::fmt::Debug"
                    | "::core::fmt::Debug"
                    | "std::fmt::Debug"
                    | "core::fmt::Debug"
                    | "::std::fmt::Display"
                    | "::core::fmt::Display"
                    | "std::fmt::Display"
                    | "core::fmt::Display"
                    | "::std::marker::Send"
                    | "::core::marker::Send"
                    | "std::marker::Send"
                    | "core::marker::Send"
                    | "::std::marker::Sync"
                    | "::core::marker::Sync"
                    | "std::marker::Sync"
                    | "core::marker::Sync"
                    | "::std::marker::Unpin"
                    | "::core::marker::Unpin"
                    | "std::marker::Unpin"
                    | "core::marker::Unpin"
            )
        }
        _ => false,
    }
}

/// Returns whether a dyn application must name a concrete binding for this
/// associated type.
pub(super) fn associated_type_requires_dyn_binding(associated: &TraitItemType) -> bool {
    !where_clause_requires_sized_self(associated.generics.where_clause.as_ref())
}

/// Returns whether one method is dispatchable through a trait object or is
/// explicitly excluded from the vtable by `Self: Sized`.
pub(super) fn method_is_dyn_dispatchable(method: &TraitItemFn) -> bool {
    if where_clause_requires_sized_self(method.sig.generics.where_clause.as_ref()) {
        return true;
    }
    if method.sig.asyncness.is_some()
        || method
            .sig
            .generics
            .params
            .iter()
            .any(|parameter| !matches!(parameter, GenericParam::Lifetime(_)))
    {
        return false;
    }
    if method
        .sig
        .generics
        .where_clause
        .as_ref()
        .is_some_and(|clause| tokens_contain_unprojected_self(clause.to_token_stream()))
    {
        return false;
    }
    let Some(FnArg::Receiver(receiver)) = method.sig.inputs.first() else {
        return false;
    };
    if !receiver_is_dyn_dispatchable(receiver) {
        return false;
    }
    method.sig.inputs.iter().skip(1).all(|input| {
        let tokens = input.to_token_stream();
        !tokens_contain_unprojected_self(tokens.clone()) && !tokens_contain_ident(tokens, "impl")
    }) && {
        let output = method.sig.output.to_token_stream();
        !tokens_contain_unprojected_self(output.clone()) && !tokens_contain_ident(output, "impl")
    }
}

/// Returns whether a method receiver is one of Rust's dyn-dispatchable forms.
pub(super) fn receiver_is_dyn_dispatchable(receiver: &Receiver) -> bool {
    if receiver.colon_token.is_none() {
        return true;
    }
    receiver_type_is_dyn_dispatchable(&receiver.ty)
}

/// Checks explicit `Self`, reference, smart-pointer, and pinned receiver types.
pub(super) fn receiver_type_is_dyn_dispatchable(ty: &Type) -> bool {
    match ty {
        Type::Path(path) if path.qself.is_none() && path.path.is_ident("Self") => true,
        Type::Reference(reference) => receiver_type_is_dyn_dispatchable(&reference.elem),
        Type::Path(path) if path.qself.is_none() => {
            let Some(segment) = path.path.segments.last() else {
                return false;
            };
            if !matches!(
                segment.ident.to_string().as_str(),
                "Box" | "Rc" | "Arc" | "Pin"
            ) {
                return false;
            }
            let SynPathArguments::AngleBracketed(arguments) = &segment.arguments else {
                return false;
            };
            let mut types = arguments.args.iter().filter_map(|argument| match argument {
                GenericArgument::Type(ty) => Some(ty),
                _ => None,
            });
            let Some(inner) = types.next() else {
                return false;
            };
            types.next().is_none() && receiver_type_is_dyn_dispatchable(inner)
        }
        _ => false,
    }
}

/// Returns whether `Self` occurs outside an associated-type projection.
pub(super) fn tokens_contain_unprojected_self(tokens: TokenStream) -> bool {
    let tokens: Vec<_> = tokens.into_iter().collect();
    tokens.iter().enumerate().any(|(index, token)| match token {
        TokenTree::Group(group) => tokens_contain_unprojected_self(group.stream()),
        TokenTree::Ident(identifier) if identifier == "Self" => !matches!(
            tokens.get(index + 1..index + 4),
            Some([
                TokenTree::Punct(first),
                TokenTree::Punct(second),
                TokenTree::Ident(_),
            ]) if first.as_char() == ':' && second.as_char() == ':'
        ),
        _ => false,
    })
}

/// Returns whether a where clause contains a direct `Self: Sized` predicate.
pub(super) fn where_clause_requires_sized_self(where_clause: Option<&WhereClause>) -> bool {
    where_clause.is_some_and(|where_clause| {
        where_clause.predicates.iter().any(|predicate| {
            let SynWherePredicate::Type(predicate) = predicate else {
                return false;
            };
            matches!(predicate.bounded_ty, Type::Path(ref path) if path.qself.is_none() && path.path.is_ident("Self"))
                && predicate.bounds.iter().any(|bound| {
                    matches!(bound, TypeParamBound::Trait(bound)
                        if matches!(bound.modifier, TraitBoundModifier::None)
                            && bound.path.is_ident("Sized"))
                })
        })
    })
}

/// Returns whether a token stream contains the standalone `Self` type name.
pub(super) fn tokens_contain_self(tokens: TokenStream) -> bool {
    tokens_contain_ident(tokens, "Self")
}

/// Returns whether a token stream contains one standalone identifier.
pub(super) fn tokens_contain_ident(tokens: TokenStream, expected: &str) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Ident(identifier) => identifier == expected,
        TokenTree::Group(group) => tokens_contain_ident(group.stream(), expected),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

#[cfg(test)]
mod tests {
    use quote::quote;
    #[test]
    fn inherited_binding_and_projection_are_analyzed_in_one_module() {
        let supertrait = crate::parse::convert_path(&syn::parse_str::<syn::Path>("Base").unwrap());
        let inherited =
            crate::parse::convert_path(&syn::parse_str::<syn::Path>("Base::Assoc").unwrap());
        assert!(super::inherited_belongs_to_supertrait(
            &inherited,
            &supertrait
        ));
        assert!(!super::tokens_contain_unprojected_self(quote!(Self::Assoc)));
    }
}
