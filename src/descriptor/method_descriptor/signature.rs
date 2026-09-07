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
#[derive(Clone, Debug)]
pub struct ParameterDescriptor {
    index: usize,
    name: Option<&'static str>,
    pattern: ParameterPatternDescriptor,
    passing_mode: ParameterPassingMode,
    pub(super) signature_type: TypeExpression,
    concrete_type: Option<TypeDescriptorResolver>,
}

impl ParameterDescriptor {
    /// Creates immutable parameter facts.
    ///
    /// `index` excludes the receiver. `name` must be `None` for wildcard and
    /// destructuring patterns. `concrete_type` is present only when the
    /// declaration can navigate to an exact reflected root.
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
    #[must_use]
    #[inline(always)]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the identifier used for named binding.
    ///
    /// `None` denotes a wildcard or destructuring pattern.
    #[must_use]
    #[inline(always)]
    pub const fn name(&self) -> Option<&'static str> {
        self.name
    }

    /// Returns the parser-independent source pattern category.
    #[must_use]
    #[inline(always)]
    pub const fn pattern(&self) -> &ParameterPatternDescriptor {
        &self.pattern
    }

    /// Returns how the argument crosses the method boundary.
    #[must_use]
    #[inline(always)]
    pub const fn passing_mode(&self) -> ParameterPassingMode {
        self.passing_mode
    }

    /// Returns the declared, possibly symbolic parameter type.
    #[must_use]
    #[inline(always)]
    pub const fn signature_type(&self) -> &TypeExpression {
        &self.signature_type
    }

    /// Returns the exact reflected parameter type when it is known.
    ///
    /// `None` denotes a symbolic, opaque, or otherwise unresolved type.
    #[must_use]
    #[inline(always)]
    pub fn concrete_type(&self) -> Option<&'static TypeDescriptor> {
        self.concrete_type.map(|resolver| resolver())
    }
}

/// The receiver form written by a reflected method declaration.
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
#[derive(Clone, Debug)]
pub struct ReturnDescriptor {
    kind: ReturnKind,
    pub(super) signature_type: Option<TypeExpression>,
    concrete_type: Option<TypeDescriptorResolver>,
}

impl ReturnDescriptor {
    /// Creates immutable return facts.
    ///
    /// `signature_type` is absent for unit and never returns when their
    /// [`ReturnKind`] is sufficient. `concrete_type` is present only for an
    /// exact reflected root.
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
    #[must_use]
    pub const fn unit() -> Self {
        Self::new(ReturnKind::Unit, None, None)
    }

    /// Returns the structural return category.
    #[must_use]
    #[inline(always)]
    pub const fn kind(&self) -> ReturnKind {
        self.kind
    }

    /// Returns the declared return type expression.
    ///
    /// `None` means the unit or never category carries the complete fact.
    #[must_use]
    #[inline(always)]
    pub const fn signature_type(&self) -> Option<&TypeExpression> {
        self.signature_type.as_ref()
    }

    /// Returns the exact reflected return type when it is known.
    ///
    /// `None` denotes unit, never, a reference, opaque output, or an unresolved
    /// symbolic type.
    #[must_use]
    #[inline(always)]
    pub fn concrete_type(&self) -> Option<&'static TypeDescriptor> {
        self.concrete_type.map(|resolver| resolver())
    }
}

/// Where a method declaration obtains its source visibility.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum MethodVisibility {
    /// Visibility declared by an inherent or implementation method.
    Declared(Visibility),
    /// Trait-item reachability inherited from the declaring trait.
    InheritedFromTrait,
}

/// Qualifiers that affect whether a declaration can have an invocation adapter.
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
    #[must_use]
    pub const fn is_async(&self) -> bool {
        self.is_async
    }
    /// Returns whether the declaration is unsafe.
    #[must_use]
    pub const fn is_unsafe(&self) -> bool {
        self.is_unsafe
    }
    /// Returns whether the declaration is const.
    #[must_use]
    pub const fn is_const(&self) -> bool {
        self.is_const
    }
    /// Returns the explicitly declared ABI.
    #[must_use]
    pub const fn abi(&self) -> Option<&FunctionAbi> {
        self.abi.as_ref()
    }
    /// Returns whether the declaration has a variadic tail.
    #[must_use]
    pub const fn is_variadic(&self) -> bool {
        self.is_variadic
    }
}

impl Default for MethodQualifiers {
    /// Returns the qualifiers of an ordinary safe Rust method.
    fn default() -> Self {
        Self::new(false, false, false, None, false)
    }
}
