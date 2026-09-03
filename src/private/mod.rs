// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Internal dependencies used by generated reflection code.

/// Versioned protocol consumed by code generated for this release.
#[doc(hidden)]
pub mod codegen_v2;
/// Conformance hooks kept separate from the generated-code ABI.
#[doc(hidden)]
pub mod testing;

#[cfg(test)]
pub(crate) mod assertions;
pub(crate) mod descriptor;
mod lazy_type_ref;
mod lazy_type_ref_list;
pub(crate) mod registration;
mod type_ref_list_source;
mod type_ref_source;

pub(crate) use lazy_type_ref::LazyTypeRef;
pub(crate) use lazy_type_ref_list::LazyTypeRefList;
pub(crate) use type_ref_list_source::TypeRefListSource;
pub(crate) use type_ref_source::TypeRefSource;
