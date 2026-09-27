// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Reflection descriptors for shared and mutable static references.

use std::any::type_name;

use crate::builtin::interner;
use crate::descriptor::ReferenceKind;
use crate::descriptor::Reflect;
use crate::descriptor::TypeDescriptor;

impl<T: Reflect + ?Sized> Reflect for &'static T {
    /// Returns the interned descriptor for this shared-reference
    /// specialization.
    ///
    /// # Returns
    ///
    /// The descriptor recording shared-reference semantics and the deferred
    /// target relationship.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_reference_lazy::<Self>(
                type_name::<Self>(),
                ReferenceKind::Shared,
                crate::__private::descriptor::lazy_type_ref::<T>(),
            )
        })
    }
}

impl<T: Reflect + ?Sized> Reflect for &'static mut T {
    /// Returns the interned descriptor for this mutable-reference
    /// specialization.
    ///
    /// # Returns
    ///
    /// The descriptor recording mutable-reference semantics and the deferred
    /// target relationship.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_reference_lazy::<Self>(
                type_name::<Self>(),
                ReferenceKind::Mutable,
                crate::__private::descriptor::lazy_type_ref::<T>(),
            )
        })
    }
}
