// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Context-bound implementation declarations and concrete applications.

mod descriptor;
pub(crate) mod metadata;

pub use descriptor::AssociatedConstBindingDescriptor;
pub use descriptor::AssociatedConstBindings;
pub use descriptor::AssociatedTypeBindingDescriptor;
pub use descriptor::AssociatedTypeBindings;
pub use descriptor::ImplAssociatedConstDefinitions;
pub use descriptor::ImplAssociatedTypeDefinitions;
pub use descriptor::ImplDefinitionDescriptor;
pub use descriptor::ImplDefinitions;
pub use descriptor::ImplDescriptor;
pub use descriptor::Impls;
pub use descriptor::Methods;
pub use metadata::AssociatedConstBindingMetadata;
pub use metadata::AssociatedItemSource;
pub use metadata::AssociatedTypeBindingMetadata;
pub use metadata::ImplApplicationMetadata;
pub use metadata::ImplDefinitionId;
pub use metadata::ImplDefinitionMetadata;
pub use metadata::ImplKind;
pub use metadata::ImplQualifiers;
