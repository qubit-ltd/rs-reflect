// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Provides the constintegerkind operation used to resolve or dispatch
/// reflected methods.
#[derive(Clone, Copy)]
pub(super) enum ConstIntegerKind {
    /// The selector expects an `i8` constant.
    I8,
    /// The selector expects an `i16` constant.
    I16,
    /// The selector expects an `i32` constant.
    I32,
    /// The selector expects an `i64` constant.
    I64,
    /// The selector expects an `i128` constant.
    I128,
    /// Provides the isize operation used to resolve or dispatch reflected
    /// methods.
    Isize,
    /// The selector expects a `u8` constant.
    U8,
    /// The selector expects a `u16` constant.
    U16,
    /// The selector expects a `u32` constant.
    U32,
    /// The selector expects a `u64` constant.
    U64,
    /// The selector expects a `u128` constant.
    U128,
    /// Provides the usize operation used to resolve or dispatch reflected
    /// methods.
    Usize,
}

impl MethodSpecializationArgument {
    /// Records a concrete type argument without exposing runtime identity data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    pub const fn type_argument<T: 'static>() -> Self {
        Self {
            value: MethodSpecializationArgumentValue::Type(crate::runtime::descriptor::metadata::TypeMetadata::of::<T>),
        }
    }

    /// Records a concrete lifetime argument for exact specialization selection.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    pub const fn lifetime(name: &'static str) -> Self {
        Self {
            value: MethodSpecializationArgumentValue::Lifetime(name),
        }
    }

    /// Records a boolean const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn const_bool(value: bool) -> Self {
        Self {
            value: MethodSpecializationArgumentValue::ConstBoolean(value),
        }
    }

    /// Records a character const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn const_char(value: char) -> Self {
        Self {
            value: MethodSpecializationArgumentValue::ConstCharacter(value),
        }
    }

    /// Records an unsuffixed integer literal for validation against its
    /// declaration.
    ///
    /// # Parameters
    ///
    /// - `negative`: Whether the integer literal has a leading minus sign.
    /// - `magnitude`: Absolute integer literal magnitude before applying its
    ///   sign.
    pub const fn const_integer_literal(negative: bool, magnitude: u128) -> Self {
        Self {
            value: MethodSpecializationArgumentValue::ConstIntegerLiteral { negative, magnitude },
        }
    }

    /// Records a signed integer const argument.
    ///
    /// # Parameters
    ///
    /// - `kind`: Receiver form, return category, or constant type being
    ///   recorded.
    /// - `value`: Owned value retained for validation and construction.
    const fn const_signed(kind: ConstIntegerKind, value: i128) -> Self {
        Self {
            value: MethodSpecializationArgumentValue::ConstSigned { kind, value },
        }
    }

    /// Records an unsigned integer const argument.
    ///
    /// # Parameters
    ///
    /// - `kind`: Receiver form, return category, or constant type being
    ///   recorded.
    /// - `value`: Owned value retained for validation and construction.
    const fn const_unsigned(kind: ConstIntegerKind, value: u128) -> Self {
        Self {
            value: MethodSpecializationArgumentValue::ConstUnsigned { kind, value },
        }
    }

    /// Records an `i8` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn i8(value: i8) -> Self {
        Self::const_signed(ConstIntegerKind::I8, value as i128)
    }

    /// Records an `i16` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn i16(value: i16) -> Self {
        Self::const_signed(ConstIntegerKind::I16, value as i128)
    }

    /// Records an `i32` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn i32(value: i32) -> Self {
        Self::const_signed(ConstIntegerKind::I32, value as i128)
    }

    /// Records an `i64` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn i64(value: i64) -> Self {
        Self::const_signed(ConstIntegerKind::I64, value as i128)
    }

    /// Records an `i128` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn i128(value: i128) -> Self {
        Self::const_signed(ConstIntegerKind::I128, value)
    }

    /// Records an `isize` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn isize(value: isize) -> Self {
        Self::const_signed(ConstIntegerKind::Isize, value as i128)
    }

    /// Records a `u8` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn u8(value: u8) -> Self {
        Self::const_unsigned(ConstIntegerKind::U8, value as u128)
    }

    /// Records a `u16` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn u16(value: u16) -> Self {
        Self::const_unsigned(ConstIntegerKind::U16, value as u128)
    }

    /// Records a `u32` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn u32(value: u32) -> Self {
        Self::const_unsigned(ConstIntegerKind::U32, value as u128)
    }

    /// Records a `u64` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn u64(value: u64) -> Self {
        Self::const_unsigned(ConstIntegerKind::U64, value as u128)
    }

    /// Records a `u128` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn u128(value: u128) -> Self {
        Self::const_unsigned(ConstIntegerKind::U128, value)
    }

    /// Records a `usize` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn usize(value: usize) -> Self {
        Self::const_unsigned(ConstIntegerKind::Usize, value as u128)
    }
}
