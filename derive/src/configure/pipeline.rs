// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Compiler-assisted conditional configuration for reflected traits and impls.

use std::mem::take;

use proc_macro::TokenStream as CompilerTokenStream;
use proc_macro_crate::FoundCrate;
use proc_macro_crate::crate_name;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Attribute;
use syn::Data;
use syn::DeriveInput;
use syn::Error;
use syn::Expr;
use syn::Field;
use syn::Fields;
use syn::FieldsNamed;
use syn::Ident;
use syn::ImplItem;
use syn::Item;
use syn::ItemImpl;
use syn::ItemStruct;
use syn::ItemTrait;
use syn::Meta;
use syn::Path;
use syn::Result;
use syn::Token;
use syn::TraitItem;
use syn::parse::Parser;
use syn::parse_quote;
use syn::parse2;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::configure::carrier::carrier_name;
use crate::configure::conditional_attributes;
use crate::entry::process_macro;
use crate::ir::MacroKind;

/// Produces a carrier whose members are filtered by rustc before reflection.
///
/// # Parameters
///
/// - `kind`: Trait or impl attribute macro kind.
/// - `arguments`: Original attribute arguments.
/// - `item`: Original trait or impl item.
///
/// # Returns
///
/// Returns the compiler-filtered carrier or a source-oriented diagnostic.
#[must_use]
pub(crate) fn configure_attribute(
    kind: MacroKind,
    arguments: CompilerTokenStream,
    item: CompilerTokenStream,
) -> CompilerTokenStream {
    match make_carrier(kind, arguments.into(), item.into()) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

/// Reconstructs and processes one compiler-filtered carrier.
///
/// # Parameters
///
/// - `input`: Internal derive input after rustc applied conditional attributes.
///
/// # Returns
///
/// Returns reflected declaration tokens or compiler diagnostics.
#[must_use]
pub(crate) fn process_configured(input: CompilerTokenStream) -> CompilerTokenStream {
    match expand_configured(input.into()) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

/// Builds a carrier declaration for one reflected trait or impl.
///
/// # Parameters
///
/// - `kind`: Macro kind selecting the declaration syntax.
/// - `arguments`: Outer reflection macro arguments.
/// - `item`: Original declaration token stream.
///
/// # Returns
///
/// Returns carrier tokens for rustc to configure before the internal derive.
///
/// # Errors
///
/// Returns a syntax diagnostic when the input or support macro path is invalid.
pub(super) fn make_carrier(
    kind: MacroKind,
    arguments: TokenStream,
    item: TokenStream,
) -> Result<TokenStream> {
    let source_tokens = item.clone();
    let (header, members, outer_attributes) = match kind {
        MacroKind::Trait => {
            let mut declaration: ItemTrait = parse2(item)?;
            let mut members = Vec::with_capacity(declaration.items.len());
            for mut member in take(&mut declaration.items) {
                let Some(attributes) = trait_item_attributes_mut(&mut member) else {
                    members.push((quote!(#member), Vec::new()));
                    continue;
                };
                let (projected, retained) =
                    conditional_attributes::split_attributes(attributes.clone());
                if let Some(attributes) = trait_item_attributes_mut(&mut member) {
                    *attributes = retained;
                }
                members.push((quote!(#member), projected));
            }
            let attrs = declaration.attrs.clone();
            (quote!(#declaration), members, attrs)
        }
        MacroKind::Impl => {
            let Item::Impl(mut declaration) = parse2(item.clone())? else {
                return Err(Error::new_spanned(
                    item,
                    "`#[reflect_impl]` can only be applied to an impl block",
                ));
            };
            let mut members = Vec::with_capacity(declaration.items.len());
            for mut member in take(&mut declaration.items) {
                let Some(attributes) = impl_item_attributes_mut(&mut member) else {
                    members.push((quote!(#member), Vec::new()));
                    continue;
                };
                let (projected, retained) =
                    conditional_attributes::split_attributes(attributes.clone());
                if let Some(attributes) = impl_item_attributes_mut(&mut member) {
                    *attributes = retained;
                }
                members.push((quote!(#member), projected));
            }
            let attrs = declaration.attrs.clone();
            (quote!(#declaration), members, attrs)
        }
        MacroKind::Derive => {
            return Err(Error::new(
                Span::call_site(),
                "conditional carrier only supports trait and impl macros",
            ));
        }
    };
    let mut fields = Vec::with_capacity(members.len());
    for (ordinal, (member_tokens, projected)) in members.into_iter().enumerate() {
        let marker: Attribute = parse_quote!(#[reflect_configure_member(#member_tokens)]);
        let name = Ident::new(&format!("__member_{ordinal}"), Span::call_site());
        let field: Field = parse_quote!(#(#projected)* #marker #name: ());
        fields.push(field);
    }
    let support = support_path(&arguments)?;
    let name = carrier_name(&source_tokens, &arguments);
    let header_attribute: Attribute = parse_quote!(#[reflect_configure_header(#header)]);
    let argument_attribute: Attribute = parse_quote!(#[reflect_configure_args(#arguments)]);
    let kind_name = match kind {
        MacroKind::Trait => quote!(trait),
        MacroKind::Impl => quote!(implementation),
        MacroKind::Derive => unreachable!(),
    };
    let kind_attribute: Attribute = parse_quote!(#[reflect_configure_kind(#kind_name)]);
    let fields: FieldsNamed = parse_quote!({ #(#fields),* });
    let mut carrier_attributes = outer_attributes;
    carrier_attributes.push(header_attribute);
    carrier_attributes.push(argument_attribute);
    carrier_attributes.push(kind_attribute);
    let carrier: ItemStruct = parse2(quote!(
        #[derive(#support)]
        #(#carrier_attributes)*
        struct #name #fields
    ))?;
    Ok(quote!(#carrier))
}

/// Reconstructs the configured declaration and runs the existing pipeline.
///
/// # Parameters
///
/// - `input`: Compiler-filtered carrier derive input.
///
/// # Returns
///
/// Returns the original macro expansion after semantic validation.
///
/// # Errors
///
/// Returns an error when internal carrier metadata is missing or malformed.
fn expand_configured(input: TokenStream) -> Result<TokenStream> {
    let input: DeriveInput = parse2(input)?;
    let header = attribute_tokens(&input.attrs, "reflect_configure_header")?;
    let arguments = attribute_tokens(&input.attrs, "reflect_configure_args")?;
    let kind_attribute = input
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("reflect_configure_kind"))
        .ok_or_else(|| Error::new_spanned(&input.ident, "missing internal reflection kind"))?;
    let kind = match kind_attribute
        .meta
        .require_list()?
        .tokens
        .to_string()
        .as_str()
    {
        "trait" => MacroKind::Trait,
        "implementation" => MacroKind::Impl,
        _ => {
            return Err(Error::new_spanned(
                kind_attribute,
                "invalid internal reflection kind",
            ));
        }
    };
    let mut members = Vec::new();
    let Data::Struct(data) = input.data else {
        return Err(Error::new_spanned(
            input.ident,
            "internal reflection carrier must be a struct",
        ));
    };
    let Fields::Named(fields) = data.fields else {
        return Err(Error::new_spanned(
            input.ident,
            "internal reflection carrier must use named fields",
        ));
    };
    for field in fields.named {
        let member = attribute_tokens(&field.attrs, "reflect_configure_member")?;
        let reflection_attributes: Vec<_> = field
            .attrs
            .iter()
            .filter(|attribute| attribute.path().is_ident("reflect"))
            .cloned()
            .collect();
        members.push(restore_member(member, &reflection_attributes, kind)?);
    }
    match kind {
        MacroKind::Trait => {
            let mut declaration: ItemTrait = parse2(header)?;
            for member in members {
                declaration.items.push(parse2(member)?);
            }
            Ok(process_macro(kind, arguments.into(), quote!(#declaration).into()).into())
        }
        MacroKind::Impl => {
            let mut declaration: ItemImpl = parse2(header)?;
            for member in members {
                declaration.items.push(parse2(member)?);
            }
            Ok(process_macro(kind, arguments.into(), quote!(#declaration).into()).into())
        }
        MacroKind::Derive => unreachable!(),
    }
}

/// Extracts the token payload from one required internal helper attribute.
///
/// # Parameters
///
/// - `attributes`: Attributes supplied on the carrier or its member field.
/// - `name`: Required helper attribute name.
///
/// # Returns
///
/// Returns the helper's inner token stream.
///
/// # Errors
///
/// Returns a diagnostic when the helper is absent or has no list payload.
fn attribute_tokens(attributes: &[Attribute], name: &str) -> Result<TokenStream> {
    let attribute = attributes
        .iter()
        .find(|attribute| attribute.path().is_ident(name))
        .ok_or_else(|| {
            Error::new(
                Span::call_site(),
                format!("missing internal attribute `{name}`"),
            )
        })?;
    Ok(attribute.meta.require_list()?.tokens.clone())
}

/// Restores the active reflection helpers to one compiler-filtered member.
///
/// # Parameters
///
/// - `member`: Member tokens stored in the carrier field.
/// - `helpers`: Reflection helper attributes left active by rustc.
/// - `kind`: Declaration kind selecting the member parser.
///
/// # Returns
///
/// Returns member tokens ready for the existing parse/validate pipeline.
///
/// # Errors
///
/// Returns a syntax diagnostic if the member or helper cannot be parsed.
fn restore_member(
    member: TokenStream,
    helpers: &[Attribute],
    kind: MacroKind,
) -> Result<TokenStream> {
    match kind {
        MacroKind::Trait => {
            let mut item: TraitItem = parse2(member)?;
            if let Some(attributes) = trait_item_attributes_mut(&mut item) {
                attributes.extend_from_slice(helpers);
            } else if !helpers.is_empty() {
                return Err(Error::new_spanned(
                    item,
                    "reflection helpers cannot be attached to a verbatim trait member",
                ));
            }
            Ok(quote!(#item))
        }
        MacroKind::Impl => {
            let mut item: ImplItem = parse2(member)?;
            if let Some(attributes) = impl_item_attributes_mut(&mut item) {
                attributes.extend_from_slice(helpers);
            } else if !helpers.is_empty() {
                return Err(Error::new_spanned(
                    item,
                    "reflection helpers cannot be attached to a verbatim impl member",
                ));
            }
            Ok(quote!(#item))
        }
        MacroKind::Derive => unreachable!(),
    }
}

/// Returns mutable attributes for a trait item.
///
/// # Parameters
///
/// - `item`: Trait member whose attributes are updated.
///
/// # Returns
///
/// Returns the member's mutable attribute vector, or `None` for verbatim items.
#[must_use]
fn trait_item_attributes_mut(item: &mut TraitItem) -> Option<&mut Vec<Attribute>> {
    match item {
        TraitItem::Const(value) => Some(&mut value.attrs),
        TraitItem::Fn(value) => Some(&mut value.attrs),
        TraitItem::Type(value) => Some(&mut value.attrs),
        TraitItem::Macro(value) => Some(&mut value.attrs),
        TraitItem::Verbatim(_) => None,
        _ => None,
    }
}

/// Returns mutable attributes for an impl item.
///
/// # Parameters
///
/// - `item`: Impl member whose attributes are updated.
///
/// # Returns
///
/// Returns the member's mutable attribute vector, or `None` for verbatim items.
#[must_use]
fn impl_item_attributes_mut(item: &mut ImplItem) -> Option<&mut Vec<Attribute>> {
    match item {
        ImplItem::Const(value) => Some(&mut value.attrs),
        ImplItem::Fn(value) => Some(&mut value.attrs),
        ImplItem::Type(value) => Some(&mut value.attrs),
        ImplItem::Macro(value) => Some(&mut value.attrs),
        ImplItem::Verbatim(_) => None,
        _ => None,
    }
}

/// Chooses an internal derive path available to the downstream crate.
///
/// # Parameters
///
/// - `arguments`: Outer reflection macro arguments that may name a facade.
///
/// # Returns
///
/// Returns a direct derive path or the versioned runtime facade re-export.
///
/// # Errors
///
/// Returns a diagnostic if neither supported dependency path is available.
fn support_path(arguments: &TokenStream) -> Result<TokenStream> {
    if let Ok(found) = crate_name("qubit-reflect-derive") {
        let path = match found {
            FoundCrate::Itself => quote!(crate),
            FoundCrate::Name(name) => {
                let identifier = Ident::new(&name, Span::call_site());
                quote!(::#identifier)
            }
        };
        return Ok(quote!(#path::ConfiguredReflection));
    }
    if let Some(path) = explicit_runtime_path(arguments)? {
        return Ok(quote!(#path::__private::codegen_v3::macro_support::ConfiguredReflection));
    }
    match crate_name("qubit-reflect") {
        Ok(FoundCrate::Itself) => Ok(quote!(
            crate::__private::codegen_v3::macro_support::ConfiguredReflection
        )),
        Ok(FoundCrate::Name(name)) => {
            let identifier = Ident::new(&name, Span::call_site());
            Ok(quote!(::#identifier::__private::codegen_v3::macro_support::ConfiguredReflection))
        }
        Err(_) => Err(Error::new(
            Span::call_site(),
            "cannot resolve the internal reflection derive; add `qubit-reflect-derive` or enable the facade's `derive` feature",
        )),
    }
}

/// Reads an explicit runtime path from outer macro arguments.
///
/// # Parameters
///
/// - `arguments`: Reflection helper syntax.
///
/// # Returns
///
/// Returns the `crate = path` value when present, or `None` when absent.
///
/// # Errors
///
/// Returns a syntax diagnostic for malformed arguments.
pub(super) fn explicit_runtime_path(arguments: &TokenStream) -> Result<Option<Path>> {
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let values = parser.parse2(arguments.clone())?;
    for value in values {
        if let Meta::NameValue(name_value) = value
            && name_value.path.is_ident("crate")
        {
            let Expr::Path(path) = name_value.value else {
                return Err(Error::new(
                    name_value.value.span(),
                    "`crate` must be a path",
                ));
            };
            return Ok(Some(path.path));
        }
    }
    Ok(None)
}
