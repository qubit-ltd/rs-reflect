// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Semantic validation for reflection declarations.

use std::collections::HashMap;
use std::collections::HashSet;

use proc_macro2::Span;

use crate::internal::ErrorCollector;
use crate::ir::DeclarationIr;
use crate::ir::GenericBoundIr;
use crate::ir::GenericKindIr;
use crate::ir::GenericsIr;
use crate::ir::HelperAttributeIr;
use crate::ir::HelperName;
use crate::ir::HelperValueIr;
use crate::ir::ImplDeclarationIr;
use crate::ir::MethodIr;
use crate::ir::ParsedDeclaration;
use crate::ir::SpecializationIr;
use crate::ir::SpecializationValueIr;
use crate::ir::TraitDeclarationIr;
use crate::ir::TypeDeclarationIr;
use crate::ir::TypeDeclarationKindIr;
use crate::ir::ValidatedDeclaration;

/// Validates all locally provable invariants in parsed reflection IR.
///
/// Returns a combined `syn::Error` whose component spans point at the offending
/// helpers.
#[allow(
    dead_code,
    reason = "the staged validation API is exercised directly by unit tests and later expansion tasks"
)]
pub(crate) fn validate_declaration(
    declaration: ParsedDeclaration,
) -> syn::Result<ValidatedDeclaration> {
    match validation_error(&declaration.declaration) {
        Some(error) => Err(error),
        None => Ok(ValidatedDeclaration {
            declaration: declaration.declaration,
        }),
    }
}

/// Collects semantic diagnostics without consuming the parsed declaration.
pub(crate) fn validation_error(declaration: &DeclarationIr) -> Option<syn::Error> {
    let mut errors = ErrorCollector::default();
    match declaration {
        DeclarationIr::Type(value) => validate_type(value, &mut errors),
        DeclarationIr::Trait(value) => validate_trait(value, &mut errors),
        DeclarationIr::Impl(value) => validate_impl(value, &mut errors),
    }
    errors.into_error()
}

/// Validates a struct, enum, or rejected union declaration.
fn validate_type(declaration: &TypeDeclarationIr, errors: &mut ErrorCollector) {
    validate_attributes(&declaration.attributes, errors);
    if declaration.generics.params.is_empty() {
        for attribute in &declaration.attributes {
            if attribute.name == HelperName::DefinitionProviderV2 {
                errors.push(syn::Error::new(attribute.span, "definition_provider_v2 requires a generic type"));
            }
        }
    }
    if declaration.kind == TypeDeclarationKindIr::Union {
        errors.push(syn::Error::new(
            declaration.span,
            "Reflect cannot be derived for unions",
        ));
    }
    validate_query_name(
        &declaration.attributes,
        &declaration.name.to_string(),
        errors,
    );
    validate_fields(&declaration.fields, errors);
    validate_query_name_scope(
        declaration.fields.iter().filter_map(|field| {
            field
                .name
                .as_ref()
                .map(|name| (name.to_string(), &field.attributes, field.span))
        }),
        "field",
        errors,
    );
    validate_query_name_scope(
        declaration
            .variants
            .iter()
            .map(|variant| (variant.name.to_string(), &variant.attributes, variant.span)),
        "variant",
        errors,
    );
    for variant in &declaration.variants {
        validate_attributes(&variant.attributes, errors);
        validate_query_name(&variant.attributes, &variant.name.to_string(), errors);
        validate_fields(&variant.fields, errors);
        validate_query_name_scope(
            variant.fields.iter().filter_map(|field| {
                field
                    .name
                    .as_ref()
                    .map(|name| (name.to_string(), &field.attributes, field.span))
            }),
            "field",
            errors,
        );
    }
}

/// Validates field helper policies in source order.
fn validate_fields(fields: &[crate::ir::FieldIr], errors: &mut ErrorCollector) {
    for field in fields {
        validate_attributes(&field.attributes, errors);
        let rust_name = field
            .name
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| field.index.to_string());
        validate_query_name(&field.attributes, &rust_name, errors);
        if field.name.is_none()
            && let Some(rename) = field
                .attributes
                .iter()
                .find(|attribute| attribute.name == HelperName::Rename)
        {
            errors.push(syn::Error::new(
                rename.span,
                "`rename` is not supported on an unnamed field",
            ));
        }
    }
}

