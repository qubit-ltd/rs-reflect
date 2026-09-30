// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Conservative dyn-compatibility analysis helpers.

use proc_macro2::Ident;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::ToTokens;
use quote::format_ident;
use quote::quote;
use syn::FnArg;
use syn::GenericArgument;
use syn::GenericParam;
use syn::ItemTrait;
use syn::LitStr;
use syn::Path;
use syn::Receiver;
use syn::TraitBoundModifier;
use syn::TraitItem;
use syn::TraitItemFn;
use syn::TraitItemType;
use syn::Type;
use syn::TypeParamBound;
use syn::WhereClause;
use syn::parse_quote;
use syn::parse2;

use super::SynPathArguments;
use super::SynWherePredicate;
use super::replace_declared_lifetimes_with_static;
use crate::ir::HelperName;
use crate::ir::HelperValueIr;
use crate::ir::TraitDeclarationIr;

/// Returns inherited associated types explicitly proven for a dyn root.
///
/// # Parameters
///
/// - `declaration`: Validated trait declaration containing dyn-compatible
///   attributes.
///
/// # Returns
///
/// Returns each declared inherited associated type path in source order.
#[must_use]
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
///
/// # Parameters
///
/// - `path`: Parsed reflected supertrait path to augment.
/// - `declaration`: Validated declaration containing proven inherited bindings.
///
/// # Returns
///
/// Returns the generated supertrait path with explicit bindings and static
/// lifetimes.
#[must_use]
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
///
/// # Parameters
///
/// - `path`: Parsed external supertrait path.
/// - `declaration`: Validated declaration containing inherited bindings.
/// - `facade`: Runtime facade path used by generated references.
///
/// # Returns
///
/// Returns expressions describing applicable inherited associated types.
#[must_use]
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
///
/// # Parameters
///
/// - `inherited`: Fully qualified inherited associated item path.
/// - `supertrait`: Direct supertrait path to compare against.
///
/// # Returns
///
/// Returns whether the inherited path extends the direct supertrait by one
/// segment.
#[must_use]
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
///
/// # Parameters
///
/// - `item`: Parsed Rust trait item.
/// - `declaration`: Validated trait metadata and helper attributes.
///
/// # Returns
///
/// Returns whether this trait can be treated as dyn-compatible by this
/// analysis.
#[must_use]
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
///
/// # Parameters
///
/// - `bound`: Rust supertrait bound to inspect.
///
/// # Returns
///
/// Returns whether the bound is on the locally recognized safe list.
#[must_use]
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
///
/// # Parameters
///
/// - `associated`: Parsed associated type declaration.
///
/// # Returns
///
/// Returns whether the associated type requires a dyn binding.
#[must_use]
pub(super) fn associated_type_requires_dyn_binding(associated: &TraitItemType) -> bool {
    !where_clause_requires_sized_self(associated.generics.where_clause.as_ref())
}

/// Returns whether one method is dispatchable through a trait object or is
/// explicitly excluded from the vtable by `Self: Sized`.
///
/// # Parameters
///
/// - `method`: Parsed trait method declaration.
///
/// # Returns
///
/// Returns whether the method is valid on a dyn-dispatchable trait object.
#[must_use]
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
///
/// # Parameters
///
/// - `receiver`: Parsed method receiver.
///
/// # Returns
///
/// Returns whether the receiver syntax is supported for trait-object dispatch.
#[must_use]
pub(super) fn receiver_is_dyn_dispatchable(receiver: &Receiver) -> bool {
    if receiver.colon_token.is_none() {
        return true;
    }
    receiver_type_is_dyn_dispatchable(&receiver.ty)
}

/// Checks explicit `Self`, reference, smart-pointer, and pinned receiver types.
///
/// # Parameters
///
/// - `ty`: Explicit receiver type to inspect recursively.
///
/// # Returns
///
/// Returns whether the type resolves to a supported trait-object receiver.
#[must_use]
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
///
/// # Parameters
///
/// - `tokens`: Token stream to scan recursively.
///
/// # Returns
///
/// Returns whether an unprojected `Self` identifier occurs.
#[must_use]
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
///
/// # Parameters
///
/// - `where_clause`: Optional parsed where clause.
///
/// # Returns
///
/// Returns whether the clause excludes the method or type from trait objects.
#[must_use]
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
///
/// # Parameters
///
/// - `tokens`: Token stream to scan recursively.
///
/// # Returns
///
/// Returns whether a standalone `Self` identifier occurs.
#[must_use]
pub(super) fn tokens_contain_self(tokens: TokenStream) -> bool {
    tokens_contain_ident(tokens, "Self")
}

/// Returns whether a token stream contains one standalone identifier.
///
/// # Parameters
///
/// - `tokens`: Token stream to scan recursively.
/// - `expected`: Identifier spelling to find.
///
/// # Returns
///
/// Returns whether the identifier occurs outside literal and punctuation
/// tokens.
#[must_use]
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
    use syn::Path;
    use syn::parse_str;
    #[test]
    fn test_inherited_binding_and_projection_are_analyzed_in_one_module() {
        let supertrait = crate::parse::convert_path(&parse_str::<Path>("Base").unwrap());
        let inherited = crate::parse::convert_path(&parse_str::<Path>("Base::Assoc").unwrap());
        assert!(
            super::inherited_belongs_to_supertrait(&inherited, &supertrait),
            "Base::Assoc must identify an associated item on the direct Base supertrait"
        );
        assert!(
            !super::tokens_contain_unprojected_self(quote!(Self::Assoc)),
            "Self::Assoc is a projection and must not count as unprojected Self"
        );
    }
}
