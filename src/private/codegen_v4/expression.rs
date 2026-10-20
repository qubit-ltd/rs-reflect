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
//! TypeExpression}` and `runtime::descriptor::generic` metadata. Those types
//! and their fields are crate-private, so the façade must expose validated
//! typed constructors rather than raw text or forgeable identities.

pub use crate::runtime::descriptor::GenericArgumentMetadata;
pub use crate::runtime::descriptor::GenericDefinitionMetadata;
pub use crate::runtime::descriptor::GenericParameterMetadata;
pub use crate::runtime::descriptor::GenericParameterMetadataKind;
pub use crate::runtime::descriptor::StructKind;
pub use crate::runtime::descriptor::TypeDefinitionKind;
pub use crate::runtime::descriptor::TypeDefinitionMetadata;
pub use crate::runtime::expression::ConstExpression;
pub use crate::runtime::expression::ExpressionData;
pub use crate::runtime::expression::FunctionPointerData;
pub use crate::runtime::expression::GenericExpressionArgument;
pub use crate::runtime::expression::LifetimeExpression;
pub use crate::runtime::expression::OpaqueTypeData;
pub use crate::runtime::expression::PredicateData;
pub use crate::runtime::expression::TraitBoundModifier;
pub use crate::runtime::expression::TraitExpressionData;
pub use crate::runtime::expression::TraitObjectData;

/// Static source-level expression data emitted by the derive crate.
pub type TypeExpressionData = ExpressionData;
