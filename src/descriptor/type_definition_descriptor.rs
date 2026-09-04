// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! First-class reflected generic type declarations.

use crate::descriptor::FieldDefinitionDescriptor;
use crate::descriptor::StructKind;
use crate::descriptor::TypeDefinitionData;
use crate::descriptor::TypeDefinitionId;
use crate::descriptor::VariantDefinitionDescriptor;
use crate::expression::GenericDefinitionDescriptor;

/// The immutable source-level description of one generic type declaration.
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
    #[must_use]
    pub const fn id(&self) -> TypeDefinitionId {
        self.id
    }

    /// Returns the fully qualified source path.
    #[must_use]
    pub const fn rust_path(&self) -> &'static str {
        self.rust_path
    }

    /// Returns the immutable lookup name.
    #[must_use]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns the generic parameters and predicates.
    #[must_use]
    pub const fn generics(&self) -> &'static GenericDefinitionDescriptor {
        self.generics
    }

    /// Returns the kind-specific declaration structure.
    #[must_use]
    pub const fn data(&self) -> &TypeDefinitionData {
        &self.data
    }

    /// Returns struct fields, or `None` for non-struct declarations.
    #[must_use]
    pub const fn fields(&self) -> Option<&'static [FieldDefinitionDescriptor]> {
        match &self.data {
            TypeDefinitionData::Struct { fields, .. } => Some(fields),
            _ => None,
        }
    }

    /// Returns enum variants, or `None` for non-enum declarations.
    #[must_use]
    pub const fn variants(&self) -> Option<&'static [VariantDefinitionDescriptor]> {
        match &self.data {
            TypeDefinitionData::Enum { variants } => Some(variants),
            _ => None,
        }
    }
}
