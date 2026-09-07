// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! AST-aware substitution of specialization type and const arguments.

use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use syn::Expr;
use syn::GenericArgument;
use syn::Path;
use syn::PathArguments;
use syn::Type;
use syn::parse2;
use syn::visit_mut::VisitMut;
use syn::visit_mut::visit_expr_mut;
use syn::visit_mut::visit_expr_path_mut;
use syn::visit_mut::visit_generic_argument_mut;
use syn::visit_mut::visit_type_mut;
use syn::visit_mut::visit_type_path_mut;

/// Substitutes generic symbols only in type and const expression positions.
pub(in crate::expand::impls) fn substitute_type_syntax(
    tokens: &TokenStream,
    replacements: &[(Ident, TokenStream)],
) -> TokenStream {
    let mut ty: Type = parse2(tokens.clone())
        .expect("validated specialization target must remain valid type syntax");
    GenericSubstituter { replacements }.visit_type_mut(&mut ty);
    ty.into_token_stream()
}

/// Substitutes generic symbols in a validated path.
pub(in crate::expand::impls) fn substitute_path_syntax(
    path: &Path,
    replacements: &[(Ident, TokenStream)],
) -> Path {
    let mut path = path.clone();
    GenericSubstituter { replacements }.visit_path_mut(&mut path);
    path
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
    fn visit_type_mut(&mut self, ty: &mut Type) {
        if let Type::Path(path) = ty
            && path.qself.is_none()
            && path.path.leading_colon.is_none()
            && !path.path.segments.is_empty()
            && matches!(path.path.segments[0].arguments, PathArguments::None)
            && let Some(replacement) = self.replacement(&path.path.segments[0].ident)
        {
            if path.path.segments.len() > 1 {
                let tail = Path {
                    leading_colon: None,
                    segments: path.path.segments.iter().skip(1).cloned().collect(),
                };
                if let Ok(mut replacement) = parse2::<Type>(quote!(#replacement :: #tail)) {
                    visit_type_mut(self, &mut replacement);
                    *ty = replacement;
                    return;
                }
            }
            let Ok(replacement) = parse2::<Type>(replacement.clone()) else {
                return;
            };
            if path.path.segments.len() == 1 {
                *ty = replacement;
                return;
            }
            if let Type::Path(replacement) = replacement
                && replacement.qself.is_none()
            {
                let tail = path.path.segments.iter().skip(1).cloned();
                let mut segments = replacement.path.segments;
                segments.extend(tail);
                path.path.leading_colon = replacement.path.leading_colon;
                path.path.segments = segments;
                visit_type_path_mut(self, path);
            }
            return;
        }
        visit_type_mut(self, ty);
    }

    fn visit_expr_mut(&mut self, expression: &mut Expr) {
        if let Expr::Path(path) = expression
            && path.qself.is_none()
            && path.path.leading_colon.is_none()
            && !path.path.segments.is_empty()
            && matches!(path.path.segments[0].arguments, PathArguments::None)
            && let Some(replacement) = self.replacement(&path.path.segments[0].ident)
        {
            if path.path.segments.len() > 1 {
                let tail = Path {
                    leading_colon: None,
                    segments: path.path.segments.iter().skip(1).cloned().collect(),
                };
                if let Ok(mut replacement) = parse2::<Expr>(quote!(#replacement :: #tail)) {
                    visit_expr_mut(self, &mut replacement);
                    *expression = replacement;
                    return;
                }
            }
            let Ok(replacement) = parse2::<Expr>(replacement.clone()) else {
                return;
            };
            if path.path.segments.len() == 1 {
                *expression = replacement;
                return;
            }
            if let Expr::Path(replacement) = replacement
                && replacement.qself.is_none()
            {
                let tail = path.path.segments.iter().skip(1).cloned();
                let mut segments = replacement.path.segments;
                segments.extend(tail);
                path.path.leading_colon = replacement.path.leading_colon;
                path.path.segments = segments;
                visit_expr_path_mut(self, path);
            }
            return;
        }
        visit_expr_mut(self, expression);
    }

    fn visit_generic_argument_mut(&mut self, argument: &mut GenericArgument) {
        if let GenericArgument::Type(Type::Path(path)) = argument
            && path.qself.is_none()
            && path.path.leading_colon.is_none()
            && path.path.segments.len() == 1
            && matches!(path.path.segments[0].arguments, PathArguments::None)
            && let Some(replacement) = self.replacement(&path.path.segments[0].ident)
            && let Ok(expression) = parse2::<Expr>(replacement.clone())
            && parse2::<Type>(replacement.clone()).is_err()
        {
            *argument = GenericArgument::Const(expression);
            return;
        }
        visit_generic_argument_mut(self, argument);
    }
}
