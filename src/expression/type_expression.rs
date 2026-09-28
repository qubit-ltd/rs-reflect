// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Structural representations of Rust type expressions.

#[macro_use]
mod identity;
mod array_type_expression;
mod associated_type_expression;
mod concrete_path_segment;
mod concrete_type_expression;
mod diagnostic_text;
mod function_abi;
mod function_pointer_expression;
mod function_safety;
mod opaque_type_expression;
mod raw_pointer_type_expression;
mod reference_type_expression;
mod trait_object_expression;
#[path = "type_expression/type_expression.rs"]
mod type_expression_definition;

pub use array_type_expression::ArrayTypeExpression;
pub use associated_type_expression::AssociatedTypeExpression;
pub use concrete_path_segment::ConcretePathSegment;
pub use concrete_type_expression::ConcreteTypeExpression;
pub use diagnostic_text::DiagnosticText;
pub use function_abi::FunctionAbi;
pub use function_pointer_expression::FunctionPointerExpression;
pub use function_safety::FunctionSafety;
pub use opaque_type_expression::OpaqueTypeExpression;
pub use raw_pointer_type_expression::RawPointerTypeExpression;
pub use reference_type_expression::ReferenceTypeExpression;
pub use trait_object_expression::TraitObjectExpression;
pub use type_expression_definition::TypeExpression;
