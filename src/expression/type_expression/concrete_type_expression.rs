// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural representations of Rust type expressions.

//! ConcreteTypeExpression structure and operations.

use crate::expression::ConcretePathSegment;
use crate::expression::DiagnosticText;
use crate::expression::ExpressionError;
use crate::expression::GenericArgument;

/// A concrete type path and its final-segment generic arguments.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::ConcreteTypeExpression;
/// let ty = ConcreteTypeExpression::new(["std", "vec", "Vec"], [])
///     .expect("non-empty path");
/// assert_eq!(ty.path().last().map(|part| part.as_ref()), Some("Vec"));
/// ```
#[derive(Clone, Debug)]
pub struct ConcreteTypeExpression {
    /// Path segments in declaration order, for example `std`, `vec`, and
    /// `Vec`.
    pub(crate) path: Box<[Box<str>]>,
    /// Generic arguments of the final path segment in declaration order.
    pub(crate) arguments: Box<[GenericArgument]>,
    /// Structural path segments with arguments retained at their source site.
    pub(crate) segments: Box<[ConcretePathSegment]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl ConcreteTypeExpression {
    /// Creates a concrete type expression from a non-empty path.
    ///
    /// # Type Parameters
    ///
    /// - `P`: Iterable collection of path segments.
    /// - `S`: Segment value convertible to owned text.
    ///
    /// # Parameters
    ///
    /// - `path`: Non-empty concrete path in declaration order.
    /// - `arguments`: Generic arguments applied to the final path segment.
    ///
    /// # Returns
    ///
    /// Returns the validated concrete type expression.
    ///
    /// # Errors
    ///
    /// Returns [`ExpressionError::EmptyConcretePath`] for an empty path or
    /// [`ExpressionError::EmptyPathSegment`] when a path segment is empty.
    pub fn new<P, S>(path: P, arguments: impl IntoIterator<Item = GenericArgument>) -> Result<Self, ExpressionError>
    where
        P: IntoIterator<Item = S>,
        S: Into<Box<str>>,
    {
        let path = path.into_iter().map(Into::into).collect::<Box<[_]>>();
        if path.is_empty() {
            return Err(ExpressionError::EmptyConcretePath);
        }
        if let Some(index) = path.iter().position(|segment| segment.is_empty()) {
            return Err(ExpressionError::EmptyPathSegment { index });
        }
        let arguments = arguments.into_iter().collect::<Box<[_]>>();
        let mut segments = path
            .iter()
            .map(|name| ConcretePathSegment::new(name.clone(), Box::default()))
            .collect::<Vec<_>>();
        if let Some(last) = segments.last_mut() {
            let name = last.name().to_owned();
            *last = ConcretePathSegment::new(name, arguments.clone());
        }
        Ok(Self {
            path,
            arguments,
            segments: segments.into_boxed_slice(),
            diagnostic: DiagnosticText::default(),
        })
    }

    /// Creates a concrete type expression from structural path segments.
    ///
    /// # Parameters
    ///
    /// - `segments`: Non-empty path segments with arguments attached at their
    ///   source positions.
    ///
    /// # Returns
    ///
    /// Returns the validated concrete type expression.
    ///
    /// # Errors
    ///
    /// Returns [`ExpressionError::EmptyConcretePath`] for no segments or
    /// [`ExpressionError::EmptyPathSegment`] when a segment name is empty.
    pub fn from_segments(segments: impl IntoIterator<Item = ConcretePathSegment>) -> Result<Self, ExpressionError> {
        let segments = segments.into_iter().collect::<Box<[_]>>();
        if segments.is_empty() {
            return Err(ExpressionError::EmptyConcretePath);
        }
        if let Some(index) = segments.iter().position(|segment| segment.name().is_empty()) {
            return Err(ExpressionError::EmptyPathSegment { index });
        }
        let path = segments
            .iter()
            .map(|segment| Box::<str>::from(segment.name()))
            .collect();
        let arguments = segments
            .last()
            .map_or_else(Box::default, |segment| segment.arguments().to_vec().into_boxed_slice());
        Ok(Self {
            path,
            arguments,
            segments,
            diagnostic: DiagnosticText::default(),
        })
    }

    /// Returns the path segments in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the path names without their segment-local arguments.
    #[must_use]
    pub fn path(&self) -> &[Box<str>] {
        &self.path
    }

    /// Returns final-segment generic arguments in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the generic arguments on the final path segment.
    #[must_use]
    #[inline]
    pub fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Returns structural path segments with their local generic arguments.
    ///
    /// # Returns
    ///
    /// Returns every path segment with its own arguments retained.
    #[must_use]
    pub fn segments(&self) -> &[ConcretePathSegment] {
        &self.segments
    }

    /// Returns source-oriented diagnostic text when present.
    ///
    /// # Returns
    ///
    /// Returns source-oriented text, or `None` when absent.
    #[must_use]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }

    /// Attaches source-oriented diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `diagnostic`: Source-oriented rendering for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the expression with diagnostic text attached; structural
    /// identity is unchanged.
    #[must_use]
    pub fn with_diagnostic(mut self, diagnostic: impl Into<Box<str>>) -> Self {
        self.diagnostic = DiagnosticText::from(diagnostic.into());
        self
    }
}

impl_identity_without_diagnostic!(ConcreteTypeExpression { segments });
