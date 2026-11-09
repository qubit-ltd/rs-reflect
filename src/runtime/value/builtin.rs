// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::TypeRegistration;
use crate::runtime::descriptor::generic::GenericArgumentMetadata;
use crate::runtime::descriptor::type_kind::TypeKindMetadata;
use crate::runtime::expression::ExpressionData;

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
static TYPE_PARAMETER_EXPRESSION: ExpressionData = ExpressionData::ParameterName("T");
static KEY_PARAMETER_EXPRESSION: ExpressionData = ExpressionData::ParameterName("K");
static VALUE_PARAMETER_EXPRESSION: ExpressionData = ExpressionData::ParameterName("V");

/// Builds a shape-only registration for a built-in type with no generic args.
///
/// # Type Parameters
///
/// * `T` — concrete built-in type represented by the registration.
///
/// # Parameters
///
/// * `kind` — static shape category assigned to the type.
fn shaped_registration<T: ?Sized + 'static>(kind: &'static TypeKindMetadata) -> &'static TypeRegistration<T> {
    TypeRegistration::get_or_init_with(|| TypeRegistration::with_shape(kind, None, &[]))
}

/// Builds a map registration while retaining its key and value type arguments.
///
/// # Type Parameters
///
/// * `T` — concrete map type being registered.
/// * `K` — reflected key type stored by the map.
/// * `V` — reflected value type stored by the map.
fn map_registration<T: ?Sized + 'static, K: Reflect, V: Reflect>() -> &'static TypeRegistration<T> {
    TypeRegistration::get_or_init_with(|| {
        let arguments = Box::leak(
            vec![
                GenericArgumentMetadata::reflected_type_argument::<K>(&KEY_PARAMETER_EXPRESSION),
                GenericArgumentMetadata::reflected_type_argument::<V>(&VALUE_PARAMETER_EXPRESSION),
            ]
            .into_boxed_slice(),
        );
        TypeRegistration::with_shape(&MAP_KIND, None, &[]).with_generic_arguments(arguments)
    })
}

/// Builds a set registration while retaining its element type argument.
///
/// # Type Parameters
///
/// * `Collection` — concrete set type being registered.
/// * `T` — reflected element type stored by the set.
fn set_registration<Collection: ?Sized + 'static, T: Reflect>() -> &'static TypeRegistration<Collection> {
    TypeRegistration::get_or_init_with(|| {
        let arguments = Box::leak(
            vec![GenericArgumentMetadata::reflected_type_argument::<T>(
                &TYPE_PARAMETER_EXPRESSION,
            )]
            .into_boxed_slice(),
        );
        TypeRegistration::with_shape(&SET_KIND, None, &[]).with_generic_arguments(arguments)
    })
}

/// Builds a smart-pointer registration while retaining the pointee type.
///
/// # Type Parameters
///
/// * `Pointer` — concrete pointer wrapper being registered.
/// * `T` — reflected pointee type.
fn smart_pointer_registration<Pointer: ?Sized + 'static, T: Reflect + ?Sized>() -> &'static TypeRegistration<Pointer> {
    TypeRegistration::get_or_init_with(|| {
        let arguments = Box::leak(
            vec![GenericArgumentMetadata::reflected_type_argument::<T>(
                &TYPE_PARAMETER_EXPRESSION,
            )]
            .into_boxed_slice(),
        );
        TypeRegistration::with_shape(&SMART_POINTER_KIND, None, &[]).with_generic_arguments(arguments)
    })
}

macro_rules! impl_builtin_reflect {
    ($kind:ident; $($ty:ty),+ $(,)?) => {
        $(
            impl Reflect for $ty {
                /// Registers a scalar with the kind metadata selected by the macro.
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
    /// Registers strings with text shape metadata and no struct fields.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(|| TypeRegistration::with_shape(&TEXT_KIND, None, &[]))
    }
}

impl Reflect for () {
    /// Registers the unit type with an empty tuple shape.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(|| TypeRegistration::with_shape(&TUPLE_KIND, None, &[]))
    }
}

crate::__private::codegen_v4::registration::submit_type!(());

impl<T: Reflect> Reflect for Vec<T> {
    /// Registers the sequence shape and retains the element type argument.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(|| {
            let arguments = Box::leak(
                vec![GenericArgumentMetadata::reflected_type_argument::<T>(
                    &TYPE_PARAMETER_EXPRESSION,
                )]
                .into_boxed_slice(),
            );
            TypeRegistration::with_shape(&SEQUENCE_KIND, None, &[]).with_generic_arguments(arguments)
        })
    }
}

impl<T: Reflect> Reflect for Option<T> {
    /// Registers the optional shape and retains the wrapped type argument.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(|| {
            let arguments = Box::leak(
                vec![GenericArgumentMetadata::reflected_type_argument::<T>(
                    &TYPE_PARAMETER_EXPRESSION,
                )]
                .into_boxed_slice(),
            );
            TypeRegistration::with_shape(&OPTIONAL_KIND, None, &[]).with_generic_arguments(arguments)
        })
    }
}

