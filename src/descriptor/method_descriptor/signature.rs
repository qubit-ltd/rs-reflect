// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Method parameter, receiver, return, visibility, and qualifier facts.

use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeDescriptorResolver;
use crate::expression::FunctionAbi;
use crate::expression::TypeExpression;
use crate::identity::Visibility;

/// How a non-receiver parameter is passed to a reflected method.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::ParameterPassingMode;
/// assert_eq!(ParameterPassingMode::SharedBorrow, ParameterPassingMode::SharedBorrow);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ParameterPassingMode {
    /// The method consumes an owned argument.
    Owned,
    /// The method borrows an argument immutably.
    SharedBorrow,
    /// The method borrows an argument mutably.
    MutableBorrow,
}

/// The source pattern category of a non-receiver parameter.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::ParameterPatternDescriptor;
/// let pattern = ParameterPatternDescriptor::Identifier;
/// assert!(matches!(pattern, ParameterPatternDescriptor::Identifier));
/// ```
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ParameterPatternDescriptor {
    /// A simple identifier that can participate in named binding.
    Identifier,
    /// A wildcard pattern without a bindable name.
    Wildcard,
    /// A destructuring pattern retained for positional binding and diagnostics.
    Destructure(Box<str>),
}

/// One non-receiver method parameter in declaration order.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::{ParameterDescriptor, ParameterPassingMode, ParameterPatternDescriptor};
/// use qubit_reflect::expression::{ConcreteTypeExpression, TypeExpression};
///
/// let parameter = ParameterDescriptor::new(
///     0,
///     Some("value"),
///     ParameterPatternDescriptor::Identifier,
///     ParameterPassingMode::Owned,
///     TypeExpression::Concrete(
///         ConcreteTypeExpression::new(["u8"], []).expect("non-empty path"),
///     ),
///     None,
/// );
/// assert_eq!(parameter.name(), Some("value"));
/// ```
#[derive(Clone, Debug)]
pub struct ParameterDescriptor {
    /// Zero-based position among non-receiver parameters.
    index: usize,
    /// Bindable identifier, absent for wildcard and destructuring patterns.
    name: Option<&'static str>,
    /// Source pattern category retained independently of parser syntax.
    pattern: ParameterPatternDescriptor,
    /// Ownership or borrowing mode at the method boundary.
    passing_mode: ParameterPassingMode,
    /// Declared type expression, which may still be symbolic.
    pub(super) signature_type: TypeExpression,
    /// Resolver for an exact reflected root, when available.
    concrete_type: Option<TypeDescriptorResolver>,
}

impl ParameterDescriptor {
    /// Creates immutable parameter facts.
    ///
    /// `index` excludes the receiver. `name` must be `None` for wildcard and
    /// destructuring patterns. `concrete_type` is present only when the
    /// declaration can navigate to an exact reflected root.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based non-receiver parameter position.
    /// - `name`: Bindable identifier, or `None` for wildcard/destructuring.
    /// - `pattern`: Source pattern category.
    /// - `passing_mode`: Whether the method owns or borrows the argument.
    /// - `signature_type`: Declared type expression.
    /// - `concrete_type`: Exact type resolver, when available.
    ///
    /// # Returns
    ///
    /// Returns immutable parameter facts for the declaration.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        index: usize,
        name: Option<&'static str>,
        pattern: ParameterPatternDescriptor,
        passing_mode: ParameterPassingMode,
        signature_type: TypeExpression,
        concrete_type: Option<TypeDescriptorResolver>,
    ) -> Self {
        Self {
            index,
            name,
            pattern,
            passing_mode,
            signature_type,
            concrete_type,
        }
    }

    /// Returns the zero-based non-receiver parameter index.
    ///
    /// # Returns
    ///
    /// Returns the parameter's position after excluding the receiver.
    #[must_use]
    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the identifier used for named binding.
    ///
    /// `None` denotes a wildcard or destructuring pattern.
    ///
    /// # Returns
    ///
    /// Returns the bindable identifier, or `None` when the pattern has none.
    #[must_use]
    #[inline]
    pub const fn name(&self) -> Option<&'static str> {
        self.name
    }

    /// Returns the parser-independent source pattern category.
    ///
    /// # Returns
    ///
    /// Returns the parameter's source pattern classification.
    #[must_use]
    #[inline]
    pub const fn pattern(&self) -> &ParameterPatternDescriptor {
        &self.pattern
    }

    /// Returns how the argument crosses the method boundary.
    ///
    /// # Returns
    ///
    /// Returns whether the argument is owned, shared-borrowed, or mutably
    /// borrowed.
    #[must_use]
    #[inline]
    pub const fn passing_mode(&self) -> ParameterPassingMode {
        self.passing_mode
    }

    /// Returns the declared, possibly symbolic parameter type.
    ///
    /// # Returns
    ///
    /// Returns the structural type expression from the signature.
    #[must_use]
    #[inline]
    pub const fn signature_type(&self) -> &TypeExpression {
        &self.signature_type
    }

    /// Returns the exact reflected parameter type when it is known.
    ///
    /// `None` denotes a symbolic, opaque, or otherwise unresolved type.
    ///
    /// # Returns
    ///
    /// Returns the exact reflected type root, or `None` when unavailable.
    #[must_use]
    pub fn concrete_type(&self) -> Option<&'static TypeDescriptor> {
        self.concrete_type.map(|resolver| resolver())
    }
}

