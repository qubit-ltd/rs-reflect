// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! TypeKind category metadata.

use super::FunctionPointerKind;
use super::Mutability;
use super::PrimitiveKind;
use super::ReferenceKind;
use super::SmartPointerKind;
use super::StructKind;
use super::TextKind;

/// The stable top-level category of a reflected Rust type.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::{PrimitiveKind, TypeKind};
/// let kind = TypeKind::Primitive(PrimitiveKind::Bool);
/// assert!(matches!(kind, TypeKind::Primitive(PrimitiveKind::Bool)));
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TypeKind {
    /// A scalar primitive.
    Primitive(PrimitiveKind),
    /// An owned or borrowed UTF-8 text type.
    Text(TextKind),
    /// A declared struct and its precise shape.
    Struct(StructKind),
    /// A declared enum.
    Enum,
    /// A Rust tuple, including `()`.
    Tuple,
    /// A fixed-length array.
    Array,
    /// An optional value.
    Optional,
    /// An ordered sequence.
    Sequence,
    /// A set.
    Set,
    /// A key-value map.
    Map,
    /// A standard smart pointer.
    SmartPointer(SmartPointerKind),
    /// A shared or mutable reference.
    Reference(ReferenceKind),
    /// An unsized slice.
    Slice,
    /// A raw pointer.
    RawPointer(Mutability),
    /// A function pointer.
    FunctionPointer(FunctionPointerKind),
    /// A dyn-compatible trait object.
    TraitObject,
    /// A root type whose internal shape is intentionally hidden.
    Opaque,
}
