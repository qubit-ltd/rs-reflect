// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Versioned field and variant access contracts used by generated code.
//!
//! The runtime implementation modules are private. This module re-exports
//! metadata, access results, and typed adapter constructors such as
//! `FieldMetadata`, `TypeShapeRegistration`, `TypedFieldAdapter`, and
//! `TypedVariantAdapter`; `VariantAdapter` is also re-exported. The erased
//! `FieldAdapter` and `DynamicFieldAdapter` traits are not re-exported. These
//! names are accessible to generated code through the hidden v4 namespace, but
//! accessibility alone does not make every item a supported compatibility
//! contract. The supported protocol surface is documented in the architecture
//! and user guides.

pub use crate::runtime::descriptor::collections::TypeShapeRegistration;
pub use crate::runtime::descriptor::field::FieldAccess;
pub use crate::runtime::descriptor::field::FieldAccessError;
pub use crate::runtime::descriptor::field::FieldAccessUnavailableReason;
pub use crate::runtime::descriptor::field::FieldMetadata;
pub use crate::runtime::descriptor::field::FieldSetError;
pub use crate::runtime::descriptor::field::FieldSetFailure;
pub use crate::runtime::descriptor::field::Visibility;
pub use crate::runtime::descriptor::type_kind::TypeKindMetadata;
pub use crate::runtime::descriptor::variant::DiscriminantOrigin;
pub use crate::runtime::descriptor::variant::NumericDiscriminant;
pub use crate::runtime::descriptor::variant::TypedVariantAdapter;
pub use crate::runtime::descriptor::variant::VariantAdapter;
pub use crate::runtime::descriptor::variant::VariantDiscriminant;
pub use crate::runtime::descriptor::variant::VariantIdentity;
pub use crate::runtime::descriptor::variant::VariantMetadata;
pub use crate::runtime::descriptor::variant::VariantShape;

/// Submits a type's structural shape, with the target type fixed at compile
/// time.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_submit_type_shape {
    ($ty:ty, $shape:expr) => {
        $crate::__private::__inventory::submit! {
            $crate::__private::codegen_v4::access::TypeShapeRegistration::new::<$ty>($shape)
        }
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_submit_type_shape as submit_type_shape;
pub use crate::runtime::access::TypedFieldAdapter;
