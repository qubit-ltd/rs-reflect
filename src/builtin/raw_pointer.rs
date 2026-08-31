// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Reflection descriptors for raw pointer types.

use crate::builtin::interner;
use crate::descriptor::Mutability;
use crate::descriptor::Reflect;
use crate::descriptor::TypeDescriptor;

impl<T: Reflect + ?Sized> Reflect for *const T {
    /// Returns the interned descriptor for this const raw-pointer
    /// specialization.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_raw_pointer_lazy::<Self>(
                std::any::type_name::<Self>(),
                Mutability::Const,
                crate::__private::descriptor::lazy_type_ref::<T>(),
            )
        })
    }
}

impl<T: Reflect + ?Sized> Reflect for *mut T {
    /// Returns the interned descriptor for this mutable raw-pointer
    /// specialization.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_raw_pointer_lazy::<Self>(
                std::any::type_name::<Self>(),
                Mutability::Mutable,
                crate::__private::descriptor::lazy_type_ref::<T>(),
            )
        })
    }
}
