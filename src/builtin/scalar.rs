// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Built-in reflected scalar and text descriptors.

use std::sync::OnceLock;

use crate::capability::TypeCapabilities;
use crate::capability::clone_descriptor;
use crate::capability::default_descriptor;
use crate::capability::send_descriptor;
use crate::capability::sync_descriptor;
use crate::descriptor::PrimitiveKind;
use crate::descriptor::Reflect;
use crate::descriptor::TextKind;
use crate::descriptor::TypeDescriptor;

macro_rules! reflected_primitive {
    ($type:ty, $kind:expr, $descriptor:ident, $capabilities:ident) => {
        fn $capabilities() -> &'static TypeCapabilities {
            static CAPABILITIES: OnceLock<TypeCapabilities> = OnceLock::new();
            CAPABILITIES.get_or_init(|| {
                TypeCapabilities::try_new(vec![
                    send_descriptor::<$type>(),
                    sync_descriptor::<$type>(),
                    clone_descriptor::<$type>(),
                    default_descriptor::<$type>(),
                ])
                .expect("built-in core capability IDs must be distinct")
            })
        }

        static $descriptor: TypeDescriptor =
            crate::__private::descriptor::primitive_with_capabilities::<$type>(
                stringify!($type),
                $kind,
                $capabilities,
            );

        impl Reflect for $type {
            /// Returns this built-in type's unique static descriptor.
            fn type_descriptor() -> &'static TypeDescriptor {
                &$descriptor
            }
        }
    };
}

reflected_primitive!(
    bool,
    PrimitiveKind::Bool,
    BOOL_DESCRIPTOR,
    bool_capabilities
);
reflected_primitive!(
    char,
    PrimitiveKind::Char,
    CHAR_DESCRIPTOR,
    char_capabilities
);
reflected_primitive!(i8, PrimitiveKind::I8, I8_DESCRIPTOR, i8_capabilities);
reflected_primitive!(i16, PrimitiveKind::I16, I16_DESCRIPTOR, i16_capabilities);
reflected_primitive!(i32, PrimitiveKind::I32, I32_DESCRIPTOR, i32_capabilities);
reflected_primitive!(i64, PrimitiveKind::I64, I64_DESCRIPTOR, i64_capabilities);
reflected_primitive!(
    i128,
    PrimitiveKind::I128,
    I128_DESCRIPTOR,
    i128_capabilities
);
reflected_primitive!(
    isize,
    PrimitiveKind::Isize,
    ISIZE_DESCRIPTOR,
    isize_capabilities
);
reflected_primitive!(u8, PrimitiveKind::U8, U8_DESCRIPTOR, u8_capabilities);
reflected_primitive!(u16, PrimitiveKind::U16, U16_DESCRIPTOR, u16_capabilities);
reflected_primitive!(u32, PrimitiveKind::U32, U32_DESCRIPTOR, u32_capabilities);
reflected_primitive!(u64, PrimitiveKind::U64, U64_DESCRIPTOR, u64_capabilities);
reflected_primitive!(
    u128,
    PrimitiveKind::U128,
    U128_DESCRIPTOR,
    u128_capabilities
);
reflected_primitive!(
    usize,
    PrimitiveKind::Usize,
    USIZE_DESCRIPTOR,
    usize_capabilities
);
reflected_primitive!(f32, PrimitiveKind::F32, F32_DESCRIPTOR, f32_capabilities);
reflected_primitive!(f64, PrimitiveKind::F64, F64_DESCRIPTOR, f64_capabilities);

/// Returns the core capabilities registered for the concrete `String` type.
fn string_capabilities() -> &'static TypeCapabilities {
    static CAPABILITIES: OnceLock<TypeCapabilities> = OnceLock::new();
    CAPABILITIES.get_or_init(|| {
        TypeCapabilities::try_new(vec![
            send_descriptor::<String>(),
            sync_descriptor::<String>(),
            clone_descriptor::<String>(),
            default_descriptor::<String>(),
        ])
        .expect("built-in core capability IDs must be distinct")
    })
}

static STRING_DESCRIPTOR: TypeDescriptor =
    crate::__private::descriptor::text_with_capabilities::<String>(
        "String",
        TextKind::String,
        string_capabilities,
    );

impl Reflect for String {
    /// Returns `String`'s unique static descriptor.
    fn type_descriptor() -> &'static TypeDescriptor {
        &STRING_DESCRIPTOR
    }
}

static STR_DESCRIPTOR: TypeDescriptor =
    crate::__private::descriptor::text::<str>("str", TextKind::Str);

impl Reflect for str {
    /// Returns `str`'s unique static descriptor.
    fn type_descriptor() -> &'static TypeDescriptor {
        &STR_DESCRIPTOR
    }
}
