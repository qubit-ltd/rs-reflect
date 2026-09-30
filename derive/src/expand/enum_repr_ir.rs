// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Normalized enum representation metadata used during expansion.

use proc_macro2::TokenStream;
use quote::quote;

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

    /// Emits the public structured representation value for descriptor data.
    ///
    /// # Parameters
    ///
    /// - `facade`: Runtime facade path referenced by the generated expression.
    ///
    /// # Returns
    ///
    /// Returns tokens for the corresponding runtime representation value.
    #[must_use]
    pub(super) fn descriptor_tokens(&self, facade: &TokenStream) -> TokenStream {
        match self {
            Self::Rust => {
                quote!(#facade::__private::codegen_v3::descriptor::EnumRepr::Rust)
            }
            Self::C => {
                quote!(#facade::__private::codegen_v3::descriptor::EnumRepr::C)
            }
            Self::Transparent => {
                quote!(#facade::__private::codegen_v3::descriptor::EnumRepr::Transparent)
            }
            Self::I8 => {
                quote!(#facade::__private::codegen_v3::descriptor::EnumRepr::I8)
            }
            Self::I16 => {
                quote!(#facade::__private::codegen_v3::descriptor::EnumRepr::I16)
            }
            Self::I32 => {
                quote!(#facade::__private::codegen_v3::descriptor::EnumRepr::I32)
            }
            Self::I64 => {
                quote!(#facade::__private::codegen_v3::descriptor::EnumRepr::I64)
            }
            Self::I128 => {
                quote!(#facade::__private::codegen_v3::descriptor::EnumRepr::I128)
            }
            Self::Isize => {
                quote!(#facade::__private::codegen_v3::descriptor::EnumRepr::Isize)
            }
            Self::U8 => {
                quote!(#facade::__private::codegen_v3::descriptor::EnumRepr::U8)
            }
            Self::U16 => {
                quote!(#facade::__private::codegen_v3::descriptor::EnumRepr::U16)
            }
            Self::U32 => {
                quote!(#facade::__private::codegen_v3::descriptor::EnumRepr::U32)
            }
            Self::U64 => {
                quote!(#facade::__private::codegen_v3::descriptor::EnumRepr::U64)
            }
            Self::U128 => {
                quote!(#facade::__private::codegen_v3::descriptor::EnumRepr::U128)
            }
            Self::Usize => {
                quote!(#facade::__private::codegen_v3::descriptor::EnumRepr::Usize)
            }
            Self::Align(alignment) => {
                quote!(#facade::__private::codegen_v3::descriptor::EnumRepr::Align(#alignment))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;

    use super::EnumReprIr;

    #[test]
    fn test_all_enum_representations_emit_matching_descriptor_values() {
        let representations = [
            (EnumReprIr::Rust, "Rust", None),
            (EnumReprIr::C, "C", None),
            (EnumReprIr::Transparent, "Transparent", None),
            (EnumReprIr::I8, "I8", Some("i8")),
            (EnumReprIr::I16, "I16", Some("i16")),
            (EnumReprIr::I32, "I32", Some("i32")),
            (EnumReprIr::I64, "I64", Some("i64")),
            (EnumReprIr::I128, "I128", Some("i128")),
            (EnumReprIr::Isize, "Isize", Some("isize")),
            (EnumReprIr::U8, "U8", Some("u8")),
            (EnumReprIr::U16, "U16", Some("u16")),
            (EnumReprIr::U32, "U32", Some("u32")),
            (EnumReprIr::U64, "U64", Some("u64")),
            (EnumReprIr::U128, "U128", Some("u128")),
            (EnumReprIr::Usize, "Usize", Some("usize")),
            (EnumReprIr::Align(16), "", None),
        ];

        for (representation, expected_variant, integer_name) in representations {
            let tokens: TokenStream = representation.descriptor_tokens(&quote!(::qubit_reflect));
            let expected = match representation {
                EnumReprIr::Align(alignment) => format!("EnumRepr :: Align ({alignment}usize)"),
                _ => format!("EnumRepr :: {expected_variant}"),
            };

            assert!(
                tokens.to_string().contains(&expected),
                "{representation:?}: {tokens}"
            );
            assert_eq!(
                representation.integer_name(),
                integer_name,
                "{representation:?}"
            );
        }
    }
}
