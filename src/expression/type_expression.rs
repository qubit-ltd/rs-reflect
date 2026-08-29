//! Structural representations of Rust type expressions.

use std::hash::{Hash, Hasher};

use crate::expression::{
    ConstExpression, GenericArgument, LifetimeExpression, PredicateDescriptor,
};

macro_rules! impl_identity_without_diagnostic {
    ($type:ty { $first:ident $(, $field:ident)* $(,)? }) => {
        impl PartialEq for $type {
            fn eq(&self, other: &Self) -> bool {
                self.$first == other.$first $(&& self.$field == other.$field)*
            }
        }

        impl Eq for $type {}

        impl Hash for $type {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.$first.hash(state);
                $(self.$field.hash(state);)*
            }
        }
    };
}

/// Source-oriented text that supplements diagnostics.
///
/// This value has ordinary text equality and hashing. Descriptor implementations deliberately
/// exclude diagnostic fields from their structural identity.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DiagnosticText(pub Option<Box<str>>);

impl From<&str> for DiagnosticText {
    fn from(value: &str) -> Self {
        Self(Some(value.into()))
    }
}

impl From<String> for DiagnosticText {
    fn from(value: String) -> Self {
        Self(Some(value.into_boxed_str()))
    }
}

/// A closed, navigable Rust type expression independent of parser implementation types.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeExpression {
    /// A concrete path such as `std::vec::Vec<T>`.
    Concrete(ConcreteTypeExpression),
    /// A type parameter such as `T`.
    Parameter(Box<str>),
    /// The `Self` type.
    SelfType,
    /// An associated type projection such as `<T as Trait>::Item`.
    Associated(AssociatedTypeExpression),
    /// A shared or mutable reference.
    Reference(ReferenceTypeExpression),
    /// A const or mutable raw pointer.
    RawPointer(RawPointerTypeExpression),
    /// A slice expression such as `[T]`.
    Slice(Box<TypeExpression>),
    /// An array expression such as `[T; N]`.
    Array(ArrayTypeExpression),
    /// A tuple expression, including the empty tuple.
    Tuple(Box<[TypeExpression]>),
    /// A function pointer expression.
    FunctionPointer(FunctionPointerExpression),
    /// A `dyn Trait` object expression.
    TraitObject(TraitObjectExpression),
    /// An `impl Trait` opaque expression.
    Opaque(OpaqueTypeExpression),
    /// The never type `!`.
    Never,
}

/// A concrete type path and its final-segment generic arguments.
#[derive(Clone, Debug)]
pub struct ConcreteTypeExpression {
    /// Path segments in declaration order, for example `std`, `vec`, and `Vec`.
    pub path: Box<[Box<str>]>,
    /// Generic arguments of the final path segment in declaration order.
    pub arguments: Box<[GenericArgument]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}

impl_identity_without_diagnostic!(ConcreteTypeExpression { path, arguments });

/// An associated type projection.
#[derive(Clone, Debug)]
pub struct AssociatedTypeExpression {
    /// The self type whose associated item is projected.
    pub self_type: Box<TypeExpression>,
    /// The optional qualifying trait path from an `as Trait` clause.
    pub trait_path: Option<Box<TypeExpression>>,
    /// The associated item name.
    pub item: Box<str>,
    /// Generic arguments applied to the associated type.
    pub arguments: Box<[GenericArgument]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}

impl_identity_without_diagnostic!(AssociatedTypeExpression {
    self_type,
    trait_path,
    item,
    arguments
});

/// A shared or mutable reference type.
#[derive(Clone, Debug)]
pub struct ReferenceTypeExpression {
    /// The reference lifetime, including [`LifetimeExpression::Elided`] when
    /// omitted.
    pub lifetime: LifetimeExpression,
    /// Whether this is a mutable reference.
    pub mutable: bool,
    /// The referenced type.
    pub target: Box<TypeExpression>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}

impl_identity_without_diagnostic!(ReferenceTypeExpression {
    lifetime,
    mutable,
    target
});

/// A const or mutable raw pointer type.
#[derive(Clone, Debug)]
pub struct RawPointerTypeExpression {
    /// Whether this is a mutable raw pointer.
    pub mutable: bool,
    /// The pointee type.
    pub target: Box<TypeExpression>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}

impl_identity_without_diagnostic!(RawPointerTypeExpression { mutable, target });

/// An array type and its structural length expression.
#[derive(Clone, Debug)]
pub struct ArrayTypeExpression {
    /// The repeated element type.
    pub element: Box<TypeExpression>,
    /// The array length expression.
    pub length: ConstExpression,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}

impl_identity_without_diagnostic!(ArrayTypeExpression { element, length });

/// A function pointer's calling convention.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FunctionAbi {
    /// The default Rust ABI.
    Rust,
    /// The C ABI.
    C,
    /// The platform system ABI.
    System,
    /// Any explicitly named ABI not covered by a standard variant.
    Other(Box<str>),
}

/// A function pointer's safety qualifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FunctionSafety {
    /// A safe function pointer.
    Safe,
    /// An `unsafe fn` pointer.
    Unsafe,
}

/// A function pointer signature.
#[derive(Clone, Debug)]
pub struct FunctionPointerExpression {
    /// The function's ABI.
    pub abi: FunctionAbi,
    /// The function's safety qualifier.
    pub safety: FunctionSafety,
    /// Whether the final argument is variadic.
    pub variadic: bool,
    /// Lifetimes introduced by a higher-ranked function pointer.
    pub higher_ranked_lifetimes: Box<[LifetimeExpression]>,
    /// Parameter types in declaration order.
    pub parameters: Box<[TypeExpression]>,
    /// The function return type.
    pub return_type: Box<TypeExpression>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}

impl_identity_without_diagnostic!(FunctionPointerExpression {
    abi,
    safety,
    variadic,
    higher_ranked_lifetimes,
    parameters,
    return_type
});

/// A `dyn Trait` object and the predicates it must satisfy.
#[derive(Clone, Debug)]
pub struct TraitObjectExpression {
    /// Trait and lifetime predicates in declaration order.
    pub bounds: Box<[PredicateDescriptor]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}

impl_identity_without_diagnostic!(TraitObjectExpression { bounds });

/// An `impl Trait` opaque type and the predicates it must satisfy.
#[derive(Clone, Debug)]
pub struct OpaqueTypeExpression {
    /// Trait and lifetime predicates in declaration order.
    pub bounds: Box<[PredicateDescriptor]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}

impl_identity_without_diagnostic!(OpaqueTypeExpression { bounds });
