use std::marker::PhantomData;

use qubit_reflect::Reflect;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
#[reflect(opaque)]
struct Constrained<T>(PhantomData<T>);

#[reflect_impl(specialize(T = String))]
impl<T: Copy> Constrained<T> {}

fn main() {}
