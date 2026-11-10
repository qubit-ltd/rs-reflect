// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::descriptor::TypeDescriptor;
use crate::descriptor::VariantDescriptor;

/// The reflected target selected for a construction operation.
#[derive(Clone, Copy)]
pub enum ConstructionTarget<'ctx> {
    /// A reflected struct or tuple struct.
    Struct(
        /// Descriptor whose type is the construction target.
        TypeDescriptor<'ctx>,
    ),
    /// An explicitly selected enum variant.
    Variant(
        /// Descriptor of the variant selected for construction.
        VariantDescriptor<'ctx>,
    ),
}
