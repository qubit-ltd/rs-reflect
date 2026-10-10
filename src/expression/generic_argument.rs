// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

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
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::{GenericArgument, TypeExpression};
/// let argument = GenericArgument::Type(TypeExpression::parameter("Item").expect("valid parameter"));
/// assert!(matches!(argument, GenericArgument::Type(_)));
/// ```
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
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::{ConstExpression, ConstGenericArgument, TypeExpression};
///
/// let argument = ConstGenericArgument::new(
///     TypeExpression::parameter("usize").expect("valid type parameter"),
///     ConstExpression::UnsignedInteger(4),
///     "4",
/// );
/// assert_eq!(argument.normalized_diagnostic(), "4");
/// ```
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
    ///
    /// # Parameters
    ///
    /// - `declared_type`: Type declared for the const generic parameter.
    /// - `value`: Structural const expression supplied as the argument.
    /// - `normalized_diagnostic`: Source-oriented text retained for
    ///   diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the typed const argument. Diagnostic text does not affect
    /// identity.
    #[must_use]
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
    ///
    /// # Returns
    ///
    /// Returns the type declared for this const argument.
    #[must_use]
    #[inline]
    pub fn declared_type(&self) -> &TypeExpression {
        &self.declared_type
    }

    /// Returns the structural const value.
    ///
    /// # Returns
    ///
    /// Returns the const expression supplied as this argument.
    #[must_use]
    #[inline]
    pub fn value(&self) -> &ConstExpression {
        &self.value
    }

    /// Returns the normalized source-oriented rendering.
    ///
    /// # Returns
    ///
    /// Returns the retained diagnostic rendering.
    #[must_use]
    #[inline]
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
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::ConstExpression;
/// let parameter = ConstExpression::parameter("COUNT").expect("valid parameter");
/// assert!(matches!(parameter, ConstExpression::Parameter(_)));
/// ```
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
    /// # Parameters
    ///
    /// - `name`: Const parameter identifier.
    ///
    /// # Returns
    ///
    /// Returns a parameter expression.
    ///
    /// # Errors
    ///
    /// Returns [`ExpressionError::EmptyName`] when `name` is empty.
    pub fn parameter(name: impl Into<Box<str>>) -> Result<Self, ExpressionError> {
        ExpressionName::new(name).map(Self::Parameter)
    }

    /// Creates a qualified const-item path.
    ///
    /// # Type Parameters
    ///
    /// - `P`: Iterable collection of path segments.
    /// - `S`: Segment value convertible to owned text.
    ///
    /// # Parameters
    ///
    /// - `segments`: Qualified const item path.
    ///
    /// # Returns
    ///
    /// Returns a path expression.
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
