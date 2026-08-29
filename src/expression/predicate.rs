//! Where-clause and trait-bound predicates.

use std::hash::Hash;
use std::hash::Hasher;

use crate::expression::DiagnosticText;
use crate::expression::LifetimeExpression;
use crate::expression::TypeExpression;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraitBoundModifier {
    None,
    Maybe,
}

/// A structural predicate from a generic declaration, trait object, or opaque
/// type bound.
#[derive(Clone, Debug)]
pub enum PredicateDescriptor {
    /// Bounds placed on a type, for example `T: Display + Send`.
    TypeBound {
        /// The type being constrained.
        subject: TypeExpression,
        /// Trait or lifetime bounds in declaration order.
        bounds: Box<[TypeExpression]>,
        bound_modifiers: Box<[TraitBoundModifier]>,
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

impl PartialEq for PredicateDescriptor {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::TypeBound {
                    subject,
                    bounds,
                    bound_modifiers,
                    higher_ranked_lifetimes,
                    diagnostic: _,
                },
                Self::TypeBound {
                    subject: other_subject,
                    bounds: other_bounds,
                    bound_modifiers: other_modifiers,
                    higher_ranked_lifetimes: other_lifetimes,
                    diagnostic: _,
                },
            ) => {
                subject == other_subject
                    && bounds == other_bounds
                    && bound_modifiers == other_modifiers
                    && higher_ranked_lifetimes == other_lifetimes
            }
            (
                Self::LifetimeOutlives {
                    lifetime,
                    bounds,
                    diagnostic: _,
                },
                Self::LifetimeOutlives {
                    lifetime: other_lifetime,
                    bounds: other_bounds,
                    diagnostic: _,
                },
            ) => lifetime == other_lifetime && bounds == other_bounds,
            (
                Self::TypeOutlives {
                    ty,
                    lifetime,
                    diagnostic: _,
                },
                Self::TypeOutlives {
                    ty: other_ty,
                    lifetime: other_lifetime,
                    diagnostic: _,
                },
            ) => ty == other_ty && lifetime == other_lifetime,
            (
                Self::TypeEquality {
                    left,
                    right,
                    diagnostic: _,
                },
                Self::TypeEquality {
                    left: other_left,
                    right: other_right,
                    diagnostic: _,
                },
            ) => left == other_left && right == other_right,
            _ => false,
        }
    }
}

impl Eq for PredicateDescriptor {}

impl Hash for PredicateDescriptor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::TypeBound {
                subject,
                bounds,
                bound_modifiers,
                higher_ranked_lifetimes,
                diagnostic: _,
            } => {
                subject.hash(state);
                bounds.hash(state);
                bound_modifiers.hash(state);
                higher_ranked_lifetimes.hash(state);
            }
            Self::LifetimeOutlives {
                lifetime,
                bounds,
                diagnostic: _,
            } => {
                lifetime.hash(state);
                bounds.hash(state);
            }
            Self::TypeOutlives {
                ty,
                lifetime,
                diagnostic: _,
            } => {
                ty.hash(state);
                lifetime.hash(state);
            }
            Self::TypeEquality {
                left,
                right,
                diagnostic: _,
            } => {
                left.hash(state);
                right.hash(state);
            }
        }
    }
}
