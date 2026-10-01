// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Reflection descriptors for optional values.

use std::any::type_name;

use crate::builtin::interner;
use crate::descriptor::Reflect;
use crate::descriptor::TypeDescriptor;

impl<T: Reflect> Reflect for Option<T> {
    /// Returns the interned descriptor for this optional-value specialization.
    ///
    /// # Returns
    ///
    /// The shared descriptor with a deferred relationship to `T`.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_optional_lazy::<T>(
                type_name::<Self>(),
                crate::__private::descriptor::lazy_type_ref::<T>(),
                crate::descriptor::project_option_ref::<T>,
            )
        })
    }
}
