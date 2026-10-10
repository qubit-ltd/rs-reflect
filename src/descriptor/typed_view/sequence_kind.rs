// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Identifies a standard ordered-sequence family.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::SequenceKind;
/// let kind = SequenceKind::Vec;
/// assert!(matches!(kind, SequenceKind::Vec));
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SequenceKind {
    /// [`Vec<T>`].
    Vec,
}
