// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Internal eager-or-lazy storage for one type relationship.

use crate::__private::LazyTypeRef;
use crate::descriptor::TypeRef;

/// The eager or deferred storage policy of one descriptor relationship.
#[derive(Clone, Copy, Debug)]
pub(crate) enum TypeRefSource {
    /// A relationship supplied as already constructed static data.
    Eager(&'static TypeRef),
    /// A relationship resolved only when public navigation reaches it.
    Lazy(&'static LazyTypeRef),
}

impl TypeRefSource {
    /// Returns the stable public relationship object.
    #[must_use]
    #[inline(always)]
    pub(crate) fn get(self) -> &'static TypeRef {
        match self {
            Self::Eager(reference) => reference,
            Self::Lazy(reference) => reference.get(),
        }
    }
}
