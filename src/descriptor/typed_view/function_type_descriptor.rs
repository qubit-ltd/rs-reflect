// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::__private::LazyTypeRef;
use crate::__private::LazyTypeRefList;
use crate::__private::TypeRefListSource;
use crate::__private::TypeRefSource;
use crate::descriptor::FunctionPointerKind;
use crate::descriptor::TypeRef;
use crate::expression::FunctionAbi;

/// The typed view of a function pointer signature.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let function = TypeDescriptor::of::<fn(u8) -> bool>().as_function().expect("function pointer type");
/// assert_eq!(function.parameters().len(), 1);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FunctionTypeDescriptor {
    /// Safe or unsafe function-pointer category.
    kind: FunctionPointerKind,
    /// Static calling-convention descriptor.
    abi: &'static FunctionAbi,
    /// Whether the signature accepts a variadic tail.
    variadic: bool,
    /// Eager or lazily resolved parameter types in declaration order.
    parameters: TypeRefListSource,
    /// Eager or lazily resolved return type.
    return_type: TypeRefSource,
}

impl FunctionTypeDescriptor {
    /// Creates a function-pointer view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Safe or unsafe function-pointer category.
    /// - `abi`: Calling convention.
    /// - `variadic`: Whether the signature accepts a variadic tail.
    /// - `parameters`: Eager parameter types in declaration order.
    /// - `return_type`: Eager return type.
    ///
    /// # Returns
    ///
    /// Returns a function signature view backed by the supplied types.
    pub(crate) const fn new(
        kind: FunctionPointerKind,
        abi: &'static FunctionAbi,
        variadic: bool,
        parameters: &'static [TypeRef],
        return_type: &'static TypeRef,
    ) -> Self {
        Self {
            kind,
            abi,
            variadic,
            parameters: TypeRefListSource::Eager(parameters),
            return_type: TypeRefSource::Eager(return_type),
        }
    }

    /// Creates a function view whose signature relationships resolve on first
    /// navigation.
    ///
    /// # Parameters
    ///
    /// - `kind`: Safe or unsafe function-pointer category.
    /// - `abi`: Calling convention.
    /// - `variadic`: Whether the signature accepts a variadic tail.
    /// - `parameters`: Lazy parameter type list in declaration order.
    /// - `return_type`: Lazy return type source.
    ///
    /// # Returns
    ///
    /// Returns a function signature view backed by the lazy type sources.
    pub(crate) const fn new_lazy(
        kind: FunctionPointerKind,
        abi: &'static FunctionAbi,
        variadic: bool,
        parameters: &'static LazyTypeRefList,
        return_type: &'static LazyTypeRef,
    ) -> Self {
        Self {
            kind,
            abi,
            variadic,
            parameters: TypeRefListSource::Lazy(parameters),
            return_type: TypeRefSource::Lazy(return_type),
        }
    }

    /// Returns whether the function pointer is safe or unsafe.
    ///
    /// # Returns
    ///
    /// Returns the function-pointer safety category.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> FunctionPointerKind {
        self.kind
    }

    /// Returns the declared calling convention.
    ///
    /// # Returns
    ///
    /// Returns the static ABI descriptor.
    #[must_use]
    #[inline]
    pub const fn abi(&self) -> &'static FunctionAbi {
        self.abi
    }

    /// Returns whether the function pointer accepts a C-style variadic tail.
    ///
    /// # Returns
    ///
    /// Returns `true` when the signature is variadic.
    #[must_use]
    #[inline]
    pub const fn is_variadic(&self) -> bool {
        self.variadic
    }

    /// Returns parameter types in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the resolved static parameter list, initializing lazy entries
    /// on first access.
    #[must_use]
    #[inline]
    pub fn parameters(&self) -> &'static [TypeRef] {
        self.parameters.get()
    }

    /// Returns the function return type.
    ///
    /// # Returns
    ///
    /// Returns the resolved static return type, initializing it on first
    /// access.
    #[must_use]
    #[inline]
    pub fn return_type(&self) -> &'static TypeRef {
        self.return_type.get()
    }
}
