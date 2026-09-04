// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

//! Kind-specific source structure for generic declarations.

use crate::descriptor::FieldDefinitionDescriptor;
use crate::descriptor::StructKind;
use crate::descriptor::VariantDefinitionDescriptor;

/// Kind-specific source structure for one generic declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeDefinitionData {
    /// An opaque declaration without navigable members.
    Opaque,
    /// A struct declaration and its fields.
    Struct {
        /// The declared struct shape.
        kind: StructKind,
        /// Fields in source order.
        fields: &'static [FieldDefinitionDescriptor],
    },
    /// An enum declaration and its variants.
    Enum {
        /// Variants in source order.
        variants: &'static [VariantDefinitionDescriptor],
    },
}