/// Validates trait-level mappings, methods, and associated-item helpers.
fn validate_trait(declaration: &TraitDeclarationIr, errors: &mut ErrorCollector) {
    validate_attributes(&declaration.attributes, errors);
    validate_query_name(
        &declaration.attributes,
        &declaration.name.to_string(),
        errors,
    );
    validate_external_traits(declaration, errors);
    let mut inherited_dyn_items = std::collections::HashSet::new();
    for attribute in &declaration.attributes {
        let HelperValueIr::DynCompatible(paths) = &attribute.value else {
            continue;
        };
        for path in paths {
            if path.segments.len() < 2 {
                errors.push(syn::Error::new(
                    attribute.value_span,
                    "`dyn_compatible` inherited associated types must use `Supertrait::Item` paths",
                ));
                continue;
            }
            let owner = &path.segments[..path.segments.len() - 1];
            let is_direct_supertrait = declaration.supertraits.iter().any(|bound| {
                let GenericBoundIr::Trait { path, .. } = bound else {
                    return false;
                };
                path.segments.len() == owner.len()
                    && path
                        .segments
                        .iter()
                        .zip(owner)
                        .all(|(direct, proven)| direct.name == proven.name)
            });
            if !is_direct_supertrait {
                errors.push(syn::Error::new(
                    attribute.value_span,
                    "`dyn_compatible` associated-type prefixes must name a direct supertrait",
                ));
                continue;
            }
            let item = &path
                .segments
                .last()
                .expect("a path with two segments has an item")
                .name;
            if !inherited_dyn_items.insert(item.clone()) {
                errors.push(syn::Error::new(
                    attribute.value_span,
                    format!("duplicate inherited dyn associated type `{item}`"),
                ));
            }
        }
    }
    for method in &declaration.methods {
        validate_method(method, errors);
    }
    for associated in &declaration.associated_types {
        validate_attributes(&associated.attributes, errors);
    }
    for associated in &declaration.associated_consts {
        validate_attributes(&associated.attributes, errors);
    }
    validate_method_query_names(&declaration.methods, errors);
}

/// Validates impl-level identity, specialization, methods, and associated
/// items.
fn validate_impl(declaration: &ImplDeclarationIr, errors: &mut ErrorCollector) {
    validate_attributes(&declaration.attributes, errors);
    if declaration.trait_path.is_none() {
        for attribute in &declaration.attributes {
            if attribute.name == HelperName::ExternalTraitId {
                errors.push(syn::Error::new(
                    attribute.span,
                    "`external_trait_id` is only valid on a trait impl",
                ));
            }
        }
    }
    for attribute in &declaration.attributes {
        if let HelperValueIr::ExternalTraitId(id) = &attribute.value {
            validate_stable_id(id, attribute.value_span, errors);
        }
    }
    for specialization in &declaration.specializations {
        validate_specialization(specialization, &declaration.generics, errors);
    }
    for method in &declaration.methods {
        validate_method(method, errors);
    }
    for associated in &declaration.associated_types {
        validate_attributes(&associated.attributes, errors);
    }
    for associated in &declaration.associated_consts {
        validate_attributes(&associated.attributes, errors);
    }
    validate_method_query_names(&declaration.methods, errors);
}

/// Validates one method and each of its concrete specialization declarations.
fn validate_method(method: &MethodIr, errors: &mut ErrorCollector) {
    validate_attributes(&method.attributes, errors);
    if method.qualifiers.is_async
        && let Some(attribute) = method
            .attributes
            .iter()
            .find(|attribute| attribute.name == HelperName::CatchUnwind)
    {
        errors.push(syn::Error::new(
            attribute.span,
            "`catch_unwind` cannot be used on an async method; panics raised while polling follow ordinary Future semantics",
        ));
    }
    validate_query_name(&method.attributes, &method.name.to_string(), errors);
    for specialization in &method.specializations {
        validate_specialization(specialization, &method.generics, errors);
    }
}

/// Validates legality, cardinality, values, and mutually exclusive helper
/// policies.
fn validate_attributes(attributes: &[HelperAttributeIr], errors: &mut ErrorCollector) {
    let mut seen = HashMap::new();
    for attribute in attributes {
        if !attribute.name.supports(attribute.target) {
            errors.push(syn::Error::new(
                attribute.span,
                format!(
                    "`{}` is not valid on a {}",
                    attribute.name.as_str(),
                    attribute.target.as_str()
                ),
            ));
        }
        let repeatable = matches!(
            attribute.name,
            HelperName::Specialize | HelperName::ExternalTrait
        );
        if !repeatable && seen.insert(attribute.name, attribute.span).is_some() {
            errors.push(syn::Error::new(
                attribute.span,
                format!("duplicate `{}` reflection helper", attribute.name.as_str()),
            ));
        }
        if let HelperValueIr::Paths(paths) = &attribute.value {
            let mut path_names = HashSet::new();
            for path in paths {
                if !path_names.insert(path.source.clone()) {
                    errors.push(syn::Error::new(
                        path.span,
                        format!("duplicate capability `{}`", path.source),
                    ));
                }
            }
        }
    }
    validate_conflicts(attributes, errors);
}

