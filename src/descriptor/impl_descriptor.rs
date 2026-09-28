// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Reflected inherent and trait implementation descriptors.

mod associated_const_binding_descriptor;
mod associated_const_implementation_source;
mod associated_const_read_unavailable_reason;
mod associated_const_reader;
mod associated_type_binding_descriptor;
mod impl_associated_const_descriptor;
mod impl_associated_type_descriptor;
mod impl_definition_descriptor;
mod impl_descriptor_build_error;
mod impl_descriptor_builder;
#[path = "impl_descriptor/impl_descriptor.rs"]
mod impl_descriptor_impl;
mod impl_kind;
mod method_lookup;
mod method_qualifier;

pub use associated_const_binding_descriptor::AssociatedConstBindingDescriptor;
pub use associated_const_implementation_source::AssociatedConstImplementationSource;
pub use associated_const_read_unavailable_reason::AssociatedConstReadUnavailableReason;
pub use associated_const_reader::AssociatedConstReader;
pub use associated_type_binding_descriptor::AssociatedTypeBindingDescriptor;
pub use impl_associated_const_descriptor::ImplAssociatedConstDescriptor;
pub use impl_associated_type_descriptor::ImplAssociatedTypeDescriptor;
pub use impl_definition_descriptor::ImplDefinitionDescriptor;
pub use impl_descriptor_build_error::ImplDescriptorBuildError;
pub use impl_descriptor_impl::ImplDescriptor;
pub use impl_kind::ImplKind;
pub use method_lookup::MethodLookup;
pub use method_qualifier::MethodQualifier;
