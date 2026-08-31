// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Reflection descriptors for ordered sequence collections.

use crate::builtin::interner;
use crate::descriptor::Reflect;
use crate::descriptor::SequenceKind;
use crate::descriptor::TypeDescriptor;

impl<T: Reflect> Reflect for Vec<T> {
    /// Returns the interned descriptor for this vector specialization.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_sequence_lazy::<Self>(
                std::any::type_name::<Self>(),
                SequenceKind::Vec,
                crate::__private::descriptor::lazy_type_ref::<T>(),
            )
        })
    }
}
