// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Normalized enum representation metadata used during expansion.

/// A normalized enum representation component retained by generated metadata.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum EnumReprIr {
    /// Rust's default representation.
    Rust,
    /// C-compatible representation.
    C,
    /// Single-field transparent representation.
    Transparent,
    /// Signed 8-bit discriminant representation.
    I8,
    /// Signed 16-bit discriminant representation.
    I16,
    /// Signed 32-bit discriminant representation.
    I32,
    /// Signed 64-bit discriminant representation.
    I64,
    /// Signed 128-bit discriminant representation.
    I128,
    /// Pointer-sized signed discriminant representation.
    Isize,
    /// Unsigned 8-bit discriminant representation.
    U8,
    /// Unsigned 16-bit discriminant representation.
    U16,
    /// Unsigned 32-bit discriminant representation.
    U32,
    /// Unsigned 64-bit discriminant representation.
    U64,
    /// Unsigned 128-bit discriminant representation.
    U128,
    /// Pointer-sized unsigned discriminant representation.
    Usize,
    /// Explicit minimum alignment in bytes.
    ///
    /// The payload is the requested alignment.
    Align(usize),
}

impl EnumReprIr {
    /// Returns the primitive integer spelling used for compiler-checked casts.
    ///
    /// # Returns
    ///
    /// Returns the integer type spelling, or `None` for non-integer
    /// representations.
    #[must_use]
    #[inline]
    pub(super) fn integer_name(&self) -> Option<&'static str> {
        match self {
            Self::I8 => Some("i8"),
            Self::I16 => Some("i16"),
            Self::I32 => Some("i32"),
            Self::I64 => Some("i64"),
            Self::I128 => Some("i128"),
            Self::Isize => Some("isize"),
            Self::U8 => Some("u8"),
            Self::U16 => Some("u16"),
            Self::U32 => Some("u32"),
            Self::U64 => Some("u64"),
            Self::U128 => Some("u128"),
            Self::Usize => Some("usize"),
            Self::Rust | Self::C | Self::Transparent | Self::Align(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EnumReprIr;

    #[test]
    fn test_enum_representation_integer_names() {
        let representations = [
            (EnumReprIr::Rust, None),
            (EnumReprIr::C, None),
            (EnumReprIr::Transparent, None),
            (EnumReprIr::I8, Some("i8")),
            (EnumReprIr::I16, Some("i16")),
            (EnumReprIr::I32, Some("i32")),
            (EnumReprIr::I64, Some("i64")),
            (EnumReprIr::I128, Some("i128")),
            (EnumReprIr::Isize, Some("isize")),
            (EnumReprIr::U8, Some("u8")),
            (EnumReprIr::U16, Some("u16")),
            (EnumReprIr::U32, Some("u32")),
            (EnumReprIr::U64, Some("u64")),
            (EnumReprIr::U128, Some("u128")),
            (EnumReprIr::Usize, Some("usize")),
            (EnumReprIr::Align(16), None),
        ];

        for (representation, integer_name) in representations {
            assert_eq!(representation.integer_name(), integer_name, "{representation:?}");
        }
    }
}
