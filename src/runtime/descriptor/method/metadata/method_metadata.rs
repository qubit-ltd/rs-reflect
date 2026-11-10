// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Source declaration metadata, shared by all effective implementations.
pub struct MethodMetadata {
    /// Returns the unique source identifier when the parameter pattern binds a
    /// name.
    pub(crate) name: &'static str,
    /// Inherent or trait implementation that supplies this effective method.
    pub(crate) namespace: MethodNamespaceMetadata,
    /// Visibility recorded for this method declaration.
    pub(crate) visibility: MethodVisibility,
    /// Receiver kind and type metadata used to validate invocation binding.
    pub(crate) receiver: Option<ReceiverDescriptor>,
    /// Source-ordered metadata for the method’s non-receiver arguments.
    pub(crate) parameters: &'static [ParameterMetadata],
    /// Return category and type metadata for values produced by the method.
    pub(crate) output: &'static ReturnMetadata,
    /// Const, async, unsafe, ABI, and variadic facts copied from the
    /// declaration.
    pub(crate) qualifiers: MethodQualifiers,
    /// Generic parameter metadata required to select a method specialization.
    pub(crate) generics: &'static GenericDefinitionMetadata,
    /// Whether the declaration requires specialization before invocation.
    pub(crate) is_generic: bool,
    /// Whether a trait method uses its default body rather than an
    /// implementation override.
    pub(crate) default_body: bool,
}
