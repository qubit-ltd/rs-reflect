//! Where-clause and trait-bound predicates.

use crate::expression::{DiagnosticText, LifetimeExpression, TypeExpression};

/// A structural predicate from a generic declaration, trait object, or opaque type bound.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PredicateDescriptor {
    /// Bounds placed on a type, for example `T: Display + Send`.
    TypeBound {
        /// The type being constrained.
        subject: TypeExpression,
        /// Trait or lifetime bounds in declaration order.
        bounds: Box<[TypeExpression]>,
        /// Lifetimes introduced by a higher-ranked trait bound.
        higher_ranked_lifetimes: Box<[LifetimeExpression]>,
        /// Optional source-oriented diagnostic text excluded from identity.
        diagnostic: DiagnosticText,
    },
    /// A lifetime outlives relation such as `'a: 'b`.
    LifetimeOutlives {
        /// The lifetime that must outlive every bound lifetime.
        lifetime: LifetimeExpression,
        /// Lifetimes required to be outlived, in declaration order.
        bounds: Box<[LifetimeExpression]>,
        /// Optional source-oriented diagnostic text excluded from identity.
        diagnostic: DiagnosticText,
    },
    /// A type outlives relation such as `T: 'a`.
    TypeOutlives {
        /// The type that must outlive the lifetime.
        ty: TypeExpression,
        /// The required lifetime.
        lifetime: LifetimeExpression,
        /// Optional source-oriented diagnostic text excluded from identity.
        diagnostic: DiagnosticText,
    },
    /// A type equality relation such as `<T as Trait>::Item = U`.
    TypeEquality {
        /// The left side of the equality.
        left: TypeExpression,
        /// The right side of the equality.
        right: TypeExpression,
        /// Optional source-oriented diagnostic text excluded from identity.
        diagnostic: DiagnosticText,
    },
}
