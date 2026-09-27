// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Validated paths used by structural expressions.

use crate::expression::ExpressionError;
use crate::expression::ExpressionName;

/// A non-empty sequence of non-empty structural path segments.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::ExpressionPath;
/// let path = ExpressionPath::new(["std", "vec", "Vec"]).expect("valid path");
/// assert_eq!(path.segments().len(), 3);
/// ```
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ExpressionPath(Box<[ExpressionName]>);

impl ExpressionPath {
    /// Creates a validated structural path.
    ///
    /// # Type Parameters
    ///
    /// - `P`: Iterable collection of path segments.
    /// - `S`: Segment value convertible to owned text.
    ///
    /// # Parameters
    ///
    /// - `segments`: Non-empty path segments in source order.
    ///
    /// # Returns
    ///
    /// Returns a validated path.
    ///
    /// # Errors
    ///
    /// Returns [`ExpressionError::EmptyPath`] when no segments are supplied,
    /// or [`ExpressionError::EmptyPathSegment`] with the offending index when
    /// any segment is empty.
    pub fn new<P, S>(segments: P) -> Result<Self, ExpressionError>
    where
        P: IntoIterator<Item = S>,
        S: Into<Box<str>>,
    {
        let mut validated = Vec::new();
        for (index, segment) in segments.into_iter().enumerate() {
            let segment = segment.into();
            if segment.is_empty() {
                return Err(ExpressionError::EmptyPathSegment { index });
            }
            validated.push(ExpressionName::new(segment).expect("an empty segment was rejected"));
        }
        if validated.is_empty() {
            return Err(ExpressionError::EmptyPath);
        }
        Ok(Self(validated.into_boxed_slice()))
    }

    /// Returns the validated path segments in source order.
    ///
    /// # Returns
    ///
    /// Returns all non-empty path segments.
    #[must_use]
    #[inline]
    pub fn segments(&self) -> &[ExpressionName] {
        &self.0
    }
}
