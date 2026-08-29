//! Reflection descriptors for portable function-pointer signatures.

use crate::builtin::interner;
use crate::descriptor::FunctionPointerKind;
use crate::descriptor::Reflect;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeRef;
use crate::expression::FunctionAbi;

/// Creates a function-pointer descriptor from fully resolved signature parts.
fn descriptor<T: ?Sized + 'static>(
    kind: FunctionPointerKind,
    abi: FunctionAbi,
    parameters: Vec<TypeRef>,
    return_type: TypeRef,
) -> TypeDescriptor {
    let abi = Box::leak(Box::new(abi));
    let parameters = Box::leak(parameters.into_boxed_slice());
    let return_type = Box::leak(Box::new(return_type));
    TypeDescriptor::new_function::<T>(
        std::any::type_name::<T>(),
                kind,
        abi,
        false,
        parameters,
        return_type,
    )
}

/// Creates a C-variadic function-pointer descriptor from resolved signature
/// parts.
fn variadic_descriptor<T: ?Sized + 'static>(
    kind: FunctionPointerKind,
    parameters: Vec<TypeRef>,
    return_type: TypeRef,
) -> TypeDescriptor {
    let abi = Box::leak(Box::new(FunctionAbi::C));
    let parameters = Box::leak(parameters.into_boxed_slice());
    let return_type = Box::leak(Box::new(return_type));
    TypeDescriptor::new_function::<T>(
        std::any::type_name::<T>(),
                kind,
        abi,
        true,
        parameters,
        return_type,
    )
}

macro_rules! impl_function_pointers {
    ($($argument:ident),*) => {
        impl<$($argument: Reflect,)* Return: Reflect> Reflect for fn($($argument),*) -> Return {
            /// Returns the interned descriptor for this safe Rust function pointer.
            fn type_descriptor() -> &'static TypeDescriptor {
                interner::intern::<Self>(|| descriptor::<Self>(
                    FunctionPointerKind::Safe,
                    FunctionAbi::Rust,
                    vec![$(TypeRef::Resolved($argument::type_descriptor())),*],
                    TypeRef::Resolved(Return::type_descriptor()),
                ))
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for unsafe fn($($argument),*) -> Return {
            /// Returns the interned descriptor for this unsafe Rust function pointer.
            fn type_descriptor() -> &'static TypeDescriptor {
                interner::intern::<Self>(|| descriptor::<Self>(
                    FunctionPointerKind::Unsafe,
                    FunctionAbi::Rust,
                    vec![$(TypeRef::Resolved($argument::type_descriptor())),*],
                    TypeRef::Resolved(Return::type_descriptor()),
                ))
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for extern "C" fn($($argument),*) -> Return {
            /// Returns the interned descriptor for this safe C function pointer.
            fn type_descriptor() -> &'static TypeDescriptor {
                interner::intern::<Self>(|| descriptor::<Self>(
                    FunctionPointerKind::Safe,
                    FunctionAbi::C,
                    vec![$(TypeRef::Resolved($argument::type_descriptor())),*],
                    TypeRef::Resolved(Return::type_descriptor()),
                ))
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for unsafe extern "C" fn($($argument),*) -> Return {
            /// Returns the interned descriptor for this unsafe C function pointer.
            fn type_descriptor() -> &'static TypeDescriptor {
                interner::intern::<Self>(|| descriptor::<Self>(
                    FunctionPointerKind::Unsafe,
                    FunctionAbi::C,
                    vec![$(TypeRef::Resolved($argument::type_descriptor())),*],
                    TypeRef::Resolved(Return::type_descriptor()),
                ))
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for extern "C-unwind" fn($($argument),*) -> Return {
            /// Returns the interned descriptor for this safe C-unwind function pointer.
            fn type_descriptor() -> &'static TypeDescriptor {
                interner::intern::<Self>(|| descriptor::<Self>(
                    FunctionPointerKind::Safe,
                    FunctionAbi::Other("C-unwind".into()),
                    vec![$(TypeRef::Resolved($argument::type_descriptor())),*],
                    TypeRef::Resolved(Return::type_descriptor()),
                ))
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for unsafe extern "C-unwind" fn($($argument),*) -> Return {
            /// Returns the interned descriptor for this unsafe C-unwind function pointer.
            fn type_descriptor() -> &'static TypeDescriptor {
                interner::intern::<Self>(|| descriptor::<Self>(
                    FunctionPointerKind::Unsafe,
                    FunctionAbi::Other("C-unwind".into()),
                    vec![$(TypeRef::Resolved($argument::type_descriptor())),*],
                    TypeRef::Resolved(Return::type_descriptor()),
                ))
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for extern "system" fn($($argument),*) -> Return {
            /// Returns the interned descriptor for this safe system function pointer.
            fn type_descriptor() -> &'static TypeDescriptor {
                interner::intern::<Self>(|| descriptor::<Self>(
                    FunctionPointerKind::Safe,
                    FunctionAbi::System,
                    vec![$(TypeRef::Resolved($argument::type_descriptor())),*],
                    TypeRef::Resolved(Return::type_descriptor()),
                ))
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for unsafe extern "system" fn($($argument),*) -> Return {
            /// Returns the interned descriptor for this unsafe system function pointer.
            fn type_descriptor() -> &'static TypeDescriptor {
                interner::intern::<Self>(|| descriptor::<Self>(
                    FunctionPointerKind::Unsafe,
                    FunctionAbi::System,
                    vec![$(TypeRef::Resolved($argument::type_descriptor())),*],
                    TypeRef::Resolved(Return::type_descriptor()),
                ))
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for extern "system-unwind" fn($($argument),*) -> Return {
            /// Returns the interned descriptor for this safe system-unwind function pointer.
            fn type_descriptor() -> &'static TypeDescriptor {
                interner::intern::<Self>(|| descriptor::<Self>(
                    FunctionPointerKind::Safe,
                    FunctionAbi::Other("system-unwind".into()),
                    vec![$(TypeRef::Resolved($argument::type_descriptor())),*],
                    TypeRef::Resolved(Return::type_descriptor()),
                ))
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for unsafe extern "system-unwind" fn($($argument),*) -> Return {
            /// Returns the interned descriptor for this unsafe system-unwind function pointer.
            fn type_descriptor() -> &'static TypeDescriptor {
                interner::intern::<Self>(|| descriptor::<Self>(
                    FunctionPointerKind::Unsafe,
                    FunctionAbi::Other("system-unwind".into()),
                    vec![$(TypeRef::Resolved($argument::type_descriptor())),*],
                    TypeRef::Resolved(Return::type_descriptor()),
                ))
            }
        }
    };
}

impl_function_pointers!();
impl_function_pointers!(A);
impl_function_pointers!(A, B);
impl_function_pointers!(A, B, C);
impl_function_pointers!(A, B, C, D);
impl_function_pointers!(A, B, C, D, E);
impl_function_pointers!(A, B, C, D, E, F);
impl_function_pointers!(A, B, C, D, E, F, G);
impl_function_pointers!(A, B, C, D, E, F, G, H);
impl_function_pointers!(A, B, C, D, E, F, G, H, I);
impl_function_pointers!(A, B, C, D, E, F, G, H, I, J);
impl_function_pointers!(A, B, C, D, E, F, G, H, I, J, K);
impl_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
impl_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
impl_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
impl_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
impl_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
impl_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U
);
impl_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V
);
impl_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W
);
impl_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X
);
impl_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y
);
impl_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z
);
impl_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA
);
impl_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB
);
impl_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB, AC
);
impl_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB, AC, AD
);
impl_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB, AC, AD,
    AE
);
impl_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB, AC, AD,
    AE, AF
);

