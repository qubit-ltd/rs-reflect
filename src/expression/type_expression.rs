// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Structural representations of Rust type expressions.

use std::hash::Hash;
use std::hash::Hasher;

use crate::expression::ConstExpression;
use crate::expression::ExpressionError;
use crate::expression::GenericArgument;
use crate::expression::LifetimeExpression;
use crate::expression::PredicateDescriptor;

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
/// This value has ordinary text equality and hashing. Descriptor
/// implementations deliberately exclude diagnostic fields from their structural
/// identity.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DiagnosticText(pub(crate) Option<Box<str>>);

impl DiagnosticText {
    /// Returns the diagnostic text when present.
    pub fn as_deref(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

impl From<Box<str>> for DiagnosticText {
    fn from(value: Box<str>) -> Self {
        Self(Some(value))
    }
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

/// A closed, navigable Rust type expression independent of parser
/// implementation types.
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
    /// Path segments in declaration order, for example `std`, `vec`, and
    /// `Vec`.
    pub(crate) path: Box<[Box<str>]>,
    /// Generic arguments of the final path segment in declaration order.
    pub(crate) arguments: Box<[GenericArgument]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl ConcreteTypeExpression {
    /// Creates a concrete type expression from a non-empty path.
    pub fn new<P, S>(path: P, arguments: impl IntoIterator<Item = GenericArgument>) -> Result<Self, ExpressionError>
    where
        P: IntoIterator<Item = S>,
        S: Into<Box<str>>,
    {
        let path = path.into_iter().map(Into::into).collect::<Box<[_]>>();
        if path.is_empty() {
            return Err(ExpressionError::EmptyConcretePath);
        }
        Ok(Self {
            path,
            arguments: arguments.into_iter().collect(),
            diagnostic: DiagnosticText::default(),
        })
    }

    /// Returns the path segments in declaration order.
    pub fn path(&self) -> &[Box<str>] {
        &self.path
    }

    /// Returns final-segment generic arguments in declaration order.
    pub fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Returns source-oriented diagnostic text when present.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }

    /// Attaches source-oriented diagnostic text.
    #[must_use]
    pub fn with_diagnostic(mut self, diagnostic: impl Into<Box<str>>) -> Self {
        self.diagnostic = DiagnosticText::from(diagnostic.into());
        self
    }
}

impl_identity_without_diagnostic!(ConcreteTypeExpression { path, arguments });

/// An associated type projection.
#[derive(Clone, Debug)]
pub struct AssociatedTypeExpression {
    /// The self type whose associated item is projected.
    pub(crate) self_type: Box<TypeExpression>,
    /// The optional qualifying trait path from an `as Trait` clause.
    pub(crate) trait_path: Option<Box<TypeExpression>>,
    /// The associated item name.
    pub(crate) item: Box<str>,
    /// Generic arguments applied to the associated type.
    pub(crate) arguments: Box<[GenericArgument]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl AssociatedTypeExpression {
    /// Creates an associated type projection.
    pub fn new(
        self_type: TypeExpression,
        trait_path: Option<TypeExpression>,
        item: impl Into<Box<str>>,
        arguments: impl Into<Box<[GenericArgument]>>,
    ) -> Self {
        Self {
            self_type: Box::new(self_type),
            trait_path: trait_path.map(Box::new),
            item: item.into(),
            arguments: arguments.into(),
            diagnostic: DiagnosticText::default(),
        }
    }

    /// Returns the projected self type.
    pub fn self_type(&self) -> &TypeExpression {
        &self.self_type
    }
    /// Returns the optional qualifying trait path.
    pub fn trait_path(&self) -> Option<&TypeExpression> {
        self.trait_path.as_deref()
    }
    /// Returns the associated item name.
    pub fn item(&self) -> &str {
        &self.item
    }
    /// Returns associated type arguments.
    pub fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }
    /// Returns source-oriented diagnostic text when present.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Attaches source-oriented diagnostic text.
    #[must_use]
    pub fn with_diagnostic(mut self, value: impl Into<Box<str>>) -> Self {
        self.diagnostic = DiagnosticText::from(value.into());
        self
    }
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
    pub(crate) lifetime: LifetimeExpression,
    /// Whether this is a mutable reference.
    pub(crate) mutable: bool,
    /// The referenced type.
    pub(crate) target: Box<TypeExpression>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl ReferenceTypeExpression {
    /// Creates a reference expression.
    pub fn new(lifetime: LifetimeExpression, mutable: bool, target: TypeExpression) -> Self {
        Self {
            lifetime,
            mutable,
            target: Box::new(target),
            diagnostic: DiagnosticText::default(),
        }
    }
    /// Returns the reference lifetime.
    pub fn lifetime(&self) -> &LifetimeExpression {
        &self.lifetime
    }
    /// Returns whether the reference is mutable.
    pub fn is_mutable(&self) -> bool {
        self.mutable
    }
    /// Returns the referenced type.
    pub fn target(&self) -> &TypeExpression {
        &self.target
    }
    /// Returns diagnostic text when present.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Attaches diagnostic text.
    #[must_use]
    pub fn with_diagnostic(mut self, value: impl Into<Box<str>>) -> Self {
        self.diagnostic = DiagnosticText::from(value.into());
        self
    }
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
    pub(crate) mutable: bool,
    /// The pointee type.
    pub(crate) target: Box<TypeExpression>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl RawPointerTypeExpression {
    /// Creates a raw pointer expression.
    pub fn new(mutable: bool, target: TypeExpression) -> Self {
        Self {
            mutable,
            target: Box::new(target),
            diagnostic: DiagnosticText::default(),
        }
    }
    /// Returns whether the pointer is mutable.
    pub fn is_mutable(&self) -> bool {
        self.mutable
    }
    /// Returns the pointee type.
    pub fn target(&self) -> &TypeExpression {
        &self.target
    }
    /// Returns diagnostic text when present.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Attaches diagnostic text.
    #[must_use]
    pub fn with_diagnostic(mut self, value: impl Into<Box<str>>) -> Self {
        self.diagnostic = DiagnosticText::from(value.into());
        self
    }
}

impl_identity_without_diagnostic!(RawPointerTypeExpression { mutable, target });

/// An array type and its structural length expression.
#[derive(Clone, Debug)]
pub struct ArrayTypeExpression {
    /// The repeated element type.
    pub(crate) element: Box<TypeExpression>,
    /// The array length expression.
    pub(crate) length: ConstExpression,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl ArrayTypeExpression {
    /// Creates an array expression.
    pub fn new(element: TypeExpression, length: ConstExpression) -> Self {
        Self {
            element: Box::new(element),
            length,
            diagnostic: DiagnosticText::default(),
        }
    }
    /// Returns the element type.
    pub fn element(&self) -> &TypeExpression {
        &self.element
    }
    /// Returns the structural length expression.
    pub fn length(&self) -> &ConstExpression {
        &self.length
    }
    /// Returns diagnostic text when present.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Attaches diagnostic text.
    #[must_use]
    pub fn with_diagnostic(mut self, value: impl Into<Box<str>>) -> Self {
        self.diagnostic = DiagnosticText::from(value.into());
        self
    }
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
    pub(crate) abi: FunctionAbi,
    /// The function's safety qualifier.
    pub(crate) safety: FunctionSafety,
    /// Whether the final argument is variadic.
    pub(crate) variadic: bool,
    /// Lifetimes introduced by a higher-ranked function pointer.
    pub(crate) higher_ranked_lifetimes: Box<[LifetimeExpression]>,
    /// Parameter types in declaration order.
    pub(crate) parameters: Box<[TypeExpression]>,
    /// The function return type.
    pub(crate) return_type: Box<TypeExpression>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl FunctionPointerExpression {
    /// Creates a function pointer expression.
    pub fn new(
        abi: FunctionAbi,
        safety: FunctionSafety,
        variadic: bool,
        higher_ranked_lifetimes: impl Into<Box<[LifetimeExpression]>>,
        parameters: impl Into<Box<[TypeExpression]>>,
        return_type: TypeExpression,
    ) -> Self {
        Self {
            abi,
            safety,
            variadic,
            higher_ranked_lifetimes: higher_ranked_lifetimes.into(),
            parameters: parameters.into(),
            return_type: Box::new(return_type),
            diagnostic: DiagnosticText::default(),
        }
    }
    /// Returns the calling convention.
    pub fn abi(&self) -> &FunctionAbi {
        &self.abi
    }
    /// Returns the safety qualifier.
    pub fn safety(&self) -> &FunctionSafety {
        &self.safety
    }
    /// Returns whether the signature is variadic.
    pub fn is_variadic(&self) -> bool {
        self.variadic
    }
    /// Returns higher-ranked lifetimes.
    pub fn higher_ranked_lifetimes(&self) -> &[LifetimeExpression] {
        &self.higher_ranked_lifetimes
    }
    /// Returns parameter types.
    pub fn parameters(&self) -> &[TypeExpression] {
        &self.parameters
    }
    /// Returns the return type.
    pub fn return_type(&self) -> &TypeExpression {
        &self.return_type
    }
    /// Returns diagnostic text when present.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Attaches diagnostic text.
    #[must_use]
    pub fn with_diagnostic(mut self, value: impl Into<Box<str>>) -> Self {
        self.diagnostic = DiagnosticText::from(value.into());
        self
    }
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
    pub(crate) bounds: Box<[PredicateDescriptor]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl TraitObjectExpression {
    /// Creates a trait object expression.
    pub fn new(bounds: impl Into<Box<[PredicateDescriptor]>>) -> Self {
        Self {
            bounds: bounds.into(),
            diagnostic: DiagnosticText::default(),
        }
    }
    /// Returns object bounds.
    pub fn bounds(&self) -> &[PredicateDescriptor] {
        &self.bounds
    }
    /// Returns diagnostic text when present.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Attaches diagnostic text.
    #[must_use]
    pub fn with_diagnostic(mut self, value: impl Into<Box<str>>) -> Self {
        self.diagnostic = DiagnosticText::from(value.into());
        self
    }
}

impl_identity_without_diagnostic!(TraitObjectExpression { bounds });

/// An `impl Trait` opaque type and the predicates it must satisfy.
#[derive(Clone, Debug)]
pub struct OpaqueTypeExpression {
    /// Trait and lifetime predicates in declaration order.
    pub(crate) bounds: Box<[PredicateDescriptor]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl OpaqueTypeExpression {
    /// Creates an opaque type expression.
    pub fn new(bounds: impl Into<Box<[PredicateDescriptor]>>) -> Self {
        Self {
            bounds: bounds.into(),
            diagnostic: DiagnosticText::default(),
        }
    }
    /// Returns opaque bounds.
    pub fn bounds(&self) -> &[PredicateDescriptor] {
        &self.bounds
    }
    /// Returns diagnostic text when present.
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Attaches diagnostic text.
    #[must_use]
    pub fn with_diagnostic(mut self, value: impl Into<Box<str>>) -> Self {
        self.diagnostic = DiagnosticText::from(value.into());
        self
    }
}

impl_identity_without_diagnostic!(OpaqueTypeExpression { bounds });
