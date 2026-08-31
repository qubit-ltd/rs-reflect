use core::fmt::Debug;

use qubit_reflect::reflect;

type Bytes = [u8];

#[reflect]
trait AliasedUnsizedConstContract {
    const BYTES: Bytes;
}

#[reflect]
trait MaybeUnsizedConstContract<T: ?Sized> {
    const VALUE: T;
}

#[reflect]
trait LifetimeProjectionConstContract<'a> {
    type Value;
    const VALUE: Self::Value;
}

#[reflect]
trait BorrowedConstContract<'a> {
    const FUNCTION: fn(&'a u8);
    const POINTER: *const &'a u8;
    const REFERENCE: &'a u8;
    const DYN_INPUT: Box<dyn Fn(&'a u8)>;
    const DYN_OUTPUT: Box<dyn Fn() -> &'a u8>;
    const STATIC_FUNCTION: fn(u8) -> u8;
    const STATIC_DYN: Option<Box<dyn Debug>> = None;
}

#[reflect]
trait HrtbConstContract {
    const BOUND_FUNCTION: for<'a> fn(&'a u8);
    const ELIDED_FUNCTION: fn(&u8);
    const BOUND_DYN: Option<Box<dyn for<'a> Fn(&'a u8)>> = None;
    const ELIDED_DYN: Option<Box<dyn Fn(&u8)>> = None;
}

fn main() {}
