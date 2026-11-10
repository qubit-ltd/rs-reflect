// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Concrete type and const arguments used to select a method specialization.
/// Concrete type and const arguments used to select a method specialization.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MethodArguments {
    /// Generic arguments supplied when selecting a method specialization.
    pub(super) arguments: Vec<GenericArgumentIdentity>,
    /// Returns the selector list used to compare generic arguments.
    pub(super) selectors: Vec<GenericArgumentSelector>,
}

impl MethodArguments {
    /// Creates an empty generic argument list.
    pub fn new() -> Self {
        Self::default()
    }
    /// Appends a concrete type argument.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    pub fn type_of<T: crate::runtime::descriptor::Reflect + ?Sized>(mut self) -> Self {
        self.arguments
            .push(GenericArgumentIdentity::Type(std::any::type_name::<T>().into()));
        self.selectors.push(GenericArgumentSelector::Type(TypeId::of::<T>()));
        self
    }
    /// Appends a concrete const argument.
    ///
    /// Primitive integer, boolean, and character values participate in exact
    /// specialization selection. Other `Any` values are retained for error
    /// diagnostics but never treated as a matching const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub fn constant<T: Any>(mut self, value: T) -> Self {
        let (display, selector) = const_argument_identity(&value);
        self.arguments.push(GenericArgumentIdentity::Const(display));
        self.selectors.push(selector);
        self
    }

    /// Returns the number of supplied generic arguments.
    pub fn len(&self) -> usize {
        self.arguments.len()
    }

    /// Returns whether no generic arguments were supplied.
    pub fn is_empty(&self) -> bool {
        self.arguments.is_empty()
    }
}

/// Builds the stable identity used to compare const generic arguments.
/// # Parameters
///
/// - `value`: Owned value retained for validation and construction.
///
/// # Type Parameters
///
/// - `T`: Concrete Rust type whose metadata or value is used by the operation.
///
/// # Parameters
///
/// - `value`: Owned value retained for validation and construction.
fn const_argument_identity<T: Any>(value: &T) -> (Box<str>, GenericArgumentSelector) {
    macro_rules! integer {
        ($($ty:ty),+ $(,)?) => {
            $(if let Some(value) = (value as &dyn Any).downcast_ref::<$ty>() {
                let text = value.to_string();
                return (
                    format!("{}:{text}", std::any::type_name::<$ty>()).into(),
                    GenericArgumentSelector::Const {
                        type_id: TypeId::of::<$ty>(),
                        value: text.into_boxed_str(),
                    },
                );
            })+
        };
    }
    integer!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
    if let Some(value) = (value as &dyn Any).downcast_ref::<bool>() {
        let text = value.to_string();
        return (
            format!("bool:{text}").into(),
            GenericArgumentSelector::Const {
                type_id: TypeId::of::<bool>(),
                value: text.into_boxed_str(),
            },
        );
    }
    if let Some(value) = (value as &dyn Any).downcast_ref::<char>() {
        return (
            format!("char:{value:?}").into(),
            GenericArgumentSelector::Const {
                type_id: TypeId::of::<char>(),
                value: u32::from(*value).to_string().into_boxed_str(),
            },
        );
    }
    (
        format!("{}:<unsupported>", std::any::type_name::<T>()).into(),
        GenericArgumentSelector::UnsupportedConst(TypeId::of::<T>()),
    )
}
