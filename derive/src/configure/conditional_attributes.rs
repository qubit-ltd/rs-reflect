// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Splits structural conditions and reflection helpers from source attributes.

use syn::Attribute;
use syn::Meta;
use syn::Result;
use syn::Token;
use syn::parse::Parser;
use syn::parse_quote;
use syn::punctuated::Punctuated;

/// Separates attributes projected onto the carrier from those retained on an
/// item.
///
/// # Parameters
///
/// - `attributes`: Original attributes on one trait or impl member.
///
/// # Returns
///
/// Returns `(carrier_attributes, retained_attributes)` in source order.
#[must_use]
pub(super) fn split_attributes(attributes: Vec<Attribute>) -> (Vec<Attribute>, Vec<Attribute>) {
    let mut projected = Vec::new();
    let mut retained = Vec::new();
    for attribute in attributes {
        match split_attribute(&attribute) {
            (Some(carrier), Some(item)) => {
                projected.push(carrier);
                retained.push(item);
            }
            (Some(carrier), None) => projected.push(carrier),
            (None, Some(item)) => retained.push(item),
            (None, None) => {}
        }
    }
    (projected, retained)
}

/// Splits one source attribute according to its reflection and cfg content.
///
/// # Parameters
///
/// - `attribute`: Source attribute to classify.
///
/// # Returns
///
/// Returns its carrier and retained forms, when either form is required.
#[must_use]
fn split_attribute(attribute: &Attribute) -> (Option<Attribute>, Option<Attribute>) {
    if attribute.path().is_ident("cfg") {
        return (Some(attribute.clone()), Some(attribute.clone()));
    }
    if attribute.path().is_ident("reflect") {
        return (Some(attribute.clone()), None);
    }
    if attribute.path().is_ident("cfg_attr") {
        let Ok((carrier, item)) = split_cfg_attr(attribute) else {
            return (Some(attribute.clone()), Some(attribute.clone()));
        };
        return (carrier, item);
    }
    (None, Some(attribute.clone()))
}

/// Splits nested cfg_attr branches without evaluating their predicates.
///
/// # Parameters
///
/// - `attribute`: A syntactically valid `cfg_attr` attribute.
///
/// # Returns
///
/// Returns cfg/reflection branches for the carrier and all other branches for
/// the original item.
///
/// # Errors
///
/// Returns a syntax error for malformed cfg_attr meta syntax.
fn split_cfg_attr(attribute: &Attribute) -> Result<(Option<Attribute>, Option<Attribute>)> {
    let Meta::List(list) = &attribute.meta else {
        return Ok((Some(attribute.clone()), Some(attribute.clone())));
    };
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let values = parser.parse2(list.tokens.clone())?;
    if values.len() < 2 {
        return Ok((Some(attribute.clone()), Some(attribute.clone())));
    }
    let Some(predicate) = values.first().cloned() else {
        return Ok((Some(attribute.clone()), Some(attribute.clone())));
    };
    let mut carrier = Vec::new();
    let mut item = Vec::new();
    for value in values.into_iter().skip(1) {
        let nested = split_meta(&value)?;
        if let Some(value) = nested.0 {
            carrier.push(value);
        }
        if let Some(value) = nested.1 {
            item.push(value);
        }
    }
    Ok((
        make_cfg_attr(attribute, &predicate, carrier)?,
        make_cfg_attr(attribute, &predicate, item)?,
    ))
}

