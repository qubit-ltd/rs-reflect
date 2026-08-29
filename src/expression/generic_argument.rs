//! Generic arguments and structural const expressions.

use std::hash::{Hash, Hasher};

use crate::expression::{LifetimeExpression, PredicateDescriptor, TypeExpression};

/// An argument applied to a generic path segment.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GenericArgument {
    /// A type argument such as `Vec<u8>`'s `u8`.
    Type(TypeExpression),
    /// A lifetime argument such as `Ref<'a, T>`'s `'a`.
    Lifetime(LifetimeExpression),
    /// A const argument such as `Array<T, 4>`'s `4`.
    Const(ConstGenericArgument),
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

/// A typed const argument applied to a generic parameter.
///
/// The declared type and structural value determine identity. The normalized
/// text is retained only for diagnostics and is deliberately excluded from
/// equality and hashing.
#[derive(Clone, Debug)]
pub struct ConstGenericArgument {
    /// The const parameter's declared type.
    pub declared_type: Box<TypeExpression>,
    /// The const argument's structural value.
    pub value: ConstExpression,
    /// A normalized, source-oriented rendering of the const argument.
    pub normalized_diagnostic: Box<str>,
}

impl PartialEq for ConstGenericArgument {
    fn eq(&self, other: &Self) -> bool {
        self.declared_type == other.declared_type && self.value == other.value
    }
}

impl Eq for ConstGenericArgument {}

impl Hash for ConstGenericArgument {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.declared_type.hash(state);
        self.value.hash(state);
    }
}

/// A structural const expression used by a generic argument, array length, or const default.
///
/// Values are stored as typed data rather than source tokens.  A path can identify a named
/// const item or parameter, but this descriptor does not attempt runtime const evaluation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstExpression {
    /// A signed integer value.
    SignedInteger(i128),
    /// An unsigned integer value.
    UnsignedInteger(u128),
    /// A boolean literal.
    Boolean(bool),
    /// A character literal.
    Character(char),
    /// A named const generic parameter.
    Parameter(Box<str>),
    /// A qualified path to a const item.
    Path(Box<[Box<str>]>),
}
