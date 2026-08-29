//! Built-in reflected type descriptors.

mod array;
mod function;
#[path = "../registry/interner.rs"]
pub(crate) mod interner;
mod map;
mod option;
mod pointer;
mod raw_pointer;
mod reference;
mod scalar;
mod sequence;
mod set;
mod slice;
mod trait_object;
mod tuple;
