use core::fmt::Debug;

use qubit_reflect::reflect;

#[reflect]
trait UnsizedAssociatedConstContract {
    const SLICE: [u8];
    const TRAIT_OBJECT: dyn Debug;
    const BOXED_TRAIT_OBJECT: Option<Box<dyn Debug>> = None;
}

fn main() {}
