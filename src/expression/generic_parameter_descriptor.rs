// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Parameters declared by a generic definition.

use std::hash::Hash;
use std::hash::Hasher;

use crate::expression::ConstExpression;
use crate::expression::DiagnosticText;
use crate::expression::ExpressionName;
use crate::expression::LifetimeExpression;
use crate::expression::PredicateDescriptor;
use crate::expression::TypeExpression;

/// A single parameter declared by a generic definition.
#[derive(Clone, Debug)]
pub enum GenericParameterDescriptor {
    /// A lifetime parameter such as `'a`.
    Lifetime {
        /// The lifetime parameter name without its leading apostrophe.
        name: ExpressionName,
        /// Lifetime bounds in declaration order.
        bounds: Box<[LifetimeExpression]>,
        /// Optional source-oriented diagnostic text excluded from identity.
        diagnostic: DiagnosticText,
    },
    /// A type parameter such as `T`.
    Type {
        /// The type parameter name.
        name: ExpressionName,
        /// Bounds in declaration order.
        bounds: Box<[PredicateDescriptor]>,
        /// The optional default type.
        default: Option<TypeExpression>,
        /// Optional source-oriented diagnostic text excluded from identity.
        diagnostic: DiagnosticText,
    },
    /// A const parameter such as `const N: usize`.
    Const {
        /// The const parameter name.
        name: ExpressionName,
        /// The declared const type.
        ty: Box<TypeExpression>,
        /// The optional default const expression.
        default: Option<ConstExpression>,
        /// Optional source-oriented diagnostic text excluded from identity.
        diagnostic: DiagnosticText,
    },
}

impl PartialEq for GenericParameterDescriptor {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Lifetime {
                    name,
                    bounds,
                    diagnostic: _,
                },
                Self::Lifetime {
                    name: other_name,
                    bounds: other_bounds,
                    diagnostic: _,
                },
            ) => name == other_name && bounds == other_bounds,
            (
                Self::Type {
                    name,
                    bounds,
                    default,
                    diagnostic: _,
                },
                Self::Type {
                    name: other_name,
                    bounds: other_bounds,
                    default: other_default,
                    diagnostic: _,
                },
            ) => name == other_name && bounds == other_bounds && default == other_default,
            (
                Self::Const {
                    name,
                    ty,
                    default,
                    diagnostic: _,
                },
                Self::Const {
                    name: other_name,
                    ty: other_ty,
                    default: other_default,
                    diagnostic: _,
                },
            ) => name == other_name && ty == other_ty && default == other_default,
            _ => false,
        }
    }
}

impl Eq for GenericParameterDescriptor {}

impl Hash for GenericParameterDescriptor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Lifetime {
                name,
                bounds,
                diagnostic: _,
            } => {
                name.hash(state);
                bounds.hash(state);
            }
            Self::Type {
                name,
                bounds,
                default,
                diagnostic: _,
            } => {
                name.hash(state);
                bounds.hash(state);
                default.hash(state);
            }
            Self::Const {
                name,
                ty,
                default,
                diagnostic: _,
            } => {
                name.hash(state);
                ty.hash(state);
                default.hash(state);
            }
        }
    }
}
