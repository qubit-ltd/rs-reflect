// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::associated_type_expression::AssociatedTypeExpression;
use super::data::ConstExpression;
use super::data::LifetimeExpression;
use super::function_pointer_expression::FunctionPointerExpression;
use super::named_type_expression::NamedTypeExpression;
use super::opaque_type_expression::OpaqueTypeExpression;
use super::trait_object_expression::TraitObjectExpression;
use super::type_expression::TypeExpression;
use crate::runtime::descriptor::generic::GenericParameterDescriptor;

/// The complete structural shape of a source-level Rust type expression.
pub enum TypeExpressionKind<'ctx> {
    /// A named path with source-level generic arguments.
    Named(
        /// Path expression and its optional definition link.
        NamedTypeExpression<'ctx>,
    ),
    /// A generic parameter linked to its declaration metadata.
    Parameter(
        /// Descriptor for the generic parameter declaration.
        GenericParameterDescriptor<'ctx>,
    ),
    /// A generic parameter name without a definition link.
    ParameterName(
        /// Source name retained when declaration metadata is unavailable.
        &'static str,
    ),
    /// The special `Self` type expression.
    SelfType,
    /// An associated-type projection.
    AssociatedType(
        /// Projection source, qualifier, and generic arguments.
        AssociatedTypeExpression<'ctx>,
    ),
    /// A borrowed type and its optional explicit lifetime.
    Reference {
        /// Lifetime written on the reference, when present.
        lifetime: Option<LifetimeExpression>,
        /// Whether the reference is mutable.
        mutable: bool,
        /// Type referenced by the borrow.
        target: TypeExpression<'ctx>,
    },
    /// A raw pointer and its mutability.
    RawPointer {
        /// Whether the pointed-to value is mutable.
        mutable: bool,
        /// Type pointed to by the raw pointer.
        target: TypeExpression<'ctx>,
    },
    /// A slice and its element type.
    Slice(
        /// Type of each slice element.
        TypeExpression<'ctx>,
    ),
    /// An array and its element type and structural length.
    Array {
        /// Type of each array element.
        element: TypeExpression<'ctx>,
        /// Recorded array length expression; it is not evaluated here.
        length: ConstExpression,
    },
    /// Tuple elements in source order.
    Tuple(
        /// Element expressions in their original source order.
        Box<[TypeExpression<'ctx>]>,
    ),
    /// A function pointer signature.
    FunctionPointer(
        /// Signature, ABI, and qualifiers retained for the function pointer.
        FunctionPointerExpression<'ctx>,
    ),
    /// A trait-object bound list.
    TraitObject(
        /// Trait bounds and object lifetime retained from source syntax.
        TraitObjectExpression<'ctx>,
    ),
    /// Bounds of an opaque `impl Trait` type.
    Opaque(
        /// Bounds retained for the opaque `impl Trait` type.
        OpaqueTypeExpression<'ctx>,
    ),
    /// The never type `!`.
    Never,
}
