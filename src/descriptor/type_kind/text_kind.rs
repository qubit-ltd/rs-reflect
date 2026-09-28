// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! TextKind category metadata.

/// A UTF-8 text representation.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::TextKind;
/// let text = TextKind::String;
/// assert_eq!(text, TextKind::String);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TextKind {
    /// An owned [`String`].
    String,
    /// A borrowed `str` slice.
    Str,
}
