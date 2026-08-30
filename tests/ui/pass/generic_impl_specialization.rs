use std::marker::PhantomData;

use qubit_reflect::Reflect;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
#[reflect(opaque)]
struct Page<T>(PhantomData<T>);

#[reflect_impl(specialize(T = u8))]
impl<T> Page<T> {
    fn reflected_count() -> usize {
        1
    }
}

fn main() {
    let _ = Page::<u8>::type_descriptor();
}
