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
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ExpressionPath(Box<[ExpressionName]>);

impl ExpressionPath {
    /// Creates a validated structural path.
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
    #[must_use]
    #[inline(always)]
    pub fn segments(&self) -> &[ExpressionName] {
        &self.0
    }
}
