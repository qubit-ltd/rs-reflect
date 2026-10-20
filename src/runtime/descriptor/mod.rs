// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// These descriptors preserve structured, lossless failures by value and their
// hidden macro support methods expose crate-private adapters in public bounds.
#![allow(private_interfaces, clippy::result_large_err)]

pub(crate) mod collections;
pub(crate) mod field;
pub(crate) mod generic;
pub(crate) mod impl_descriptor;
pub(crate) mod metadata;
pub(crate) mod method;
pub(crate) mod reflect;
pub(crate) mod trait_descriptor;
pub(crate) mod type_descriptor;
pub(crate) mod type_kind;
pub(crate) mod variant;

pub use generic::GenericArgumentMetadata;
pub use generic::GenericDefinitionMetadata;
pub use generic::GenericParameterMetadata;
pub use generic::GenericParameterMetadataKind;
pub use generic::StructKind;
pub use generic::TypeDefinitionKind;
pub use generic::TypeDefinitionMetadata;
pub(crate) use reflect::Reflect;
pub(crate) use reflect::TypeRegistration;
pub(crate) use type_descriptor::TypeDescriptor;
pub(crate) use variant::VariantDescriptor;
