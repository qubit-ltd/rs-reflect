#![allow(unsafe_code)]

mod borrowed;
mod builtin;
mod dynamic;
mod failure;
mod mode;
mod owned;

pub(crate) use borrowed::{
    DynamicMut, DynamicRef, ValueForMut, ValueForMut as RuntimeValueForMut, ValueForRef,
    ValueForRef as RuntimeValueForRef,
};
pub(crate) use dynamic::{DynamicValue, DynamicValueKind};
pub(crate) use failure::{DowncastFailure, TypeMismatch};
pub(crate) use mode::ValueMode;
pub(crate) use mode::ValueMode as RuntimeValueMode;
pub use mode::{Local, ThreadSafe};
pub(crate) use owned::{DynamicOwned, ValueFor, ValueFor as RuntimeValueFor};
