// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Exact integer forms of reflected enum discriminants.

/// The exact integer representation of an enum discriminant value.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::NumericDiscriminant;
/// assert_eq!(NumericDiscriminant::U8(3), NumericDiscriminant::U8(3));
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NumericDiscriminant {
    /// An `i8` discriminant.
    I8(i8),
    /// An `i16` discriminant.
    I16(i16),
    /// An `i32` discriminant.
    I32(i32),
    /// An `i64` discriminant.
    I64(i64),
    /// An `i128` discriminant.
    I128(i128),
    /// An `isize` discriminant.
    Isize(isize),
    /// A `u8` discriminant.
    U8(u8),
    /// A `u16` discriminant.
    U16(u16),
    /// A `u32` discriminant.
    U32(u32),
    /// A `u64` discriminant.
    U64(u64),
    /// A `u128` discriminant.
    U128(u128),
    /// A `usize` discriminant.
    Usize(usize),
}