/// Validates policy pairs that would otherwise request contradictory adapters.
fn validate_conflicts(attributes: &[HelperAttributeIr], errors: &mut ErrorCollector) {
    if let Some(skip) = attributes
        .iter()
        .find(|attribute| attribute.name == HelperName::Skip)
    {
        let conflicts = match skip.target {
            crate::ir::HelperTarget::Field => {
                [HelperName::ReadOnly, HelperName::NoConstruct].as_slice()
            }
            crate::ir::HelperTarget::Variant => [HelperName::NoConstruct].as_slice(),
            crate::ir::HelperTarget::Method => [
                HelperName::NoInvoke,
                HelperName::CatchUnwind,
                HelperName::ThreadSafe,
            ]
            .as_slice(),
            _ => [].as_slice(),
        };
        for conflict in conflicts {
            report_conflict(attributes, HelperName::Skip, *conflict, errors);
        }
    }
    if attributes
        .iter()
        .any(|attribute| attribute.name == HelperName::NoInvoke)
    {
        report_conflict(
            attributes,
            HelperName::NoInvoke,
            HelperName::CatchUnwind,
            errors,
        );
        report_conflict(
            attributes,
            HelperName::NoInvoke,
            HelperName::ThreadSafe,
            errors,
        );
    }
}

/// Reports one mutually exclusive helper pair when both keys are present.
fn report_conflict(
    attributes: &[HelperAttributeIr],
    left: HelperName,
    right: HelperName,
    errors: &mut ErrorCollector,
) {
    if let Some(attribute) = attributes.iter().find(|attribute| attribute.name == right) {
        errors.push(syn::Error::new(
            attribute.span,
            format!(
                "`{}` cannot be combined with `{}`",
                left.as_str(),
                right.as_str()
            ),
        ));
    }
}

/// Validates that rename literals are non-empty.
fn validate_query_name(
    attributes: &[HelperAttributeIr],
    rust_name: &str,
    errors: &mut ErrorCollector,
) {
    if let Some(rename) = attributes
        .iter()
        .find(|attribute| attribute.name == HelperName::Rename)
        && rename.rename().is_some_and(str::is_empty)
    {
        errors.push(syn::Error::new(
            rename.value_span,
            format!("reflection rename cannot be empty for Rust member `{rust_name}`"),
        ));
    }
}

/// Validates query-name uniqueness in one member scope.
fn validate_query_name_scope<'a>(
    members: impl Iterator<Item = (String, &'a Vec<HelperAttributeIr>, Span)>,
    member_kind: &str,
    errors: &mut ErrorCollector,
) {
    let mut names = HashMap::new();
    for (rust_name, attributes, span) in members {
        let rename = attributes
            .iter()
            .find(|attribute| attribute.name == HelperName::Rename);
        let query_name = rename
            .and_then(HelperAttributeIr::rename)
            .unwrap_or(&rust_name)
            .to_owned();
        let diagnostic_span = rename.map_or(span, |attribute| attribute.value_span);
        if let Some((existing_rust_name, _)) =
            names.insert(query_name.clone(), (rust_name.clone(), diagnostic_span))
        {
            errors.push(syn::Error::new(
                diagnostic_span,
                format!(
                    "{member_kind} query name `{query_name}` for Rust member `{rust_name}` conflicts with Rust member `{existing_rust_name}`"
                ),
            ));
        }
    }
}

/// Validates query-name uniqueness across methods in one trait or impl.
fn validate_method_query_names(methods: &[MethodIr], errors: &mut ErrorCollector) {
    validate_query_name_scope(
        methods
            .iter()
            .map(|method| (method.name.to_string(), &method.attributes, method.span)),
        "method",
        errors,
    );
}

