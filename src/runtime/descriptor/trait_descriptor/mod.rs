// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Context-bound trait declarations and applications.

mod descriptor;
mod metadata;

pub use descriptor::AssociatedConstDefinitionDescriptor;
pub use descriptor::AssociatedConstDefinitions;
pub use descriptor::AssociatedTypeDefinitionDescriptor;
pub use descriptor::AssociatedTypeDefinitions;
pub use descriptor::MethodDefinitionDescriptor;
pub use descriptor::MethodDefinitionOwner;
pub use descriptor::MethodDefinitions;
pub use descriptor::ReceiverDefinitionDescriptor;
pub use descriptor::TraitDefinitionDescriptor;
pub use descriptor::TraitDescriptor;
pub use descriptor::TraitExpressionDescriptor;
pub use descriptor::TraitExpressions;
pub use descriptor::Traits;
pub use metadata::AssociatedConstDefinitionMetadata;
pub use metadata::AssociatedTypeDefinitionMetadata;
pub use metadata::SupertraitCompleteness;
pub use metadata::TraitApplicationMetadata;
pub use metadata::TraitArgumentIdentity;
pub use metadata::TraitCompleteness;
pub use metadata::TraitDefinitionId;
pub use metadata::TraitDefinitionMetadata;
pub use metadata::TraitExpressionMetadata;
pub use metadata::Visibility;
