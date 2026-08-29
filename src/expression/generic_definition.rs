//! Generic parameter declarations and their where predicates.

use crate::expression::{
    ConstExpression, DiagnosticText, LifetimeExpression, PredicateDescriptor, TypeExpression,
};

/// The generic declaration shared by all concrete instances of a reflected item.
///
/// Parameters and predicates preserve source declaration order.  It describes a declaration;
/// it neither synthesizes a runtime type identity for lifetime-only instantiations nor evaluates
/// predicates at runtime.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericDefinitionDescriptor {
    /// Lifetime, type, and const parameters in declaration order.
    pub parameters: Box<[GenericParameterDescriptor]>,
    /// Where-clause predicates in declaration order.
    pub predicates: Box<[PredicateDescriptor]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}

/// A single parameter declared by a generic definition.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
