// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// One normalized component of an enum's explicit `repr(...)` declarations.
///
/// Values are structural metadata rather than diagnostic strings. The enum
/// view exposes components in a stable canonical order, independent of their
/// source order.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::EnumRepr;
/// let repr = EnumRepr::C;
/// assert_eq!(repr, EnumRepr::C);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EnumRepr {
    /// Rust's native representation was requested explicitly.
    Rust,
    /// The C-compatible representation was requested.
    C,
    /// The transparent representation was requested.
    Transparent,
    /// An `i8` discriminant representation.
    I8,
    /// An `i16` discriminant representation.
    I16,
    /// An `i32` discriminant representation.
    I32,
    /// An `i64` discriminant representation.
    I64,
    /// An `i128` discriminant representation.
    I128,
    /// An `isize` discriminant representation.
    Isize,
    /// A `u8` discriminant representation.
    U8,
    /// A `u16` discriminant representation.
    U16,
    /// A `u32` discriminant representation.
    U32,
    /// A `u64` discriminant representation.
    U64,
    /// A `u128` discriminant representation.
    U128,
    /// A `usize` discriminant representation.
    Usize,
    /// An explicit minimum alignment in bytes.
    Align(usize),
}
