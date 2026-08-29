//! Generic arguments and structural const expressions.

use crate::expression::{LifetimeExpression, PredicateDescriptor, TypeExpression};

/// An argument applied to a generic path segment.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GenericArgument {
    /// A type argument such as `Vec<u8>`'s `u8`.
    Type(TypeExpression),
    /// A lifetime argument such as `Ref<'a, T>`'s `'a`.
    Lifetime(LifetimeExpression),
    /// A const argument such as `Array<T, 4>`'s `4`.
    Const(ConstExpression),
    /// An associated type equality such as `Iterator<Item = T>`.
    AssociatedType {
        /// The associated type name.
        name: Box<str>,
        /// The associated type value.
        value: Box<TypeExpression>,
    },
    /// An associated type bound such as `Iterator<Item: Display>`.
    AssociatedTypeBound {
        /// The associated type name.
        name: Box<str>,
        /// Bounds that apply to the associated type.
        bounds: Box<[PredicateDescriptor]>,
    },
}

/// A structural const expression used by a generic argument, array length, or const default.
///
/// Values are stored as typed data rather than source tokens.  A path can identify a named
/// const item or parameter, but this descriptor does not attempt runtime const evaluation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstExpression {
    /// An unsigned integer literal.
    Integer(u128),
    /// A boolean literal.
    Boolean(bool),
    /// A character literal.
    Character(char),
    /// A named const generic parameter.
    Parameter(Box<str>),
    /// A qualified path to a const item.
    Path(Box<[Box<str>]>),
}
