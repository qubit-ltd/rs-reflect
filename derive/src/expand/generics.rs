// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Expansion helpers for generic syntax and source configuration attributes.

use crate::ir::TypeDeclarationIr;
use crate::ir::TypeDeclarationKindIr;

/// Re-parses the generic syntax retained in the type IR and reports malformed
/// internal syntax at the declaration name instead of dropping the derive.
///
/// # Parameters
///
/// - `declaration`: Validated type declaration containing retained generic
///   syntax.
///
/// # Returns
///
/// Returns the parsed generic parameters and where clause.
///
/// # Errors
///
/// Returns a syntax diagnostic when the retained generic declaration or where
/// clause cannot be parsed.
pub(crate) fn parse_type_generics(declaration: &TypeDeclarationIr) -> syn::Result<syn::Generics> {
    let kind = match declaration.kind {
        TypeDeclarationKindIr::Struct => "struct",
        TypeDeclarationKindIr::Enum => "enum",
        TypeDeclarationKindIr::Union => "union",
    };
    let error = |detail: &str, error: syn::Error| {
        syn::Error::new(
            declaration.span,
            format!(
                "cannot expand Reflect for {kind} `{}`: invalid {detail}: {error}",
                declaration.name
            ),
        )
    };
    let mut generics: syn::Generics =
        syn::parse2(declaration.generics.declaration.clone()).map_err(|cause| error("generics", cause))?;
    if !declaration.generics.where_clause.is_empty() {
        generics.where_clause =
            Some(syn::parse2(declaration.generics.where_clause.clone()).map_err(|cause| error("where clause", cause))?);
    }
    Ok(generics)
}

/// Returns source `cfg` attributes for one struct or enum-variant field.
///
/// The parser keeps source indices in the semantic IR while `cfg` attributes
/// are compiler evaluated after procedural macro expansion. Reapplying these
/// attributes to generated items keeps their presence in step with the Rust
/// field.
#[must_use]
pub(crate) fn field_cfg_attributes(
    declaration: &TypeDeclarationIr,
    variant_index: Option<usize>,
    field_index: usize,
) -> Vec<syn::Attribute> {
    let input: syn::DeriveInput =
        syn::parse2(declaration.retained_tokens.clone()).expect("the parser retains a valid type declaration");
    let fields = match (&input.data, variant_index) {
        (syn::Data::Struct(data), None) => &data.fields,
        (syn::Data::Enum(data), Some(index)) => {
            let Some(variant) = data.variants.iter().nth(index) else {
                return Vec::new();
            };
            &variant.fields
        }
        _ => return Vec::new(),
    };
    fields
        .iter()
        .nth(field_index)
        .map(|field| {
            field
                .attrs
                .iter()
                .filter(|attribute| attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr"))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// Returns source `cfg` attributes for one enum variant.
#[must_use]
pub(crate) fn variant_cfg_attributes(declaration: &TypeDeclarationIr, variant_index: usize) -> Vec<syn::Attribute> {
    let input: syn::DeriveInput =
        syn::parse2(declaration.retained_tokens.clone()).expect("the parser retains a valid type declaration");
    let syn::Data::Enum(data) = input.data else {
        return Vec::new();
    };
    data.variants
        .iter()
        .nth(variant_index)
        .map(|variant| {
            variant
                .attrs
                .iter()
                .filter(|attribute| attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr"))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use crate::ir::DeclarationIr;
    use crate::ir::MacroKind;
    use crate::parse::parse_declaration;

    #[test]
    fn test_generic_definition_emits_codegen_v4_only() {
        let parsed = parse_declaration(
            MacroKind::Derive,
            quote!(),
            quote! {
                struct Generic<'a, T: Copy, const N: usize>
                where
                    T: 'a,
                {
                    value: Unreflected<'a, T, N>,
                }
            },
        )
        .expect("the generic fixture must parse");
        let DeclarationIr::Type(declaration) = parsed.declaration else {
            panic!("the generic fixture must produce a type declaration");
        };
        let generated = super::super::expression_codegen::generic_definition(
            &declaration.generics,
            declaration.span,
            &quote!(qubit_reflect),
        )
        .to_string();

        assert!(
            generated.contains("codegen_v4"),
            "generic definitions must use the v4 protocol"
        );
        assert!(
            !generated.contains(concat!("codegen_", "v3")),
            "generic definitions must not use the v3 protocol"
        );
    }
}
