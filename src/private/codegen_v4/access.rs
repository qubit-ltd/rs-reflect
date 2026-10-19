// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed field and variant access adapters emitted by derive.
//!
//! The runtime counterparts are `FieldMetadata`, `TypeShapeRegistration`,
//! `FieldAdapter`, `DynamicFieldAdapter`, `VariantMetadata`, and
//! `VariantAdapter` in `runtime::descriptor` and `runtime::access`. Derive needs
//! safe typed constructors that keep each adapter tied to the owner and value
//! types. These internal traits and metadata are currently crate-private.

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
pub use crate::runtime::descriptor::variant::VariantAdapter;
pub use crate::runtime::descriptor::variant::VariantDiscriminant;
pub use crate::runtime::descriptor::variant::VariantMetadata;
pub use crate::runtime::descriptor::variant::VariantShape;

/// Submits a type's structural shape, with the target type fixed at compile time.
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
