// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Stable, hierarchical categories for reflected Rust types.

mod function_pointer_kind;
mod mutability;
mod primitive_kind;
mod reference_kind;
mod smart_pointer_kind;
mod struct_kind;
mod text_kind;
mod type_kind;

pub use function_pointer_kind::FunctionPointerKind;
pub use mutability::Mutability;
pub use primitive_kind::PrimitiveKind;
pub use reference_kind::ReferenceKind;
pub use smart_pointer_kind::SmartPointerKind;
pub use struct_kind::StructKind;
pub use text_kind::TextKind;
pub use type_kind::TypeKind;
