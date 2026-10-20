// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
/// Runtime and display identity for one concrete Rust type.
pub struct TypeIdentity {
    /// Exact process-local Rust type identifier.
    pub type_id: TypeId,
    /// Rust's diagnostic name for the type.
    pub rust_name: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
/// Category of reflected item represented by an [`ItemIdentity`].
pub enum ItemKind {
    /// A concrete Rust type.
    Type,
    /// A generic type declaration.
    TypeDefinition,
    /// A trait declaration.
    TraitDefinition,
    /// An application of a trait to a type.
    TraitApplication,
    /// An implementation item.
    Impl,
    /// A field.
    Field,
    /// An enum variant.
    Variant,
    /// A method.
    Method,
    /// A concrete generic method specialization.
    MethodSpecialization,
    /// A function or method parameter.
    Parameter,
    /// An associated type.
    AssociatedType,
    /// An associated constant.
    AssociatedConst,
    /// A registered capability.
    Capability,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Stable textual identifier assigned to a reflected item.
pub struct StableItemId(pub Box<str>);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
/// Kind, stable key, and user-facing name for a reflected item.
pub struct ItemIdentity {
    /// Category of the item.
    pub kind: ItemKind,
    /// Stable identifier used to distinguish items of this kind.
    pub stable_id: StableItemId,
    /// Name used in diagnostics and user-facing listings.
    pub display_name: Box<str>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
/// Structural identity of a type, const, or lifetime generic argument.
pub enum GenericArgumentIdentity {
    /// Type argument represented by its recorded type name.
    Type(Box<str>),
    /// Const argument represented by its source expression.
    Const(Box<str>),
    /// Lifetime argument represented by its source lifetime.
    Lifetime(Box<str>),
}

/// Exact internal key used to select a concrete method specialization.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum GenericArgumentSelector {
    Type(TypeId),
    Const { type_id: TypeId, value: Box<str> },
    Lifetime(Box<str>),
    UnsupportedConst(TypeId),
}

impl GenericArgumentSelector {
    pub(crate) fn matches(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Type(left), Self::Type(right)) => left == right,
            (
                Self::Const {
                    type_id: left_type,
                    value: left_value,
                },
                Self::Const {
                    type_id: right_type,
                    value: right_value,
                },
            ) => left_type == right_type && left_value == right_value,
            (Self::Lifetime(left), Self::Lifetime(right)) => left == right,
            // Arbitrary `Any` values do not expose equality. Never infer that
            // two values match from their type or diagnostic text alone.
            (Self::UnsupportedConst(_), _) | (_, Self::UnsupportedConst(_)) => false,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Source location and package metadata for a registration.
pub struct RegistrationOrigin {
    /// Name of the crate containing the registration.
    pub crate_name: &'static str,
    /// Crate version when known.
    pub crate_version: Option<&'static str>,
    /// Module path containing the registration.
    pub module_path: &'static str,
    /// Source file path when available.
    pub file: Option<&'static str>,
    /// One-based source line when available.
    pub line: Option<u32>,
    /// One-based source column when available.
    pub column: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// A candidate item returned by a lookup, with optional registration
/// provenance.
pub struct LookupCandidate {
    /// Stable identity of the candidate.
    pub identity: ItemIdentity,
    /// Name selected for displaying the candidate.
    pub display_name: Box<str>,
    /// Registration site, when the item has provenance metadata.
    pub origin: Option<RegistrationOrigin>,
}

/// Converts a lookup result into the diagnostic form used by
/// [`crate::error::LookupError`].
pub trait IntoLookupCandidate {
    /// Consumes this value and returns its lookup candidate information.
    fn into_lookup_candidate(self) -> LookupCandidate;
}

impl IntoLookupCandidate for LookupCandidate {
    fn into_lookup_candidate(self) -> LookupCandidate {
        self
    }
}
