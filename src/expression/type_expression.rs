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
use crate::expression::ExpressionName;
use crate::expression::GenericArgument;
use crate::expression::LifetimeExpression;
use crate::expression::PredicateDescriptor;

/// Implements structural equality and hashing that ignore diagnostic text.
///
/// # Parameters
///
/// The macro accepts a type and the fields that define its structural identity.
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
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::DiagnosticText;
/// let text = DiagnosticText::from("source spelling");
/// assert_eq!(text.as_deref(), Some("source spelling"));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DiagnosticText(
    /// Optional source-oriented text retained for diagnostic output.
    pub(crate) Option<Box<str>>,
);

impl DiagnosticText {
    /// Returns the diagnostic text when present.
    ///
    /// # Returns
    ///
    /// Returns the retained text, or `None` when no diagnostic was attached.
    #[must_use]
    pub fn as_deref(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

impl From<Box<str>> for DiagnosticText {
    /// Wraps owned diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned text retained for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns diagnostic text containing `value`.
    fn from(value: Box<str>) -> Self {
        Self(Some(value))
    }
}

impl From<&str> for DiagnosticText {
    /// Copies borrowed text into diagnostic storage.
    ///
    /// # Parameters
    ///
    /// - `value`: Text retained for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns diagnostic text containing a copy of `value`.
    fn from(value: &str) -> Self {
        Self(Some(value.into()))
    }
}

impl From<String> for DiagnosticText {
    /// Moves owned text into diagnostic storage.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned text retained for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns diagnostic text containing `value`.
    fn from(value: String) -> Self {
        Self(Some(value.into_boxed_str()))
    }
}

/// A closed, navigable Rust type expression independent of parser
/// implementation types.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::TypeExpression;
/// let ty = TypeExpression::parameter("Item").expect("valid parameter");
/// assert!(matches!(ty, TypeExpression::Parameter(_)));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeExpression {
    /// A concrete path such as `std::vec::Vec<T>`.
    Concrete(ConcreteTypeExpression),
    /// A type parameter such as `T`.
    Parameter(ExpressionName),
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

impl TypeExpression {
    /// Creates a named type-parameter expression.
    ///
    /// # Parameters
    ///
    /// - `name`: Type parameter identifier.
    ///
    /// # Returns
    ///
    /// Returns the parameter expression.
    ///
    /// # Errors
    ///
    /// Returns [`ExpressionError::EmptyName`] when `name` is empty.
    pub fn parameter(name: impl Into<Box<str>>) -> Result<Self, ExpressionError> {
        ExpressionName::new(name).map(Self::Parameter)
    }
}

/// A concrete type path and its final-segment generic arguments.
#[derive(Clone, Debug)]
pub struct ConcreteTypeExpression {
    /// Path segments in declaration order, for example `std`, `vec`, and
    /// `Vec`.
    pub(crate) path: Box<[Box<str>]>,
    /// Generic arguments of the final path segment in declaration order.
    pub(crate) arguments: Box<[GenericArgument]>,
    /// Structural path segments with arguments retained at their source site.
    pub(crate) segments: Box<[ConcretePathSegment]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl ConcreteTypeExpression {
    /// Creates a concrete type expression from a non-empty path.
    ///
    /// # Type Parameters
    ///
    /// - `P`: Iterable collection of path segments.
    /// - `S`: Segment value convertible to owned text.
    ///
    /// # Parameters
    ///
    /// - `path`: Non-empty concrete path in declaration order.
    /// - `arguments`: Generic arguments applied to the final path segment.
    ///
    /// # Returns
    ///
    /// Returns the validated concrete type expression.
    ///
    /// # Errors
    ///
    /// Returns [`ExpressionError::EmptyConcretePath`] for an empty path or
    /// [`ExpressionError::EmptyPathSegment`] when a path segment is empty.
    pub fn new<P, S>(path: P, arguments: impl IntoIterator<Item = GenericArgument>) -> Result<Self, ExpressionError>
    where
        P: IntoIterator<Item = S>,
        S: Into<Box<str>>,
    {
        let path = path.into_iter().map(Into::into).collect::<Box<[_]>>();
        if path.is_empty() {
            return Err(ExpressionError::EmptyConcretePath);
        }
        if let Some(index) = path.iter().position(|segment| segment.is_empty()) {
            return Err(ExpressionError::EmptyPathSegment { index });
        }
        let arguments = arguments.into_iter().collect::<Box<[_]>>();
        let mut segments = path
            .iter()
            .map(|name| ConcretePathSegment::new(name.clone(), Box::default()))
            .collect::<Vec<_>>();
        if let Some(last) = segments.last_mut() {
            last.arguments = arguments.clone();
        }
        Ok(Self {
            path,
            arguments,
            segments: segments.into_boxed_slice(),
            diagnostic: DiagnosticText::default(),
        })
    }

    /// Creates a concrete type expression from structural path segments.
    ///
    /// # Parameters
    ///
    /// - `segments`: Non-empty path segments with arguments attached at their
    ///   source positions.
    ///
    /// # Returns
    ///
    /// Returns the validated concrete type expression.
    ///
    /// # Errors
    ///
    /// Returns [`ExpressionError::EmptyConcretePath`] for no segments or
    /// [`ExpressionError::EmptyPathSegment`] when a segment name is empty.
    pub fn from_segments(segments: impl IntoIterator<Item = ConcretePathSegment>) -> Result<Self, ExpressionError> {
        let segments = segments.into_iter().collect::<Box<[_]>>();
        if segments.is_empty() {
            return Err(ExpressionError::EmptyConcretePath);
        }
        if let Some(index) = segments.iter().position(|segment| segment.name.is_empty()) {
            return Err(ExpressionError::EmptyPathSegment { index });
        }
        let path = segments.iter().map(|segment| segment.name.clone()).collect();
        let arguments = segments
            .last()
            .map_or_else(Box::default, |segment| segment.arguments.clone());
        Ok(Self {
            path,
            arguments,
            segments,
            diagnostic: DiagnosticText::default(),
        })
    }

    /// Returns the path segments in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the path names without their segment-local arguments.
    #[must_use]
    pub fn path(&self) -> &[Box<str>] {
        &self.path
    }

    /// Returns final-segment generic arguments in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the generic arguments on the final path segment.
    #[must_use]
    #[inline]
    pub fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Returns structural path segments with their local generic arguments.
    ///
    /// # Returns
    ///
    /// Returns every path segment with its own arguments retained.
    #[must_use]
    pub fn segments(&self) -> &[ConcretePathSegment] {
        &self.segments
    }

    /// Returns source-oriented diagnostic text when present.
    ///
    /// # Returns
    ///
    /// Returns source-oriented text, or `None` when absent.
    #[must_use]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }

    /// Attaches source-oriented diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `diagnostic`: Source-oriented rendering for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the expression with diagnostic text attached; structural
    /// identity is unchanged.
    #[must_use]
    pub fn with_diagnostic(mut self, diagnostic: impl Into<Box<str>>) -> Self {
        self.diagnostic = DiagnosticText::from(diagnostic.into());
        self
    }
}

impl_identity_without_diagnostic!(ConcreteTypeExpression { segments });

/// One concrete path segment and the generic arguments written on it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConcretePathSegment {
    /// Identifier for this path component.
    name: Box<str>,
    /// Generic arguments written on this exact component.
    arguments: Box<[GenericArgument]>,
}

impl ConcretePathSegment {
    /// Creates a structural concrete path segment.
    ///
    /// # Parameters
    ///
    /// - `name`: Path segment text; containing paths validate that it is
    ///   non-empty.
    /// - `arguments`: Generic arguments written on this segment.
    ///
    /// # Returns
    ///
    /// Returns the structural path segment.
    pub fn new(name: impl Into<Box<str>>, arguments: impl Into<Box<[GenericArgument]>>) -> Self {
        Self {
            name: name.into(),
            arguments: arguments.into(),
        }
    }

    /// Returns this path segment's identifier.
    ///
    /// # Returns
    ///
    /// Returns the segment name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns generic arguments attached to this exact segment.
    ///
    /// # Returns
    ///
    /// Returns the arguments retained at this segment.
    #[must_use]
    pub fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }
}

/// An associated type projection.
#[derive(Clone, Debug)]
pub struct AssociatedTypeExpression {
    /// The self type whose associated item is projected.
    pub(crate) self_type: Box<TypeExpression>,
    /// The optional qualifying trait path from an `as Trait` clause.
    pub(crate) trait_path: Option<Box<TypeExpression>>,
    /// The associated item name.
    pub(crate) item: ExpressionName,
    /// Generic arguments applied to the associated type.
    pub(crate) arguments: Box<[GenericArgument]>,
    /// Optional source-oriented diagnostic text excluded from identity.
    pub(crate) diagnostic: DiagnosticText,
}

impl AssociatedTypeExpression {
    /// Creates an associated type projection.
    ///
    /// # Parameters
    ///
    /// - `self_type`: Type on which the associated type is projected.
    /// - `trait_path`: Optional trait qualification after `as`.
    /// - `item`: Associated type name.
    /// - `arguments`: Generic arguments applied to the associated type.
    ///
    /// # Returns
    ///
    /// Returns the projection with empty diagnostic text.
    pub fn new(
        self_type: TypeExpression,
        trait_path: Option<TypeExpression>,
        item: impl Into<ExpressionName>,
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
    ///
    /// # Returns
    ///
    /// Returns the type on which the associated type is projected.
    #[must_use]
    pub fn self_type(&self) -> &TypeExpression {
        &self.self_type
    }

    /// Returns the optional qualifying trait path.
    ///
    /// # Returns
    ///
    /// Returns the trait qualification, or `None` for an unqualified
    /// projection.
    #[must_use]
    pub fn trait_path(&self) -> Option<&TypeExpression> {
        self.trait_path.as_deref()
    }

    /// Returns the associated item name.
    ///
    /// # Returns
    ///
    /// Returns the associated type identifier.
    #[must_use]
    pub fn item(&self) -> &str {
        self.item.as_str()
    }

    /// Returns associated type arguments.
    ///
    /// # Returns
    ///
    /// Returns generic arguments applied to the projection.
    #[must_use]
    #[inline]
    pub fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Returns source-oriented diagnostic text when present.
    ///
    /// # Returns
    ///
    /// Returns diagnostic text, or `None` when absent.
    #[must_use]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }

    /// Attaches source-oriented diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `value`: Source-oriented projection spelling used for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the projection with diagnostic text attached; structural
    /// identity is unchanged.
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
    ///
    /// # Parameters
    ///
    /// - `lifetime`: Source lifetime or elision category.
    /// - `mutable`: Whether the reference is mutable.
    /// - `target`: Referenced type.
    ///
    /// # Returns
    ///
    /// Returns the reference expression with empty diagnostic text.
    #[must_use]
    pub fn new(lifetime: LifetimeExpression, mutable: bool, target: TypeExpression) -> Self {
        Self {
            lifetime,
            mutable,
            target: Box::new(target),
            diagnostic: DiagnosticText::default(),
        }
    }
    /// Returns the reference lifetime.
    ///
    /// # Returns
    ///
    /// Returns the named, static, elided, or placeholder lifetime.
    #[must_use]
    pub fn lifetime(&self) -> &LifetimeExpression {
        &self.lifetime
    }
    /// Returns whether the reference is mutable.
    ///
    /// # Returns
    ///
    /// Returns `true` for `&mut` and `false` for a shared reference.
    #[must_use]
    pub fn is_mutable(&self) -> bool {
        self.mutable
    }
    /// Returns the referenced type.
    ///
    /// # Returns
    ///
    /// Returns the target type expression.
    #[must_use]
    pub fn target(&self) -> &TypeExpression {
        &self.target
    }
    /// Returns diagnostic text when present.
    ///
    /// # Returns
    ///
    /// Returns the source-oriented spelling, or `None` when absent.
    #[must_use]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Attaches diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `value`: Source-oriented reference spelling used for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the expression with diagnostic text attached; structural
    /// identity is unchanged.
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
    ///
    /// # Parameters
    ///
    /// - `mutable`: Whether the pointer is mutable.
    /// - `target`: Pointee type expression.
    ///
    /// # Returns
    ///
    /// Returns the raw pointer expression with empty diagnostic text.
    #[must_use]
    pub fn new(mutable: bool, target: TypeExpression) -> Self {
        Self {
            mutable,
            target: Box::new(target),
            diagnostic: DiagnosticText::default(),
        }
    }
    /// Returns whether the pointer is mutable.
    ///
    /// # Returns
    ///
    /// Returns `true` for `*mut` and `false` for `*const`.
    #[must_use]
    pub fn is_mutable(&self) -> bool {
        self.mutable
    }
    /// Returns the pointee type.
    ///
    /// # Returns
    ///
    /// Returns the pointed-to type expression.
    #[must_use]
    pub fn target(&self) -> &TypeExpression {
        &self.target
    }
    /// Returns diagnostic text when present.
    ///
    /// # Returns
    ///
    /// Returns the source-oriented spelling, or `None` when absent.
    #[must_use]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Attaches diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `value`: Source-oriented pointer spelling used for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the expression with diagnostic text attached; structural
    /// identity is unchanged.
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
    ///
    /// # Parameters
    ///
    /// - `element`: Type repeated in each array element.
    /// - `length`: Structural const expression for the array length.
    ///
    /// # Returns
    ///
    /// Returns the array expression with empty diagnostic text.
    #[must_use]
    pub fn new(element: TypeExpression, length: ConstExpression) -> Self {
        Self {
            element: Box::new(element),
            length,
            diagnostic: DiagnosticText::default(),
        }
    }
    /// Returns the element type.
    ///
    /// # Returns
    ///
    /// Returns the repeated element type.
    #[must_use]
    pub fn element(&self) -> &TypeExpression {
        &self.element
    }
    /// Returns the structural length expression.
    ///
    /// # Returns
    ///
    /// Returns the array length expression.
    #[must_use]
    pub fn length(&self) -> &ConstExpression {
        &self.length
    }
    /// Returns diagnostic text when present.
    ///
    /// # Returns
    ///
    /// Returns the source-oriented spelling, or `None` when absent.
    #[must_use]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Attaches diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `value`: Source-oriented array spelling used for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the expression with diagnostic text attached; structural
    /// identity is unchanged.
    #[must_use]
    pub fn with_diagnostic(mut self, value: impl Into<Box<str>>) -> Self {
        self.diagnostic = DiagnosticText::from(value.into());
        self
    }
}

impl_identity_without_diagnostic!(ArrayTypeExpression { element, length });

/// A function pointer's calling convention.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::FunctionAbi;
/// let abi = FunctionAbi::C;
/// assert_eq!(abi, FunctionAbi::C);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FunctionAbi {
    /// The default Rust ABI.
    Rust,
    /// The C ABI.
    C,
    /// The platform system ABI.
    System,
    /// Any explicitly named ABI not covered by a standard variant.
    Other(ExpressionName),
}

/// A function pointer's safety qualifier.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::FunctionSafety;
/// let safety = FunctionSafety::Unsafe;
/// assert_eq!(safety, FunctionSafety::Unsafe);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FunctionSafety {
    /// A safe function pointer.
    Safe,
    /// An `unsafe fn` pointer.
    Unsafe,
}

