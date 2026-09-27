// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! First-class reflected generic type declarations.

use crate::descriptor::FieldDefinitionDescriptor;
use crate::descriptor::StructKind;
use crate::descriptor::TypeDefinitionData;
use crate::descriptor::TypeDefinitionId;
use crate::descriptor::VariantDefinitionDescriptor;
use crate::expression::GenericDefinitionDescriptor;

/// The immutable source-level description of one generic type declaration.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::{TypeDefinitionDescriptor, TypeDefinitionId};
/// use qubit_reflect::expression::GenericDefinitionDescriptor;
/// struct Marker;
/// let generics = Box::leak(Box::new(GenericDefinitionDescriptor::new([], [])));
/// let definition = TypeDefinitionDescriptor::opaque(
///     TypeDefinitionId::of::<Marker>(), "example::Marker", "Marker", generics,
/// );
/// assert_eq!(definition.query_name(), "Marker");
/// ```
#[derive(Debug)]
pub struct TypeDefinitionDescriptor {
    id: TypeDefinitionId,
    rust_path: &'static str,
    query_name: &'static str,
    generics: &'static GenericDefinitionDescriptor,
    data: TypeDefinitionData,
}

impl TypeDefinitionDescriptor {
    /// Creates an opaque generic declaration.
    ///
    /// # Parameters
    ///
    /// - `id`: Process-local identity of the source declaration.
    /// - `rust_path`: Fully qualified Rust source path.
    /// - `query_name`: Stable reflection lookup name.
    /// - `generics`: Declaration-level generic parameters and predicates.
    ///
    /// # Returns
    ///
    /// Returns a declaration with no navigable fields or variants.
    #[doc(hidden)]
    #[must_use]
    pub const fn opaque(
        id: TypeDefinitionId,
        rust_path: &'static str,
        query_name: &'static str,
        generics: &'static GenericDefinitionDescriptor,
    ) -> Self {
        Self {
            id,
            rust_path,
            query_name,
            generics,
            data: TypeDefinitionData::Opaque,
        }
    }

    /// Creates a generic struct declaration.
    ///
    /// # Parameters
    ///
    /// - `id`: Process-local identity of the source declaration.
    /// - `rust_path`: Fully qualified Rust source path.
    /// - `query_name`: Stable reflection lookup name.
    /// - `generics`: Declaration-level generic parameters and predicates.
    /// - `kind`: Source struct shape.
    /// - `fields`: Field facts in declaration order.
    ///
    /// # Returns
    ///
    /// Returns a declaration with its source-level fields.
    #[doc(hidden)]
    #[must_use]
    pub const fn struct_type(
        id: TypeDefinitionId,
        rust_path: &'static str,
        query_name: &'static str,
        generics: &'static GenericDefinitionDescriptor,
        kind: StructKind,
        fields: &'static [FieldDefinitionDescriptor],
    ) -> Self {
        Self {
            id,
            rust_path,
            query_name,
            generics,
            data: TypeDefinitionData::Struct { kind, fields },
        }
    }

    /// Creates a generic enum declaration.
    ///
    /// # Parameters
    ///
    /// - `id`: Process-local identity of the source declaration.
    /// - `rust_path`: Fully qualified Rust source path.
    /// - `query_name`: Stable reflection lookup name.
    /// - `generics`: Declaration-level generic parameters and predicates.
    /// - `variants`: Variant facts in declaration order.
    ///
    /// # Returns
    ///
    /// Returns a declaration with its source-level variants.
    #[doc(hidden)]
    #[must_use]
    pub const fn enum_type(
        id: TypeDefinitionId,
        rust_path: &'static str,
        query_name: &'static str,
        generics: &'static GenericDefinitionDescriptor,
        variants: &'static [VariantDefinitionDescriptor],
    ) -> Self {
        Self {
            id,
            rust_path,
            query_name,
            generics,
            data: TypeDefinitionData::Enum { variants },
        }
    }

    /// Returns the process-local declaration identity.
    ///
    /// # Returns
    ///
    /// Returns the marker identity, which is stable only within this process.
    #[must_use]
    #[inline]
    pub const fn id(&self) -> TypeDefinitionId {
        self.id
    }

    /// Returns the fully qualified source path.
    ///
    /// # Returns
    ///
    /// Returns the declaration's Rust path.
    #[must_use]
    #[inline]
    pub const fn rust_path(&self) -> &'static str {
        self.rust_path
    }

    /// Returns the immutable lookup name.
    ///
    /// # Returns
    ///
    /// Returns the declaration's reflection query name.
    #[must_use]
    #[inline]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns the generic parameters and predicates.
    ///
    /// # Returns
    ///
    /// Returns the static source-level generic definition.
    #[must_use]
    #[inline]
    pub const fn generics(&self) -> &'static GenericDefinitionDescriptor {
        self.generics
    }

    /// Returns the kind-specific declaration structure.
    ///
    /// # Returns
    ///
    /// Returns the opaque, struct, or enum source facts.
    #[must_use]
    #[inline]
    pub const fn data(&self) -> &TypeDefinitionData {
        &self.data
    }

    /// Returns struct fields, or `None` for non-struct declarations.
    ///
    /// # Returns
    ///
    /// Returns source fields for structs, or `None` for opaque and enum
    /// declarations.
    #[must_use]
    #[inline]
    pub const fn fields(&self) -> Option<&'static [FieldDefinitionDescriptor]> {
        match &self.data {
            TypeDefinitionData::Struct { fields, .. } => Some(fields),
            _ => None,
        }
    }

    /// Returns enum variants, or `None` for non-enum declarations.
    ///
    /// # Returns
    ///
    /// Returns source variants for enums, or `None` for opaque and struct
    /// declarations.
    #[must_use]
    #[inline]
    pub const fn variants(&self) -> Option<&'static [VariantDefinitionDescriptor]> {
        match &self.data {
            TypeDefinitionData::Enum { variants } => Some(variants),
            _ => None,
        }
    }
}
