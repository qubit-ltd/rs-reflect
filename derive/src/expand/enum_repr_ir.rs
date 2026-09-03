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
    Rust,
    C,
    Transparent,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    Align(usize),
}

impl EnumReprIr {
    /// Returns the primitive integer spelling used for compiler-checked casts.
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
    pub(super) fn descriptor_tokens(&self, facade: &TokenStream) -> TokenStream {
        match self {
            Self::Rust => quote!(#facade::__private::codegen_v1::descriptor::EnumRepr::Rust),
            Self::C => quote!(#facade::__private::codegen_v1::descriptor::EnumRepr::C),
            Self::Transparent => {
                quote!(#facade::__private::codegen_v1::descriptor::EnumRepr::Transparent)
            }
            Self::I8 => quote!(#facade::__private::codegen_v1::descriptor::EnumRepr::I8),
            Self::I16 => quote!(#facade::__private::codegen_v1::descriptor::EnumRepr::I16),
            Self::I32 => quote!(#facade::__private::codegen_v1::descriptor::EnumRepr::I32),
            Self::I64 => quote!(#facade::__private::codegen_v1::descriptor::EnumRepr::I64),
            Self::I128 => quote!(#facade::__private::codegen_v1::descriptor::EnumRepr::I128),
            Self::Isize => quote!(#facade::__private::codegen_v1::descriptor::EnumRepr::Isize),
            Self::U8 => quote!(#facade::__private::codegen_v1::descriptor::EnumRepr::U8),
            Self::U16 => quote!(#facade::__private::codegen_v1::descriptor::EnumRepr::U16),
            Self::U32 => quote!(#facade::__private::codegen_v1::descriptor::EnumRepr::U32),
            Self::U64 => quote!(#facade::__private::codegen_v1::descriptor::EnumRepr::U64),
            Self::U128 => quote!(#facade::__private::codegen_v1::descriptor::EnumRepr::U128),
            Self::Usize => quote!(#facade::__private::codegen_v1::descriptor::EnumRepr::Usize),
            Self::Align(alignment) => {
                quote!(#facade::__private::codegen_v1::descriptor::EnumRepr::Align(#alignment))
            }
        }
    }
}