/// A function pointer signature.
///
/// # Examples
///
/// ```
/// use qubit_reflect::expression::FunctionAbi;
/// use qubit_reflect::expression::FunctionPointerExpression;
/// use qubit_reflect::expression::FunctionSafety;
/// use qubit_reflect::expression::TypeExpression;
/// let signature = FunctionPointerExpression::new(
///     FunctionAbi::Rust,
///     FunctionSafety::Safe,
///     false,
///     [],
///     [],
///     TypeExpression::Never,
/// );
/// assert!(signature.parameters().is_empty());
/// ```
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
    ///
    /// # Parameters
    ///
    /// - `abi`: Calling convention.
    /// - `safety`: Safe or unsafe function pointer qualifier.
    /// - `variadic`: Whether the final parameter is variadic.
    /// - `higher_ranked_lifetimes`: Lifetimes introduced by a `for<...>`
    ///   binder.
    /// - `parameters`: Function parameter types in declaration order.
    /// - `return_type`: Function return type.
    ///
    /// # Returns
    ///
    /// Returns the function pointer expression with empty diagnostic text.
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
    ///
    /// # Returns
    ///
    /// Returns the function ABI.
    #[must_use]
    pub fn abi(&self) -> &FunctionAbi {
        &self.abi
    }
    /// Returns the safety qualifier.
    ///
    /// # Returns
    ///
    /// Returns whether the function pointer is safe or unsafe.
    #[must_use]
    pub fn safety(&self) -> &FunctionSafety {
        &self.safety
    }
    /// Returns whether the signature is variadic.
    ///
    /// # Returns
    ///
    /// Returns `true` when the final parameter is variadic.
    #[must_use]
    pub fn is_variadic(&self) -> bool {
        self.variadic
    }
    /// Returns higher-ranked lifetimes.
    ///
    /// # Returns
    ///
    /// Returns lifetimes declared by the function pointer's binder.
    #[must_use]
    pub fn higher_ranked_lifetimes(&self) -> &[LifetimeExpression] {
        &self.higher_ranked_lifetimes
    }
    /// Returns parameter types.
    ///
    /// # Returns
    ///
    /// Returns function parameter types in declaration order.
    #[must_use]
    pub fn parameters(&self) -> &[TypeExpression] {
        &self.parameters
    }
    /// Returns the return type.
    ///
    /// # Returns
    ///
    /// Returns the function result type expression.
    #[must_use]
    pub fn return_type(&self) -> &TypeExpression {
        &self.return_type
    }
    /// Returns diagnostic text when present.
    ///
    /// # Returns
    ///
    /// Returns the source-oriented spelling, or `None` when absent.
    #[must_use]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Attaches diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `value`: Source-oriented signature used for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the expression with diagnostic text attached; structural
    /// identity is unchanged.
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
    ///
    /// # Parameters
    ///
    /// - `bounds`: Trait and lifetime predicates required of the object.
    ///
    /// # Returns
    ///
    /// Returns a trait object expression with empty diagnostic text.
    pub fn new(bounds: impl Into<Box<[PredicateDescriptor]>>) -> Self {
        Self {
            bounds: bounds.into(),
            diagnostic: DiagnosticText::default(),
        }
    }
    /// Returns object bounds.
    ///
    /// # Returns
    ///
    /// Returns trait and lifetime predicates in declaration order.
    #[must_use]
    pub fn bounds(&self) -> &[PredicateDescriptor] {
        &self.bounds
    }
    /// Returns diagnostic text when present.
    ///
    /// # Returns
    ///
    /// Returns the source-oriented spelling, or `None` when absent.
    #[must_use]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Attaches diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `value`: Source-oriented trait-object spelling used for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the expression with diagnostic text attached; structural
    /// identity is unchanged.
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
    ///
    /// # Parameters
    ///
    /// - `bounds`: Trait and lifetime predicates constraining the opaque type.
    ///
    /// # Returns
    ///
    /// Returns an opaque type expression with empty diagnostic text.
    pub fn new(bounds: impl Into<Box<[PredicateDescriptor]>>) -> Self {
        Self {
            bounds: bounds.into(),
            diagnostic: DiagnosticText::default(),
        }
    }
    /// Returns opaque bounds.
    ///
    /// # Returns
    ///
    /// Returns trait and lifetime predicates in declaration order.
    #[must_use]
    pub fn bounds(&self) -> &[PredicateDescriptor] {
        &self.bounds
    }
    /// Returns diagnostic text when present.
    ///
    /// # Returns
    ///
    /// Returns the source-oriented spelling, or `None` when absent.
    #[must_use]
    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
    /// Attaches diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `value`: Source-oriented opaque type spelling used for diagnostics.
    ///
    /// # Returns
    ///
    /// Returns the expression with diagnostic text attached; structural
    /// identity is unchanged.
    #[must_use]
    pub fn with_diagnostic(mut self, value: impl Into<Box<str>>) -> Self {
        self.diagnostic = DiagnosticText::from(value.into());
        self
    }
}

impl_identity_without_diagnostic!(OpaqueTypeExpression { bounds });
