// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::TypeRegistration;
use crate::runtime::descriptor::type_kind::TypeKindMetadata;

static ARRAY_KIND: TypeKindMetadata = TypeKindMetadata::Array;
static OPTIONAL_KIND: TypeKindMetadata = TypeKindMetadata::Optional;
static PRIMITIVE_KIND: TypeKindMetadata = TypeKindMetadata::Primitive;
static SET_KIND: TypeKindMetadata = TypeKindMetadata::Set;
static SEQUENCE_KIND: TypeKindMetadata = TypeKindMetadata::Sequence;
static MAP_KIND: TypeKindMetadata = TypeKindMetadata::Map;
static SMART_POINTER_KIND: TypeKindMetadata = TypeKindMetadata::SmartPointer;
static TEXT_KIND: TypeKindMetadata = TypeKindMetadata::Text;
static TUPLE_KIND: TypeKindMetadata = TypeKindMetadata::Tuple(&[]);
static REFERENCE_KIND: TypeKindMetadata = TypeKindMetadata::Reference;
static SLICE_KIND: TypeKindMetadata = TypeKindMetadata::Slice;
static RAW_POINTER_KIND: TypeKindMetadata = TypeKindMetadata::RawPointer;
static FUNCTION_KIND: TypeKindMetadata = TypeKindMetadata::Function;
static TRAIT_OBJECT_KIND: TypeKindMetadata = TypeKindMetadata::TraitObject;

fn shaped_registration<T: ?Sized + 'static>(kind: &'static TypeKindMetadata) -> &'static TypeRegistration<T> {
    TypeRegistration::get_or_init_with(|| TypeRegistration::with_shape(kind, None, &[]))
}

macro_rules! impl_builtin_reflect {
    ($kind:ident; $($ty:ty),+ $(,)?) => {
        $(
            impl Reflect for $ty {
                fn __reflect_registration() -> &'static TypeRegistration<Self> {
                    TypeRegistration::get_or_init_with(|| {
                        TypeRegistration::with_shape(&$kind, None, &[])
                    })
                }
            }
            crate::__private::codegen_v4::registration::submit_type!($ty);
        )+
    };
}

impl_builtin_reflect!(PRIMITIVE_KIND;
    bool, char, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64,
);
impl_builtin_reflect!(TEXT_KIND; String);

impl Reflect for str {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(|| TypeRegistration::with_shape(&TEXT_KIND, None, &[]))
    }
}

impl Reflect for () {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(|| TypeRegistration::with_shape(&TUPLE_KIND, None, &[]))
    }
}

crate::__private::codegen_v4::registration::submit_type!(());

impl<T: Reflect> Reflect for Vec<T> {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(|| TypeRegistration::with_shape(&SEQUENCE_KIND, None, &[]))
    }
}

impl<T: Reflect> Reflect for Option<T> {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(|| TypeRegistration::with_shape(&OPTIONAL_KIND, None, &[]))
    }
}

impl<T: Reflect, const N: usize> Reflect for [T; N] {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(|| TypeRegistration::with_shape(&ARRAY_KIND, None, &[]))
    }
}

impl<T: Reflect, S: 'static> Reflect for std::collections::HashSet<T, S> {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&SET_KIND)
    }
}

impl<T: Reflect> Reflect for std::collections::BTreeSet<T> {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&SET_KIND)
    }
}

impl<K: Reflect, V: Reflect> Reflect for std::collections::BTreeMap<K, V> {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&MAP_KIND)
    }
}

impl<K: Reflect, V: Reflect, S: 'static> Reflect for std::collections::HashMap<K, V, S> {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&MAP_KIND)
    }
}

macro_rules! impl_smart_pointer_reflect {
    ($module:ident, $pointer:ident) => {
        impl<T: Reflect + ?Sized> Reflect for std::$module::$pointer<T> {
            fn __reflect_registration() -> &'static TypeRegistration<Self> {
                shaped_registration(&SMART_POINTER_KIND)
            }
        }
    };
}

impl_smart_pointer_reflect!(boxed, Box);
impl_smart_pointer_reflect!(rc, Rc);
impl_smart_pointer_reflect!(sync, Arc);

impl<T: Reflect + ?Sized> Reflect for &'static T {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&REFERENCE_KIND)
    }
}

impl<T: Reflect + ?Sized> Reflect for &'static mut T {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&REFERENCE_KIND)
    }
}

impl<T: Reflect> Reflect for [T] {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&SLICE_KIND)
    }
}

impl<T: Reflect + ?Sized> Reflect for *const T {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&RAW_POINTER_KIND)
    }
}

impl<T: Reflect + ?Sized> Reflect for *mut T {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&RAW_POINTER_KIND)
    }
}

impl Reflect for dyn std::fmt::Debug {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&TRAIT_OBJECT_KIND)
    }
}

impl Reflect for dyn std::any::Any {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&TRAIT_OBJECT_KIND)
    }
}

