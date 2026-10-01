// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Kind-specific, immutable views of root type descriptors.

mod array_type_descriptor;
mod enum_repr;
mod enum_type_descriptor;
mod function_type_descriptor;
mod map_kind;
mod map_type_descriptor;
mod opaque_type_view;
mod optional_projection_error;
mod optional_type_descriptor;
mod primitive_type_descriptor;
mod raw_pointer_type_descriptor;
mod reference_type_descriptor;
mod sequence_kind;
mod sequence_type_descriptor;
mod set_kind;
mod set_type_descriptor;
mod slice_type_descriptor;
mod smart_pointer_type_descriptor;
mod struct_type_descriptor;
mod text_type_descriptor;
mod trait_object_type_descriptor;
mod tuple_type_descriptor;

pub use array_type_descriptor::ArrayTypeDescriptor;
pub use enum_repr::EnumRepr;
pub use enum_type_descriptor::EnumTypeDescriptor;
pub use function_type_descriptor::FunctionTypeDescriptor;
pub use map_kind::MapKind;
pub use map_type_descriptor::MapTypeDescriptor;
pub use opaque_type_view::OpaqueTypeView;
pub use optional_projection_error::OptionalProjectionError;
pub(crate) use optional_type_descriptor::OptionalRefProjector;
pub use optional_type_descriptor::OptionalTypeDescriptor;
pub(crate) use optional_type_descriptor::project_option_ref;
pub use primitive_type_descriptor::PrimitiveTypeDescriptor;
pub use raw_pointer_type_descriptor::RawPointerTypeDescriptor;
pub use reference_type_descriptor::ReferenceTypeDescriptor;
pub use sequence_kind::SequenceKind;
pub use sequence_type_descriptor::SequenceTypeDescriptor;
pub use set_kind::SetKind;
pub use set_type_descriptor::SetTypeDescriptor;
pub use slice_type_descriptor::SliceTypeDescriptor;
pub use smart_pointer_type_descriptor::SmartPointerTypeDescriptor;
pub use struct_type_descriptor::StructTypeDescriptor;
pub use text_type_descriptor::TextTypeDescriptor;
pub use trait_object_type_descriptor::TraitObjectTypeDescriptor;
pub use tuple_type_descriptor::TupleTypeDescriptor;
