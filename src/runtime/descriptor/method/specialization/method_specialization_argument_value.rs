// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// One concrete generic argument used to select a method specialization.
#[derive(Clone, Copy)]
enum MethodSpecializationArgumentValue {
    /// Concrete Rust type argument recorded by its static type metadata.
    Type(
        /// Function that resolves the type metadata without runtime allocation.
        fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata,
    ),
    /// Lifetime argument represented by its source spelling.
    Lifetime(
        /// Lifetime name, including the leading apostrophe.
        &'static str,
    ),
    /// Concrete signed or unsigned constant used in a specialization selector.
    /// Returns the declared return-value category.
    /// Signed integer constant argument.
    ConstSigned {
        /// Integer type used to interpret the constant.
        kind: ConstIntegerKind,
        /// Signed value supplied as the generic argument.
        value: i128,
    },
    /// Concrete signed or unsigned constant used in a specialization selector.
    /// Returns the declared return-value category.
    /// Unsigned integer constant argument.
    ConstUnsigned {
        /// Integer type used to interpret the constant.
        kind: ConstIntegerKind,
        /// Unsigned value supplied as the generic argument.
        value: u128,
    },
    /// Unsigned magnitude retained for an integer literal selector.
    /// Whether an integer literal selector has a leading minus sign.
    /// Integer literal selector before its inferred integer type is applied.
    ConstIntegerLiteral {
        /// Whether the source literal has a leading minus sign.
        negative: bool,
        /// Absolute value of the literal, stored without signed overflow.
        magnitude: u128,
    },
    /// Boolean constant argument.
    ConstBoolean(
        /// Boolean value supplied by the specialization.
        bool,
    ),
    /// Character constant argument.
    ConstCharacter(
        /// Unicode scalar value supplied by the specialization.
        char,
    ),
}
