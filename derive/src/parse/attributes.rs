// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Parsing for the shared `reflect` helper grammar.

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Attribute;
use syn::Error;
use syn::Meta;
use syn::Token;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::internal::ErrorCollector;
use crate::ir::HelperAttributeIr;
use crate::ir::HelperName;
use crate::ir::HelperTarget;
use crate::ir::HelperValueIr;

/// Parses helper attributes from a source attribute slice.
///
/// # Parameters
///
/// - `attributes`: Attributes attached to one Rust declaration.
/// - `target`: Declaration element carrying the attributes.
/// - `errors`: Collector for independent syntax diagnostics.
///
/// # Returns
///
/// Returns every successfully parsed reflection helper.
#[must_use]
pub(super) fn parse_attributes(
    attributes: &[Attribute],
    target: HelperTarget,
    errors: &mut ErrorCollector,
) -> Vec<HelperAttributeIr> {
    let mut helpers = Vec::new();
    for attribute in attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("reflect"))
    {
        match &attribute.meta {
            Meta::Path(_) => errors.push(Error::new(
                attribute.span(),
                "bare `#[reflect]` is not a valid helper attribute",
            )),
            Meta::List(list) => {
                helpers.extend(parse_helper_tokens(list.tokens.clone(), target, errors));
            }
            Meta::NameValue(_) => errors.push(Error::new(
                attribute.span(),
                "`reflect` helpers must use `#[reflect(...)]` syntax",
            )),
        }
    }
    helpers
}

/// Parses the comma-separated arguments of a macro or helper attribute.
///
/// # Parameters
///
/// - `tokens`: Attribute argument tokens.
/// - `target`: Declaration element carrying the helper.
/// - `errors`: Collector for syntax diagnostics.
///
/// # Returns
///
/// Returns every successfully converted helper argument.
#[must_use]
pub(super) fn parse_helper_tokens(
    tokens: TokenStream,
    target: HelperTarget,
    errors: &mut ErrorCollector,
) -> Vec<HelperAttributeIr> {
    if tokens.is_empty() {
        return Vec::new();
    }
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let metas = match parser.parse2(tokens) {
        Ok(metas) => metas,
        Err(error) => {
            errors.push(error);
            return Vec::new();
        }
    };
    metas
        .into_iter()
        .filter_map(|meta| convert_meta(meta, target, errors))
        .collect()
}

/// Converts one `syn::Meta` node into helper IR.
///
/// # Parameters
///
/// - `meta`: Parsed helper metadata.
/// - `target`: Declaration element carrying the helper.
/// - `errors`: Collector for conversion diagnostics.
///
/// # Returns
///
/// Returns the converted helper when its key and value are valid.
#[must_use]
fn convert_meta(
    meta: Meta,
    target: HelperTarget,
    errors: &mut ErrorCollector,
) -> Option<HelperAttributeIr> {
    let span = meta.span();
    let path_text = meta.path().to_token_stream().to_string();
    let Some(source_name) = meta.path().get_ident().map(ToString::to_string) else {
        errors.push(Error::new(
            span,
            format!("unknown reflection helper `{path_text}`"),
        ));
        return None;
    };
    let Some(name) = HelperName::from_str(&source_name) else {
        errors.push(Error::new(
            span,
            format!("unknown reflection helper `{source_name}`"),
        ));
        return None;
    };
    if name != HelperName::Skip {
        errors.push(Error::new(
            span,
            format!("reflection helper `{source_name}` is unsupported; only `skip` is accepted"),
        ));
        return None;
    }
    let value = match (&name, &meta) {
        (HelperName::Skip, Meta::Path(_)) => Ok(HelperValueIr::Flag),
        (HelperName::Skip, _) => Err(Error::new(span, "`skip` does not accept a value")),
        _ => unreachable!("all non-skip helpers are rejected before value parsing"),
    };
    match value {
        Ok(value) => {
            let value_span = match &meta {
                Meta::NameValue(name_value) => name_value.value.span(),
                Meta::List(list) => list.span(),
                Meta::Path(path) => path.span(),
            };
            Some(HelperAttributeIr {
                name,
                value,
                target,
                span,
                value_span,
            })
        }
        Err(error) => {
            errors.push(error);
            None
        }
    }
}

/// Removes reflection helper attributes before an attribute macro returns its
/// item.
///
/// # Parameters
///
/// - `attributes`: Mutable attribute list on the returned syntax item.
pub(super) fn remove_reflect_attributes(attributes: &mut Vec<Attribute>) {
    attributes.retain(|attribute| !attribute.path().is_ident("reflect"));
}
