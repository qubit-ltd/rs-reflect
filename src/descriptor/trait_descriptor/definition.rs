// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Trait declarations and their associated items.

mod associated_const_descriptor;
mod associated_type_descriptor;
mod trait_completeness;
mod trait_definition_descriptor;

pub use associated_const_descriptor::AssociatedConstDescriptor;
pub use associated_type_descriptor::AssociatedTypeDescriptor;
pub use trait_completeness::TraitCompleteness;
pub use trait_definition_descriptor::TraitDefinitionDescriptor;
