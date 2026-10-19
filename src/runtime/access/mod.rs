#![allow(unsafe_code)]

mod field_adapter;

pub use field_adapter::TypedFieldAdapter;
pub(crate) use field_adapter::{
    DynamicFieldAdapter, DynamicFieldMode, DynamicOnlyFieldAdapter, FieldAdapter,
};