/// The receiver form written by a reflected method declaration.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::ReceiverDescriptor;
/// assert!(matches!(ReceiverDescriptor::Shared, ReceiverDescriptor::Shared));
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReceiverDescriptor {
    /// A by-value `self` receiver.
    Owned,
    /// A shared `&self` receiver.
    Shared,
    /// An exclusive `&mut self` receiver.
    Mutable,
    /// A supported explicit receiver whose source form is retained.
    Explicit(&'static str),
}

/// The structural category of a method return value.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::ReturnKind;
/// assert_eq!(ReturnKind::Unit, ReturnKind::Unit);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReturnKind {
    /// The unit return type `()`.
    Unit,
    /// The never return type `!`.
    Never,
    /// A concrete owned value.
    Concrete,
    /// A shared or mutable reference.
    Reference,
    /// An opaque `impl Trait` return value.
    Opaque,
}

/// The return declaration of a reflected method.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::{ReturnDescriptor, ReturnKind};
/// let output = ReturnDescriptor::new(ReturnKind::Unit, None, None);
/// assert_eq!(output.kind(), ReturnKind::Unit);
/// ```
#[derive(Clone, Debug)]
pub struct ReturnDescriptor {
    /// Structural category of the return value.
    kind: ReturnKind,
    /// Declared return type expression, when one is needed.
    pub(super) signature_type: Option<TypeExpression>,
    /// Resolver for an exact reflected root, when available.
    concrete_type: Option<TypeDescriptorResolver>,
}

impl ReturnDescriptor {
    /// Creates immutable return facts.
    ///
    /// `signature_type` is absent for unit and never returns when their
    /// [`ReturnKind`] is sufficient. `concrete_type` is present only for an
    /// exact reflected root.
    ///
    /// # Parameters
    ///
    /// - `kind`: Structural return category.
    /// - `signature_type`: Declared type expression, when applicable.
    /// - `concrete_type`: Exact reflected type resolver, when available.
    ///
    /// # Returns
    ///
    /// Returns immutable facts for the declared return value.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        kind: ReturnKind,
        signature_type: Option<TypeExpression>,
        concrete_type: Option<TypeDescriptorResolver>,
    ) -> Self {
        Self {
            kind,
            signature_type,
            concrete_type,
        }
    }

    /// Creates a unit return descriptor.
    ///
    /// # Returns
    ///
    /// Returns a descriptor representing `()`.
    #[must_use]
    pub const fn unit() -> Self {
        Self::new(ReturnKind::Unit, None, None)
    }

    /// Returns the structural return category.
    ///
    /// # Returns
    ///
    /// Returns the return category.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> ReturnKind {
        self.kind
    }

    /// Returns the declared return type expression.
    ///
    /// `None` means the unit or never category carries the complete fact.
    ///
    /// # Returns
    ///
    /// Returns the declared type expression, or `None` for unit/never.
    #[must_use]
    #[inline]
    pub const fn signature_type(&self) -> Option<&TypeExpression> {
        self.signature_type.as_ref()
    }

    /// Returns the exact reflected return type when it is known.
    ///
    /// `None` denotes unit, never, a reference, opaque output, or an unresolved
    /// symbolic type.
    ///
    /// # Returns
    ///
    /// Returns the exact reflected type root, or `None` when unavailable.
    #[must_use]
    pub fn concrete_type(&self) -> Option<&'static TypeDescriptor> {
        self.concrete_type.map(|resolver| resolver())
    }
}

