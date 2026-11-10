// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Source-level method qualifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MethodQualifiers {
    /// Whether the source declaration marks the method `const`.
    pub(crate) is_const: bool,
    /// Whether the source declaration marks the method `async`.
    pub(crate) is_async: bool,
    /// Whether calling the source method requires an unsafe operation.
    pub(crate) is_unsafe: bool,
    /// Optional calling convention declared for the method.
    pub(crate) abi: Option<FunctionAbi>,
    /// Whether the method accepts a C-style variable argument list.
    pub(crate) is_variadic: bool,
}

impl MethodQualifiers {
    /// Creates source qualifiers without a custom ABI.
    ///
    /// # Parameters
    ///
    /// - `is_const`: Whether the source method is declared `const`.
    /// - `is_async`: Whether the source method is declared `async`.
    /// - `is_unsafe`: Whether calling the source method requires an unsafe
    ///   operation.
    /// - `is_variadic`: Whether the declaration accepts variadic arguments.
    pub const fn new(is_const: bool, is_async: bool, is_unsafe: bool, is_variadic: bool) -> Self {
        Self {
            is_const,
            is_async,
            is_unsafe,
            abi: None,
            is_variadic,
        }
    }

    /// Records a non-default ABI without accepting runtime identity data.
    ///
    /// # Parameters
    ///
    /// - `abi`: Optional calling convention recorded from the declaration.
    pub const fn with_abi(mut self, abi: Option<FunctionAbi>) -> Self {
        self.abi = abi;
        self
    }
}
