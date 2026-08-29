//! Structural representations of Rust type expressions.

use std::hash::{Hash, Hasher};

use crate::expression::{ConstExpression, GenericArgument, LifetimeExpression, PredicateDescriptor};

/// Source-oriented text that supplements diagnostics without changing descriptor identity.
///
/// Equality and hashing deliberately ignore the text so equivalent structural expressions have
/// the same identity regardless of formatting, aliases, or parser source spans.
#[derive(Clone, Debug, Default)]
pub struct DiagnosticText(pub Option<Box<str>>);

impl PartialEq for DiagnosticText {
    /// Compares diagnostic annotations without making them part of structural identity.
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for DiagnosticText {}

impl Hash for DiagnosticText {
    /// Omits diagnostic annotations from structural hashes.
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}

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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConcreteTypeExpression {
    /// Path segments in declaration order, for example `std`, `vec`, and `Vec`.
    pub path: Box<[Box<str>]>,
    /// Generic arguments of the final path segment in declaration order.
    pub arguments: Box<[GenericArgument]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}

/// An associated type projection.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssociatedTypeExpression {
    /// The type or trait-qualified base of the projection.
    pub base: Box<TypeExpression>,
    /// The associated type name.
    pub name: Box<str>,
    /// Generic arguments applied to the associated type.
    pub arguments: Box<[GenericArgument]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}

/// A shared or mutable reference type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ReferenceTypeExpression {
    /// An explicit lifetime, or `None` when the lifetime is elided.
    pub lifetime: Option<LifetimeExpression>,
    /// Whether this is a mutable reference.
    pub mutable: bool,
    /// The referenced type.
    pub target: Box<TypeExpression>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}

/// A const or mutable raw pointer type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RawPointerTypeExpression {
    /// Whether this is a mutable raw pointer.
    pub mutable: bool,
    /// The pointee type.
    pub target: Box<TypeExpression>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}

/// An array type and its structural length expression.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArrayTypeExpression {
    /// The repeated element type.
    pub element: Box<TypeExpression>,
    /// The array length expression.
    pub length: ConstExpression,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}

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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

/// A `dyn Trait` object and the predicates it must satisfy.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TraitObjectExpression {
    /// Trait and lifetime predicates in declaration order.
    pub bounds: Box<[PredicateDescriptor]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}

/// An `impl Trait` opaque type and the predicates it must satisfy.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OpaqueTypeExpression {
    /// Trait and lifetime predicates in declaration order.
    pub bounds: Box<[PredicateDescriptor]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub diagnostic: DiagnosticText,
}
