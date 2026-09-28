// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! PrimitiveKind category metadata.

/// A Rust primitive represented by a root descriptor.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::PrimitiveKind;
/// let kind = PrimitiveKind::Bool;
/// assert_eq!(kind, PrimitiveKind::Bool);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PrimitiveKind {
    /// `bool`.
    Bool,
    /// `char`.
    Char,
    /// `i8`.
    I8,
    /// `i16`.
    I16,
    /// `i32`.
    I32,
    /// `i64`.
    I64,
    /// `i128`.
    I128,
    /// `isize`.
    Isize,
    /// `u8`.
    U8,
    /// `u16`.
    U16,
    /// `u32`.
    U32,
    /// `u64`.
    U64,
    /// `u128`.
    U128,
    /// `usize`.
    Usize,
    /// `f32`.
    F32,
    /// `f64`.
    F64,
}
