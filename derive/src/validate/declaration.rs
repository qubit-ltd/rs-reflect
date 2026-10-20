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

use crate::internal::ErrorCollector;
use crate::ir::DeclarationIr;
use crate::ir::GenericKindIr;
use crate::ir::GenericsIr;
use crate::ir::HelperAttributeIr;
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
///
/// # Parameters
///
/// - `declaration`: Parsed declaration whose locally provable rules are
///   checked.
///
/// # Returns
///
/// Returns validated IR or aggregated validation diagnostics.
///
/// # Errors
///
/// Returns all collected semantic validation diagnostics with their source
/// spans.
#[allow(
    dead_code,
    reason = "the staged validation API is exercised directly by unit tests and later expansion tasks"
)]
pub(crate) fn validate_declaration(declaration: ParsedDeclaration) -> syn::Result<ValidatedDeclaration> {
    match validation_error(&declaration.declaration) {
        Some(error) => Err(error),
        None => Ok(ValidatedDeclaration {
            declaration: declaration.declaration,
        }),
    }
}

/// Collects semantic diagnostics without consuming the parsed declaration.
///
/// # Parameters
///
/// - `declaration`: Parsed declaration to inspect.
///
/// # Returns
///
/// Returns combined diagnostics, or `None` when validation succeeds.
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
///
/// # Parameters
///
/// - `declaration`: Type declaration and nested members to validate.
/// - `errors`: Collector for independent diagnostics.
fn validate_type(declaration: &TypeDeclarationIr, errors: &mut ErrorCollector) {
    validate_attributes(&declaration.attributes, errors);
    if declaration.kind == TypeDeclarationKindIr::Union {
        errors.push(syn::Error::new(
            declaration.span,
            "Reflect cannot be derived for unions",
        ));
    }
    validate_fields(&declaration.fields, errors);
    for variant in &declaration.variants {
        validate_attributes(&variant.attributes, errors);
        validate_fields(&variant.fields, errors);
    }
}

/// Validates field helper policies in source order.
///
/// # Parameters
///
/// - `fields`: Fields to validate.
/// - `errors`: Collector for independent diagnostics.
fn validate_fields(fields: &[crate::ir::FieldIr], errors: &mut ErrorCollector) {
    for field in fields {
        validate_attributes(&field.attributes, errors);
    }
}

/// Validates trait-level mappings, methods, and associated-item helpers.
///
/// # Parameters
///
/// - `declaration`: Trait declaration and its members to validate.
/// - `errors`: Collector for independent diagnostics.
fn validate_trait(declaration: &TraitDeclarationIr, errors: &mut ErrorCollector) {
    validate_attributes(&declaration.attributes, errors);
    let mut bound_indexes = HashSet::new();
    for binding in &declaration.supertrait_bindings {
        if binding.index >= declaration.supertraits.len() {
            errors.push(syn::Error::new(
                binding.provider.span,
                format!(
                    "supertrait binding index {} is out of range ({} direct supertraits)",
                    binding.index,
                    declaration.supertraits.len()
                ),
            ));
        }
        if !bound_indexes.insert(binding.index) {
            errors.push(syn::Error::new(
                binding.provider.span,
                format!("supertrait index {} is bound more than once", binding.index),
            ));
        }
        if !matches!(
            declaration.supertraits.get(binding.index),
            Some(crate::ir::GenericBoundIr::Trait { .. })
        ) {
            errors.push(syn::Error::new(
                binding.provider.span,
                "supertrait bindings must refer to a trait bound",
            ));
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
}

/// Validates impl-level identity, specialization, methods, and associated
/// items.
///
/// # Parameters
///
/// - `declaration`: Impl declaration and its members to validate.
/// - `errors`: Collector for independent diagnostics.
fn validate_impl(declaration: &ImplDeclarationIr, errors: &mut ErrorCollector) {
    validate_attributes(&declaration.attributes, errors);
    if declaration.trait_path.is_none()
        && (declaration.reflected_trait.is_some() || declaration.external_trait.is_some())
    {
        errors.push(syn::Error::new(
            declaration.span,
            "trait identity arguments require a trait impl",
        ));
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
}

/// Validates one method and each of its concrete specialization declarations.
///
/// # Parameters
///
/// - `method`: Method and its reflection policies.
/// - `errors`: Collector for independent diagnostics.
fn validate_method(method: &MethodIr, errors: &mut ErrorCollector) {
    validate_attributes(&method.attributes, errors);
    for specialization in &method.specializations {
        validate_specialization(specialization, &method.generics, errors);
    }
}

/// Validates legality, cardinality, values, and mutually exclusive helper
/// policies.
///
/// # Parameters
///
/// - `attributes`: Helper occurrences attached to one declaration.
/// - `errors`: Collector for independent diagnostics.
fn validate_attributes(attributes: &[HelperAttributeIr], errors: &mut ErrorCollector) {
    let mut seen = HashSet::new();
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
        if !seen.insert(attribute.name) {
            errors.push(syn::Error::new(
                attribute.span,
                format!("duplicate `{}` reflection helper", attribute.name.as_str()),
            ));
        }
    }
}

/// Validates named specialization completeness and RHS syntax by parameter
/// kind.
///
/// # Parameters
///
/// - `specialization`: Named concrete bindings to validate.
/// - `generics`: Generic parameter declaration to compare against.
/// - `errors`: Collector for independent diagnostics.
fn validate_specialization(specialization: &SpecializationIr, generics: &GenericsIr, errors: &mut ErrorCollector) {
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
