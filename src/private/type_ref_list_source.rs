// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Internal eager-or-lazy storage for ordered type relationships.

use crate::__private::LazyTypeRefList;
use crate::descriptor::TypeRef;

/// The eager or deferred storage policy of an ordered relationship list.
#[derive(Clone, Copy, Debug)]
pub(crate) enum TypeRefListSource {
    /// Relationships supplied as already constructed static data.
    Eager(&'static [TypeRef]),
    /// Relationships resolved together on first public navigation.
    Lazy(&'static LazyTypeRefList),
}

impl TypeRefListSource {
    /// Returns the stable public relationship slice.
    #[must_use]
    #[inline(always)]
    pub(crate) fn get(self) -> &'static [TypeRef] {
        match self {
            Self::Eager(references) => references,
            Self::Lazy(references) => references.get(),
        }
    }
}
