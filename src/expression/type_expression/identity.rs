// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural equality and hashing helpers for diagnostic-bearing expressions.

macro_rules! impl_identity_without_diagnostic {
    ($type:ty { $first:ident $(, $field:ident)* $(,)? }) => {
        impl std::cmp::PartialEq for $type {
            fn eq(&self, other: &Self) -> bool {
                self.$first == other.$first $(&& self.$field == other.$field)*
            }
        }

        impl std::cmp::Eq for $type {}

        impl std::hash::Hash for $type {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                std::hash::Hash::hash(&self.$first, state);
                $(std::hash::Hash::hash(&self.$field, state);)*
            }
        }
    };
}
