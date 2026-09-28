// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Supertrait closure views and process-local descriptor caches.

mod cache;
mod supertrait_closure;
mod trait_descriptor_ref;

pub use cache::cached_trait_object_descriptor;
pub use cache::external_supertrait;
pub use supertrait_closure::SupertraitClosure;
pub use trait_descriptor_ref::TraitDescriptorRef;
