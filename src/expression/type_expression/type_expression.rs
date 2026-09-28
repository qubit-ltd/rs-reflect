// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural representations of Rust type expressions.

//! TypeExpression structure and operations.

use crate::expression::ArrayTypeExpression;
use crate::expression::AssociatedTypeExpression;
use crate::expression::ConcreteTypeExpression;
use crate::expression::ExpressionError;
use crate::expression::ExpressionName;
use crate::expression::FunctionPointerExpression;
use crate::expression::OpaqueTypeExpression;
use crate::expression::RawPointerTypeExpression;
use crate::expression::ReferenceTypeExpression;
use crate::expression::TraitObjectExpression;

/// A closed, navigable Rust type expression independent of parser
/// implementation types.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::TypeExpression;
/// let ty = TypeExpression::parameter("Item").expect("valid parameter");
/// assert!(matches!(ty, TypeExpression::Parameter(_)));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeExpression {
    /// A concrete path such as `std::vec::Vec<T>`.
    Concrete(ConcreteTypeExpression),
    /// A type parameter such as `T`.
    Parameter(ExpressionName),
    /// The `Self` type.
    SelfType,
    /// An associated type projection such as `<T as Trait>::Item`.
    Associated(AssociatedTypeExpression),
    /// A shared or mutable reference.
    Reference(ReferenceTypeExpression),
    /// A const or mutable raw pointer.
    RawPointer(RawPointerTypeExpression),
    /// A slice expression such as `[T]`.
    Slice(Box<TypeExpression>),
    /// An array expression such as `[T; N]`.
    Array(ArrayTypeExpression),
    /// A tuple expression, including the empty tuple.
    Tuple(Box<[TypeExpression]>),
    /// A function pointer expression.
    FunctionPointer(FunctionPointerExpression),
    /// A `dyn Trait` object expression.
    TraitObject(TraitObjectExpression),
    /// An `impl Trait` opaque expression.
    Opaque(OpaqueTypeExpression),
    /// The never type `!`.
    Never,
}

impl TypeExpression {
    /// Creates a named type-parameter expression.
    ///
    /// # Parameters
    ///
    /// - `name`: Type parameter identifier.
    ///
    /// # Returns
    ///
    /// Returns the parameter expression.
    ///
    /// # Errors
    ///
    /// Returns [`ExpressionError::EmptyName`] when `name` is empty.
    pub fn parameter(name: impl Into<Box<str>>) -> Result<Self, ExpressionError> {
        ExpressionName::new(name).map(Self::Parameter)
    }
}