impl<T: Reflect, const N: usize> Reflect for [T; N] {
    /// Registers the array shape and retains its element type argument.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(|| {
            let arguments = Box::leak(
                vec![GenericArgumentMetadata::reflected_type_argument::<T>(
                    &TYPE_PARAMETER_EXPRESSION,
                )]
                .into_boxed_slice(),
            );
            TypeRegistration::with_shape(&ARRAY_KIND, None, &[]).with_generic_arguments(arguments)
        })
    }
}

impl<T: Reflect, S: 'static> Reflect for std::collections::HashSet<T, S> {
    /// Registers an unordered set and retains its element type argument.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        set_registration::<Self, T>()
    }
}

impl<T: Reflect> Reflect for std::collections::BTreeSet<T> {
    /// Registers an ordered set and retains its element type argument.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        set_registration::<Self, T>()
    }
}

impl<K: Reflect, V: Reflect> Reflect for std::collections::BTreeMap<K, V> {
    /// Registers an ordered map and retains its key and value type arguments.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        map_registration::<Self, K, V>()
    }
}

impl<K: Reflect, V: Reflect, S: 'static> Reflect for std::collections::HashMap<K, V, S> {
    /// Registers a hash map and retains its key and value type arguments.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        map_registration::<Self, K, V>()
    }
}

macro_rules! impl_smart_pointer_reflect {
    ($module:ident, $pointer:ident) => {
        impl<T: Reflect + ?Sized> Reflect for std::$module::$pointer<T> {
            /// Registers the smart-pointer shape and its pointee argument.
            fn __reflect_registration() -> &'static TypeRegistration<Self> {
                smart_pointer_registration::<Self, T>()
            }
        }
    };
}

impl<T: Reflect + ?Sized> Reflect for Box<T> {
    /// Registers box indirection and retains the pointee type argument.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(|| {
            let arguments = Box::leak(
                vec![GenericArgumentMetadata::reflected_type_argument::<T>(
                    &TYPE_PARAMETER_EXPRESSION,
                )]
                .into_boxed_slice(),
            );
            TypeRegistration::with_shape(&SMART_POINTER_KIND, None, &[]).with_generic_arguments(arguments)
        })
    }
}

impl_smart_pointer_reflect!(rc, Rc);
impl_smart_pointer_reflect!(sync, Arc);

impl<T: Reflect + ?Sized> Reflect for &'static T {
    /// Registers a shared reference shape without claiming ownership of its
    /// target.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&REFERENCE_KIND)
    }
}

impl<T: Reflect + ?Sized> Reflect for &'static mut T {
    /// Registers an exclusive reference shape without owning its target.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&REFERENCE_KIND)
    }
}

impl<T: Reflect> Reflect for [T] {
    /// Registers a slice and retains its element type argument.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(|| {
            let arguments = Box::leak(
                vec![GenericArgumentMetadata::reflected_type_argument::<T>(
                    &TYPE_PARAMETER_EXPRESSION,
                )]
                .into_boxed_slice(),
            );
            TypeRegistration::with_shape(&SLICE_KIND, None, &[]).with_generic_arguments(arguments)
        })
    }
}

impl<T: Reflect + ?Sized> Reflect for *const T {
    /// Registers a const raw-pointer shape without dereferencing the pointer.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&RAW_POINTER_KIND)
    }
}

impl<T: Reflect + ?Sized> Reflect for *mut T {
    /// Registers a mutable raw-pointer shape without dereferencing the pointer.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&RAW_POINTER_KIND)
    }
}

impl Reflect for dyn std::fmt::Debug {
    /// Registers the trait-object shape; no object instance is inspected.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&TRAIT_OBJECT_KIND)
    }
}

impl Reflect for dyn std::any::Any {
    /// Registers the trait-object shape; no object instance is inspected.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&TRAIT_OBJECT_KIND)
    }
}

macro_rules! impl_function_pointer_reflect {
    (@with_attributes [$($attribute:meta),*]; $($argument:ident),* $(,)?) => {
        $(#[$attribute])*
        impl<$($argument: Reflect,)* Return: Reflect> Reflect for fn($($argument),*) -> Return {
            /// Registers this signature as a function pointer without evaluating it.
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
    (@hrtb_overlap; $($argument:ident),* $(,)?) => {
        impl_function_pointer_reflect!(@with_attributes [allow(coherence_leak_check)]; $($argument),*);
    };
    ($($argument:ident),* $(,)?) => {
        impl_function_pointer_reflect!(@with_attributes []; $($argument),*);
    };
}

impl_function_pointer_reflect!();
impl_function_pointer_reflect!(@hrtb_overlap; A);
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

impl<T: Reflect, Return: Reflect> Reflect for for<'a> fn(&'a T) -> Return {
    /// Registers the function-pointer shape while retaining its higher-ranked
    /// signature.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        shaped_registration(&FUNCTION_KIND)
    }
}
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
            /// Registers tuple shape and retains each component type argument.
            fn __reflect_registration() -> &'static TypeRegistration<Self> {
                TypeRegistration::get_or_init_with(|| {
                    let arguments = Box::leak(
                        vec![
                            $(GenericArgumentMetadata::reflected_type_argument::<$type>(
                                Box::leak(Box::new(ExpressionData::ParameterName(stringify!($type)))),
                            ),)+
                        ]
                        .into_boxed_slice(),
                    );
                    TypeRegistration::with_shape(&TUPLE_KIND, None, &[])
                        .with_generic_arguments(arguments)
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
mod tests;
