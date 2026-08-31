// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Internal dependencies used by generated reflection code.

/// Hidden compile-time assertion helpers used by generated code.
#[doc(hidden)]
pub mod assertions;
/// Hidden static descriptor factories shared with generated reflection code.
#[doc(hidden)]
pub mod descriptor;
/// Hidden lazy relation storage shared with generated reflection code.
#[doc(hidden)]
pub mod lazy_type_ref;
mod lazy_type_ref_list;
/// Hidden distributed-registration protocol used by generated code.
#[doc(hidden)]
pub mod registration;
mod type_ref_list_source;
mod type_ref_source;

#[doc(hidden)]
pub use descriptor::intern_type;
pub use inventory;
#[doc(hidden)]
pub use lazy_type_ref::LazyTypeRef;
pub(crate) use lazy_type_ref_list::LazyTypeRefList;
#[doc(hidden)]
pub use registration::FragmentKind;
#[doc(hidden)]
pub use registration::FragmentPayload;
#[doc(hidden)]
pub use registration::RegistrationFragment;
#[doc(hidden)]
pub use registration::RuntimeIdentity;
#[doc(hidden)]
pub use registration::StaticFragmentIdentity;
pub(crate) use type_ref_list_source::TypeRefListSource;
pub(crate) use type_ref_source::TypeRefSource;

#[doc(hidden)]
pub use crate::descriptor::TraitImplPayload;
#[doc(hidden)]
pub use crate::descriptor::cached_trait_object_descriptor;
#[doc(hidden)]
pub use crate::descriptor::external_supertrait;