/// Validates external trait IDs and detects same-input path or ID conflicts.
fn validate_external_traits(declaration: &TraitDeclarationIr, errors: &mut ErrorCollector) {
    let direct_supertraits: HashSet<_> = declaration
        .supertraits
        .iter()
        .filter_map(|bound| match bound {
            crate::ir::GenericBoundIr::Trait { path, .. } => Some(path.source.as_str()),
            _ => None,
        })
        .collect();
    let mut paths = HashSet::new();
    let mut ids = HashSet::new();
    for mapping in &declaration.external_traits {
        validate_stable_id(&mapping.id, mapping.id_span, errors);
        if !direct_supertraits.contains(mapping.path.source.as_str()) {
            errors.push(syn::Error::new(
                mapping.path.span,
                format!(
                    "external trait mapping `{}` does not match a direct supertrait",
                    mapping.path.source
                ),
            ));
        }
        if !paths.insert(mapping.path.source.clone()) {
            errors.push(syn::Error::new(
                mapping.span,
                format!(
                    "external trait path `{}` is mapped more than once",
                    mapping.path.source
                ),
            ));
        }
        if !ids.insert(mapping.id.clone()) {
            errors.push(syn::Error::new(
                mapping.id_span,
                format!(
                    "external trait ID `{}` is mapped more than once",
                    mapping.id
                ),
            ));
        }
    }
    let mut reflected_paths = HashSet::new();
    for path in &declaration.reflected_supertraits {
        if !direct_supertraits.contains(path.source.as_str()) {
            errors.push(syn::Error::new(
                path.span,
                format!(
                    "reflected supertrait `{}` does not match a direct supertrait",
                    path.source
                ),
            ));
        }
        if !reflected_paths.insert(path.source.clone()) {
            errors.push(syn::Error::new(
                path.span,
                format!(
                    "reflected supertrait `{}` is listed more than once",
                    path.source
                ),
            ));
        }
    }
    for path in direct_supertraits {
        if !paths.contains(path) && !reflected_paths.contains(path) {
            errors.push(syn::Error::new(
                declaration.span,
                format!(
                    "direct supertrait `{path}` needs #[reflect(supertrait({path}))] or #[reflect(external_trait({path}, id = \"...\"))]"
                ),
            ));
        }
    }
}

/// Validates the stable dot-separated identifier grammar and reserved
/// namespace.
fn validate_stable_id(value: &str, span: Span, errors: &mut ErrorCollector) {
    let valid = !value.is_empty()
        && value.split('.').all(|segment| {
            let mut characters = segment.chars();
            characters
                .next()
                .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
                && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
        });
    if !valid {
        errors.push(syn::Error::new(
            span,
            format!("`{value}` is not a dot-separated stable identifier"),
        ));
    }
    if value == "qubit.reflect" || value.starts_with("qubit.reflect.") {
        errors.push(syn::Error::new(
            span,
            "external IDs cannot use the reserved `qubit.reflect` namespace",
        ));
    }
}

/// Validates named specialization completeness and RHS syntax by parameter
/// kind.
fn validate_specialization(
    specialization: &SpecializationIr,
    generics: &GenericsIr,
    errors: &mut ErrorCollector,
) {
    let expected: HashMap<_, _> = generics
        .params
        .iter()
        .filter(|parameter| parameter.kind != GenericKindIr::Lifetime)
        .map(|parameter| (parameter.name.as_str(), parameter.kind))
        .collect();
    let mut seen = HashSet::new();
    for binding in &specialization.bindings {
        if !seen.insert(binding.name.as_str()) {
            errors.push(syn::Error::new(
                binding.span,
                format!("duplicate specialization argument `{}`", binding.name),
            ));
        }
        let Some(kind) = expected.get(binding.name.as_str()) else {
            errors.push(syn::Error::new(
                binding.span,
                format!("unknown specialization parameter `{}`", binding.name),
            ));
            continue;
        };
        let kind_matches = match (kind, &binding.value) {
            (GenericKindIr::Type, SpecializationValueIr::Type(_))
            | (GenericKindIr::Type, SpecializationValueIr::AmbiguousPath(_))
            | (GenericKindIr::Const, SpecializationValueIr::Const(_))
            | (GenericKindIr::Const, SpecializationValueIr::AmbiguousPath(_)) => true,
            (GenericKindIr::Lifetime, _) => {
                unreachable!("lifetime parameters are filtered above")
            }
            _ => false,
        };
        if !kind_matches {
            errors.push(syn::Error::new(
                binding.value_span,
                format!(
                    "specialization value for `{}` does not match its {:?} parameter kind",
                    binding.name, kind
                ),
            ));
        }
    }
    for name in expected.keys() {
        if !seen.contains(name) {
            errors.push(syn::Error::new(
                specialization.span,
                format!("missing specialization argument `{name}`"),
            ));
        }
    }
}
