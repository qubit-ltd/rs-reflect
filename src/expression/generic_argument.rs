// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Generic arguments and structural const expressions.

use std::hash::Hash;
use std::hash::Hasher;

use crate::expression::ExpressionError;
use crate::expression::ExpressionName;
use crate::expression::ExpressionPath;
use crate::expression::LifetimeExpression;
use crate::expression::PredicateDescriptor;
use crate::expression::TypeExpression;

/// An argument applied to a generic path segment.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GenericArgument {
    /// A type argument such as `Vec<u8>`'s `u8`.
    Type(TypeExpression),
    /// A lifetime argument such as `Ref<'a, T>`'s `'a`.
    Lifetime(LifetimeExpression),
    /// A const argument such as `Array<T, 4>`'s `4`.
    Const(ConstGenericArgument),
    /// An associated type equality such as `Iterator<Item = T>`.
    AssociatedType {
        /// The associated type name.
        name: ExpressionName,
        /// The associated type value.
        value: Box<TypeExpression>,
    },
    /// An associated type bound such as `Iterator<Item: Display>`.
    AssociatedTypeBound {
        /// The associated type name.
        name: ExpressionName,
        /// Bounds that apply to the associated type.
        bounds: Box<[PredicateDescriptor]>,
    },
}

/// A typed const argument applied to a generic parameter.
///
/// The declared type and structural value determine identity. The normalized
/// text is retained only for diagnostics and is deliberately excluded from
/// equality and hashing.
#[derive(Clone, Debug)]
pub struct ConstGenericArgument {
    /// The const parameter's declared type.
    pub(crate) declared_type: Box<TypeExpression>,
    /// The const argument's structural value.
    pub(crate) value: ConstExpression,
    /// A normalized, source-oriented rendering of the const argument.
    pub(crate) normalized_diagnostic: Box<str>,
}

impl ConstGenericArgument {
    /// Creates a typed structural const argument.
    pub fn new(
        declared_type: TypeExpression,
        value: ConstExpression,
        normalized_diagnostic: impl Into<Box<str>>,
    ) -> Self {
        Self {
            declared_type: Box::new(declared_type),
            value,
            normalized_diagnostic: normalized_diagnostic.into(),
        }
    }

    /// Returns the declared const parameter type.
    pub fn declared_type(&self) -> &TypeExpression {
        &self.declared_type
    }
    /// Returns the structural const value.
    pub fn value(&self) -> &ConstExpression {
        &self.value
    }
    /// Returns the normalized source-oriented rendering.
    pub fn normalized_diagnostic(&self) -> &str {
        &self.normalized_diagnostic
    }
}

impl PartialEq for ConstGenericArgument {
    fn eq(&self, other: &Self) -> bool {
        self.declared_type == other.declared_type && self.value == other.value
    }
}

impl Eq for ConstGenericArgument {}

impl Hash for ConstGenericArgument {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.declared_type.hash(state);
        self.value.hash(state);
    }
}

/// A structural const expression used by a generic argument, array length, or
/// const default.
///
/// Values are stored as typed data rather than source tokens.  A path can
/// identify a named const item or parameter, but this descriptor does not
/// attempt runtime const evaluation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstExpression {
    /// A signed integer value.
    SignedInteger(i128),
    /// An unsigned integer value.
    UnsignedInteger(u128),
    /// A boolean literal.
    Boolean(bool),
    /// A character literal.
    Character(char),
    /// A named const generic parameter.
    Parameter(ExpressionName),
    /// A qualified path to a const item.
    Path(ExpressionPath),
}

impl ConstExpression {
    /// Creates a named const-parameter expression.
    ///
    /// # Errors
    ///
    /// Returns [`ExpressionError::EmptyName`] when `name` is empty.
    pub fn parameter(name: impl Into<Box<str>>) -> Result<Self, ExpressionError> {
        ExpressionName::new(name).map(Self::Parameter)
    }

    /// Creates a qualified const-item path.
    ///
    /// # Errors
    ///
    /// Returns an expression error when the path is empty or contains an empty
    /// segment.
    pub fn path<P, S>(segments: P) -> Result<Self, ExpressionError>
    where
        P: IntoIterator<Item = S>,
        S: Into<Box<str>>,
    {
        ExpressionPath::new(segments).map(Self::Path)
    }
}