/// Where a method declaration obtains its source visibility.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::MethodVisibility;
/// use qubit_reflect::identity::Visibility;
/// let visibility = MethodVisibility::Declared(Visibility::Public);
/// assert!(matches!(visibility, MethodVisibility::Declared(Visibility::Public)));
/// ```
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum MethodVisibility {
    /// Visibility declared by an inherent or implementation method.
    Declared(Visibility),
    /// Trait-item reachability inherited from the declaring trait.
    InheritedFromTrait,
}

/// Qualifiers that affect whether a declaration can have an invocation adapter.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::MethodQualifiers;
/// let qualifiers = MethodQualifiers::new(false, false, false, None, false);
/// assert!(!qualifiers.is_async());
/// ```
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MethodQualifiers {
    /// Whether the declaration is `async`.
    pub(crate) is_async: bool,
    /// Whether the declaration is `unsafe`.
    pub(crate) is_unsafe: bool,
    /// Whether the declaration is `const`.
    pub(crate) is_const: bool,
    /// The explicitly declared ABI, or `None` for the ordinary Rust ABI.
    pub(crate) abi: Option<FunctionAbi>,
    /// Whether the declaration has a variadic tail.
    pub(crate) is_variadic: bool,
}

impl MethodQualifiers {
    /// Creates the complete set of method qualifiers.
    ///
    /// # Parameters
    ///
    /// - `is_async`: Whether the declaration is asynchronous.
    /// - `is_unsafe`: Whether the declaration is unsafe.
    /// - `is_const`: Whether the declaration is const.
    /// - `abi`: Explicit ABI, or `None` for the Rust ABI.
    /// - `is_variadic`: Whether the declaration has a variadic tail.
    ///
    /// # Returns
    ///
    /// Returns the supplied method qualifiers.
    #[must_use]
    pub const fn new(
        is_async: bool,
        is_unsafe: bool,
        is_const: bool,
        abi: Option<FunctionAbi>,
        is_variadic: bool,
    ) -> Self {
        Self {
            is_async,
            is_unsafe,
            is_const,
            abi,
            is_variadic,
        }
    }

    /// Returns whether the declaration is asynchronous.
    ///
    /// # Returns
    ///
    /// Returns `true` for an `async` method.
    #[must_use]
    pub const fn is_async(&self) -> bool {
        self.is_async
    }

    /// Returns whether the declaration is unsafe.
    ///
    /// # Returns
    ///
    /// Returns `true` for an `unsafe` method.
    #[must_use]
    pub const fn is_unsafe(&self) -> bool {
        self.is_unsafe
    }

    /// Returns whether the declaration is const.
    ///
    /// # Returns
    ///
    /// Returns `true` for a `const` method.
    #[must_use]
    pub const fn is_const(&self) -> bool {
        self.is_const
    }

    /// Returns the explicitly declared ABI.
    ///
    /// # Returns
    ///
    /// Returns the ABI, or `None` for the ordinary Rust ABI.
    #[must_use]
    pub const fn abi(&self) -> Option<&FunctionAbi> {
        self.abi.as_ref()
    }

    /// Returns whether the declaration has a variadic tail.
    ///
    /// # Returns
    ///
    /// Returns `true` when the method has a variadic tail.
    #[must_use]
    pub const fn is_variadic(&self) -> bool {
        self.is_variadic
    }
}

impl Default for MethodQualifiers {
    /// Returns the qualifiers of an ordinary safe Rust method.
    ///
    /// # Returns
    ///
    /// Returns default qualifiers with no async, unsafe, const, ABI, or
    /// variadic modifier.
    fn default() -> Self {
        Self::new(false, false, false, None, false)
    }
}