/// Splits one nested meta item in a cfg_attr branch.
///
/// # Parameters
///
/// - `meta`: Nested attribute meta syntax.
///
/// # Returns
///
/// Returns its carrier and retained meta forms.
///
/// # Errors
///
/// Returns a syntax error for malformed nested cfg_attr syntax.
fn split_meta(meta: &Meta) -> Result<(Option<Meta>, Option<Meta>)> {
    if meta.path().is_ident("cfg") || meta.path().is_ident("reflect") {
        return Ok((Some(meta.clone()), None));
    }
    if meta.path().is_ident("cfg_attr") {
        let attribute: Attribute = parse_quote!(#[#meta]);
        let (carrier, item) = split_cfg_attr(&attribute)?;
        return Ok((
            carrier.map(|value| value.meta),
            item.map(|value| value.meta),
        ));
    }
    Ok((None, Some(meta.clone())))
}

/// Rebuilds a cfg_attr branch while retaining its original predicate.
///
/// # Parameters
///
/// - `source`: Original cfg_attr for source span provenance.
/// - `predicate`: Original condition meta item.
/// - `attributes`: Nested branches assigned to one output.
///
/// # Returns
///
/// Returns a cfg_attr attribute, or `None` when the branch is empty.
///
/// # Errors
///
/// Returns a syntax error when the reconstructed attribute is malformed.
fn make_cfg_attr(
    source: &Attribute,
    predicate: &Meta,
    attributes: Vec<Meta>,
) -> Result<Option<Attribute>> {
    if attributes.is_empty() {
        return Ok(None);
    }
    let mut rebuilt: Attribute = parse_quote!(#[cfg_attr(#predicate, #(#attributes),*)]);
    rebuilt.style = source.style;
    Ok(Some(rebuilt))
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::parse_quote;

    use super::split_attributes;

    #[test]
    fn test_cfg_and_reflect_helpers_are_projected() {
        let (carrier, retained) = split_attributes(vec![
            parse_quote!(#[cfg(unix)]),
            parse_quote!(#[reflect(rename = "active")]),
            parse_quote!(#[inline]),
        ]);
        assert_eq!(
            carrier.len(),
            2,
            "cfg and reflect attributes must be projected"
        );
        assert_eq!(
            retained.len(),
            2,
            "cfg and inline attributes must be retained"
        );
        assert_eq!(
            carrier[0].meta.to_token_stream().to_string(),
            "cfg (unix)",
            "cfg must be projected to the carrier",
        );
        assert_eq!(
            carrier[1].meta.to_token_stream().to_string(),
            "reflect (rename = \"active\")",
            "reflect policy must be projected to the carrier",
        );
        assert_eq!(
            retained[0].meta.to_token_stream().to_string(),
            "cfg (unix)",
            "cfg must remain on the original item",
        );
        assert_eq!(
            retained[1].meta.to_token_stream().to_string(),
            "inline",
            "item attributes must remain on the original item",
        );
    }

    #[test]
    fn test_nested_cfg_attr_splits_reflection_policy_and_item_attributes() {
        let (carrier, retained) = split_attributes(vec![parse_quote!(
            #[cfg_attr(feature = "special", cfg(any()), reflect(no_invoke), inline)]
        )]);
        assert_eq!(
            carrier.len(),
            1,
            "nested cfg branch must produce one carrier attribute"
        );
        assert_eq!(
            retained.len(),
            1,
            "nested item branch must produce one retained attribute"
        );
        assert!(
            carrier[0]
                .meta
                .to_token_stream()
                .to_string()
                .contains("cfg (any ())"),
            "nested cfg predicate must be projected to the carrier",
        );
        assert!(
            carrier[0]
                .meta
                .to_token_stream()
                .to_string()
                .contains("reflect (no_invoke)"),
            "reflection policy must be projected to the carrier",
        );
        assert!(
            retained[0]
                .meta
                .to_token_stream()
                .to_string()
                .contains("inline"),
            "item-only attributes must remain in the retained branch",
        );
        assert!(
            !retained[0]
                .meta
                .to_token_stream()
                .to_string()
                .contains("reflect"),
            "reflection policy must not remain on the original item",
        );
    }

    #[test]
    fn test_recursive_cfg_attr_preserves_each_predicate_and_attribute_order() {
        let (carrier, retained) = split_attributes(vec![parse_quote!(
            #[cfg_attr(feature = "outer", cfg_attr(target_os = "linux", reflect(no_invoke), inline), doc = "active")]
        )]);
        let carrier = carrier[0].meta.to_token_stream().to_string();
        let retained = retained[0].meta.to_token_stream().to_string();
        assert!(
            carrier.contains("feature = \"outer\""),
            "outer predicate must be preserved on the carrier"
        );
        assert!(
            carrier.contains("target_os = \"linux\""),
            "nested predicate must be preserved on the carrier"
        );
        assert!(
            carrier.contains("reflect (no_invoke)"),
            "nested reflection policy must be projected"
        );
        assert!(
            retained.contains("cfg_attr"),
            "retained nested attributes must keep cfg_attr"
        );
        assert!(
            retained.contains("inline"),
            "item attribute must be retained"
        );
        assert!(
            retained.contains("doc = \"active\""),
            "documentation attribute must be retained"
        );
    }
}