macro_rules! impl_function_pointer_reflect {
    ($($argument:ident),* $(,)?) => {
        impl<$($argument: Reflect,)* Return: Reflect> Reflect for fn($($argument),*) -> Return {
            fn __reflect_registration() -> &'static TypeRegistration<Self> {
                shaped_registration(&FUNCTION_KIND)
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for unsafe fn($($argument),*) -> Return {
            fn __reflect_registration() -> &'static TypeRegistration<Self> {
                shaped_registration(&FUNCTION_KIND)
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for extern "C" fn($($argument),*) -> Return {
            fn __reflect_registration() -> &'static TypeRegistration<Self> {
                shaped_registration(&FUNCTION_KIND)
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for unsafe extern "C" fn($($argument),*) -> Return {
            fn __reflect_registration() -> &'static TypeRegistration<Self> {
                shaped_registration(&FUNCTION_KIND)
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for extern "C-unwind" fn($($argument),*) -> Return {
            fn __reflect_registration() -> &'static TypeRegistration<Self> {
                shaped_registration(&FUNCTION_KIND)
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for unsafe extern "C-unwind" fn($($argument),*) -> Return {
            fn __reflect_registration() -> &'static TypeRegistration<Self> {
                shaped_registration(&FUNCTION_KIND)
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for extern "system" fn($($argument),*) -> Return {
            fn __reflect_registration() -> &'static TypeRegistration<Self> {
                shaped_registration(&FUNCTION_KIND)
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for unsafe extern "system" fn($($argument),*) -> Return {
            fn __reflect_registration() -> &'static TypeRegistration<Self> {
                shaped_registration(&FUNCTION_KIND)
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for extern "system-unwind" fn($($argument),*) -> Return {
            fn __reflect_registration() -> &'static TypeRegistration<Self> {
                shaped_registration(&FUNCTION_KIND)
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for unsafe extern "system-unwind" fn($($argument),*) -> Return {
            fn __reflect_registration() -> &'static TypeRegistration<Self> {
                shaped_registration(&FUNCTION_KIND)
            }
        }
    };
}

impl_function_pointer_reflect!();
impl_function_pointer_reflect!(A);
impl_function_pointer_reflect!(A, B);
impl_function_pointer_reflect!(A, B, C);
impl_function_pointer_reflect!(A, B, C, D);
impl_function_pointer_reflect!(A, B, C, D, E);
impl_function_pointer_reflect!(A, B, C, D, E, F);
impl_function_pointer_reflect!(A, B, C, D, E, F, G);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H, I);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H, I, J);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H, I, J, K);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W);
impl_function_pointer_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X);
impl_function_pointer_reflect!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y
);
impl_function_pointer_reflect!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z
);
impl_function_pointer_reflect!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA
);
impl_function_pointer_reflect!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB
);
impl_function_pointer_reflect!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB, AC
);
impl_function_pointer_reflect!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB, AC, AD
);
impl_function_pointer_reflect!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB, AC, AD, AE
);
impl_function_pointer_reflect!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB, AC, AD, AE, AF
);

macro_rules! impl_tuple_reflect {
    ($($type:ident),+ $(,)?) => {
        impl<$($type: Reflect),+> Reflect for ($($type,)+) {
            fn __reflect_registration() -> &'static TypeRegistration<Self> {
                TypeRegistration::get_or_init_with(|| {
                    TypeRegistration::with_shape(&TUPLE_KIND, None, &[])
                })
            }
        }
    };
}

impl_tuple_reflect!(A);
impl_tuple_reflect!(A, B);
impl_tuple_reflect!(A, B, C);
impl_tuple_reflect!(A, B, C, D);
impl_tuple_reflect!(A, B, C, D, E);
impl_tuple_reflect!(A, B, C, D, E, F);
impl_tuple_reflect!(A, B, C, D, E, F, G);
impl_tuple_reflect!(A, B, C, D, E, F, G, H);
impl_tuple_reflect!(A, B, C, D, E, F, G, H, I);
impl_tuple_reflect!(A, B, C, D, E, F, G, H, I, J);
impl_tuple_reflect!(A, B, C, D, E, F, G, H, I, J, K);
impl_tuple_reflect!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_tuple_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_tuple_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_tuple_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_tuple_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
impl_tuple_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
impl_tuple_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
impl_tuple_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
impl_tuple_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
impl_tuple_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U);
impl_tuple_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V);
impl_tuple_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W);
impl_tuple_reflect!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X);
impl_tuple_reflect!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y
);
impl_tuple_reflect!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z
);
impl_tuple_reflect!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA
);
impl_tuple_reflect!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB
);
impl_tuple_reflect!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB, AC
);
impl_tuple_reflect!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB, AC, AD
);
impl_tuple_reflect!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB, AC, AD, AE
);
impl_tuple_reflect!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB, AC, AD, AE, AF
);

#[cfg(test)]
#[path = "builtin_tests.rs"]
mod tests;
