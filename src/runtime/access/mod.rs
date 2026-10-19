#![allow(unsafe_code)]

mod field_adapter;

pub(crate) use field_adapter::{
    DynamicFieldAdapter, DynamicFieldMode, DynamicOnlyFieldAdapter, FieldAdapter, TypedFieldAdapter,
};

#[cfg(test)]
mod tests;
