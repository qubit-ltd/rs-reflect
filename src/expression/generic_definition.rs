// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Generic parameter declarations and their where predicates.

use std::hash::Hash;
use std::hash::Hasher;

use crate::expression::ConstExpression;
use crate::expression::DiagnosticText;
use crate::expression::LifetimeExpression;
use crate::expression::PredicateDescriptor;
use crate::expression::TypeExpression;

/// The generic declaration shared by all concrete instances of a reflected
/// item.
///
/// Parameters and predicates preserve source declaration order.  It describes a
/// declaration; it neither synthesizes a runtime type identity for
/// lifetime-only instantiations nor evaluates predicates at runtime.
#[derive(Clone, Debug)]
pub struct GenericDefinitionDescriptor {
    /// Lifetime, type, and const parameters in declaration order.
    pub parameters: Box<[GenericParameterDescriptor]>,
    /// Where-clause predicates in declaration order.
    pub predicates: Box<[PredicateDescriptor]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}

impl PartialEq for GenericDefinitionDescriptor {
    fn eq(&self, other: &Self) -> bool {
        self.parameters == other.parameters && self.predicates == other.predicates
    }
}

impl Eq for GenericDefinitionDescriptor {}

impl Hash for GenericDefinitionDescriptor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.parameters.hash(state);
        self.predicates.hash(state);
    }
}

/// A single parameter declared by a generic definition.
#[derive(Clone, Debug)]
pub enum GenericParameterDescriptor {
    /// A lifetime parameter such as `'a`.
    Lifetime {
        /// The lifetime parameter name without its leading apostrophe.
        name: Box<str>,
        /// Lifetime bounds in declaration order.
        bounds: Box<[LifetimeExpression]>,
        /// Optional source-oriented diagnostic text excluded from identity.
        diagnostic: DiagnosticText,
    },
    /// A type parameter such as `T`.
    Type {
        /// The type parameter name.
        name: Box<str>,
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
        name: Box<str>,
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
