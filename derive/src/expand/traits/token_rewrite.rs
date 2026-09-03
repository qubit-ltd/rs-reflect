// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Span-preserving token rewrites for generated trait provider types.

use proc_macro2::Group;
use proc_macro2::Ident;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;

/// Replaces `Self` with an explicit owner while retaining grouping and spans.
pub(super) fn replace_self_with_owner(tokens: TokenStream, owner: &Ident) -> TokenStream {
    tokens
        .into_iter()
        .map(|tree| match tree {
            TokenTree::Ident(identifier) if identifier == "Self" => {
                TokenTree::Ident(Ident::new(&owner.to_string(), identifier.span()))
            }
            TokenTree::Group(group) => {
                let mut replacement = Group::new(
                    group.delimiter(),
                    replace_self_with_owner(group.stream(), owner),
                );
                replacement.set_span(group.span());
                TokenTree::Group(replacement)
            }
            other => other,
        })
        .collect()
}
