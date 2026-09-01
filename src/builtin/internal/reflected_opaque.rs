// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Generates opaque roots for external types behind opt-in features.

macro_rules! reflected_opaque {
    ($type:ty, $descriptor:ident, $name:expr) => {
        static $descriptor: $crate::descriptor::TypeDescriptor =
            $crate::__private::descriptor::opaque_root::<$type>($name);

        impl $crate::descriptor::Reflect for $type {
            /// Returns the static opaque descriptor for this external value
            /// type.
            fn type_descriptor() -> &'static $crate::descriptor::TypeDescriptor {
                &$descriptor
            }
        }
    };
}

pub(crate) use reflected_opaque;
