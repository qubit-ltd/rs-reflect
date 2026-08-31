// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Reflection descriptors for unsized slice types.

use crate::builtin::interner;
use crate::descriptor::Reflect;
use crate::descriptor::TypeDescriptor;

impl<T: Reflect> Reflect for [T] {
    /// Returns the interned descriptor for this slice specialization.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_slice_lazy::<Self>(
                std::any::type_name::<Self>(),
                crate::__private::descriptor::lazy_type_ref::<T>(),
            )
        })
    }
}