macro_rules! impl_c_variadic_function_pointers {
    ($($argument:ident),*) => {
        impl<$($argument: Reflect,)* Return: Reflect> Reflect for extern "C" fn($($argument,)* ...) -> Return {
            /// Returns the interned descriptor for this safe C-variadic function pointer.
            fn type_descriptor() -> &'static TypeDescriptor {
                interner::intern::<Self>(|| variadic_descriptor::<Self>(
                    FunctionPointerKind::Safe,
                    vec![$(TypeRef::Resolved($argument::type_descriptor())),*],
                    TypeRef::Resolved(Return::type_descriptor()),
                ))
            }
        }

        impl<$($argument: Reflect,)* Return: Reflect> Reflect for unsafe extern "C" fn($($argument,)* ...) -> Return {
            /// Returns the interned descriptor for this unsafe C-variadic function pointer.
            fn type_descriptor() -> &'static TypeDescriptor {
                interner::intern::<Self>(|| variadic_descriptor::<Self>(
                    FunctionPointerKind::Unsafe,
                    vec![$(TypeRef::Resolved($argument::type_descriptor())),*],
                    TypeRef::Resolved(Return::type_descriptor()),
                ))
            }
        }
    };
}

impl_c_variadic_function_pointers!();
impl_c_variadic_function_pointers!(A);
impl_c_variadic_function_pointers!(A, B);
impl_c_variadic_function_pointers!(A, B, C);
impl_c_variadic_function_pointers!(A, B, C, D);
impl_c_variadic_function_pointers!(A, B, C, D, E);
impl_c_variadic_function_pointers!(A, B, C, D, E, F);
impl_c_variadic_function_pointers!(A, B, C, D, E, F, G);
impl_c_variadic_function_pointers!(A, B, C, D, E, F, G, H);
impl_c_variadic_function_pointers!(A, B, C, D, E, F, G, H, I);
impl_c_variadic_function_pointers!(A, B, C, D, E, F, G, H, I, J);
impl_c_variadic_function_pointers!(A, B, C, D, E, F, G, H, I, J, K);
impl_c_variadic_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_c_variadic_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_c_variadic_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_c_variadic_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_c_variadic_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
impl_c_variadic_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
impl_c_variadic_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
impl_c_variadic_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
impl_c_variadic_function_pointers!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
impl_c_variadic_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U
);
impl_c_variadic_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V
);
impl_c_variadic_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W
);
impl_c_variadic_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X
);
impl_c_variadic_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y
);
impl_c_variadic_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z
);
impl_c_variadic_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA
);
impl_c_variadic_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB
);
impl_c_variadic_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB, AC
);
impl_c_variadic_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB, AC, AD
);
impl_c_variadic_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB, AC, AD,
    AE
);
impl_c_variadic_function_pointers!(
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, AA, AB, AC, AD,
    AE, AF
);
