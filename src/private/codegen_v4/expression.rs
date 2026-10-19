// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural type-expression contracts for generated metadata.
//!
//! Generated code needs constructors for lifetime, const, and type parameters;
//! generic arguments; predicates; and concrete or unresolved type expressions.
//! The current runtime stores these in `runtime::expression::{ExpressionData,
//! TypeExpression}` and `runtime::descriptor::generic` metadata. Those types and
//! their fields are crate-private, so the façade must expose validated typed
//! constructors rather than raw text or forgeable identities.

pub use crate::runtime::descriptor::{
    GenericArgumentMetadata, GenericDefinitionMetadata, GenericParameterMetadata,
    GenericParameterMetadataKind, StructKind, TypeDefinitionKind, TypeDefinitionMetadata,
};
pub use crate::runtime::expression::{
    ConstExpression, ExpressionData, FunctionPointerData, GenericExpressionArgument,
    LifetimeExpression, OpaqueTypeData, PredicateData, TraitBoundModifier, TraitExpressionData,
    TraitObjectData,
};

/// Static source-level expression data emitted by the derive crate.
pub type TypeExpressionData = ExpressionData;
