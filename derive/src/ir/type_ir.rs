// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! `syn`-independent type and path representation.

// qubit-style: allow multiple-public-types

use proc_macro2::Span;
use proc_macro2::TokenStream;

use crate::ir::GenericBoundIr;

/// A Rust path retained with both structured segments and source tokens.
#[derive(Clone, Debug)]
pub(crate) struct PathIr {
    /// A normalized diagnostic rendering of the path.
    pub(crate) source: String,
    /// Identifier segments in source order.
    pub(crate) segments: Vec<PathSegmentIr>,
    /// Whether the source path starts at the crate root.
    pub(crate) leading_colon: bool,
    /// A qualified-self prefix such as `<T as Trait>`.
    pub(crate) qualified_self: Option<QualifiedSelfIr>,
    /// Original path tokens with their source spans.
    pub(crate) tokens: TokenStream,
    /// The complete source span of the path.
    pub(crate) span: Span,
}

/// One path segment and all of its generic arguments.
#[derive(Clone, Debug)]
pub(crate) struct PathSegmentIr {
    /// Segment identifier without generic arguments.
    pub(crate) name: String,
    /// Generic arguments applied to this segment.
    pub(crate) arguments: PathArgumentsIr,
}

/// The syntactic form and values of one path segment's arguments.
#[derive(Clone, Debug)]
pub(crate) enum PathArgumentsIr {
    /// No arguments follow the path segment.
    None,
    /// Angle-bracketed lifetime, type, const, or associated arguments.
    AngleBracketed(Vec<PathArgumentIr>),
    /// Parenthesized function-trait arguments and optional output type.
    Parenthesized {
        /// Input types in declaration order.
        inputs: Vec<TypeIr>,
        /// Optional function-trait output type.
        output: Option<Box<TypeIr>>,
    },
}

/// A generic argument attached to a path segment.
#[derive(Clone, Debug)]
pub(crate) enum PathArgumentIr {
    /// Lifetime argument, including its leading apostrophe.
    Lifetime(String),
    /// Nested type argument.
    Type(TypeIr),
    /// Const expression tokens with source spans.
    Const(TokenStream),
    /// Associated type equality binding.
    AssociatedType { name: String, ty: TypeIr },
    /// Associated const equality binding.
    AssociatedConst { name: String, value: TokenStream },
    /// Associated type bounds constraint.
    Constraint {
        name: String,
        bounds: Vec<GenericBoundIr>,
    },
    /// Argument syntax not represented by the known structured forms.
    Other(TokenStream),
}

/// The semantic facts of a qualified-self path prefix.
#[derive(Clone, Debug)]
pub(crate) struct QualifiedSelfIr {
    /// Type before the `as` clause or closing angle bracket.
    pub(crate) ty: Box<TypeIr>,
    /// Segment position where the qualified path resumes.
    pub(crate) position: usize,
    /// Whether the source explicitly contains an `as` clause.
    pub(crate) has_as: bool,
}

/// A Rust type converted from `syn::Type` at the parser boundary.
#[derive(Clone, Debug)]
pub(crate) struct TypeIr {
    /// A normalized diagnostic rendering of the type.
    pub(crate) source: String,
    /// Original type tokens with their source spans.
    pub(crate) tokens: TokenStream,
    /// The structural type category used by expansion.
    pub(crate) kind: TypeKindIr,
    /// The complete source span of the type.
    pub(crate) span: Span,
}

/// The structural category of a parsed Rust type.
#[derive(Clone, Debug)]
pub(crate) enum TypeKindIr {
    /// A path type, including qualified paths.
    Path(PathIr),
    /// A reference and its mutability, lifetime, and pointee.
    Reference {
        /// Explicit lifetime, when written.
        lifetime: Option<String>,
        /// Whether the reference is mutable.
        mutable: bool,
        /// Referenced type.
        element: Box<TypeIr>,
    },
    /// Tuple elements in source order.
    Tuple(Vec<TypeIr>),
    /// Slice element type.
    Slice(Box<TypeIr>),
    /// Array element type and unevaluated length expression.
    Array {
        /// Array element type.
        element: Box<TypeIr>,
        /// Array length expression tokens.
        length: TokenStream,
    },
    /// Raw pointer mutability and pointee type.
    Pointer {
        /// Whether the pointer is mutable.
        mutable: bool,
        /// Pointed-to type.
        element: Box<TypeIr>,
    },
    /// Function pointer signature facts.
    BareFunction {
        /// Higher-ranked bound lifetimes.
        lifetimes: Vec<String>,
        /// Input parameter types in declaration order.
        inputs: Vec<TypeIr>,
        /// Optional return type; `None` represents unit.
        output: Option<Box<TypeIr>>,
        /// Whether the function pointer is unsafe.
        is_unsafe: bool,
        /// ABI name, when explicitly declared.
        abi: Option<String>,
        /// Whether the signature is variadic.
        is_variadic: bool,
    },
    /// Trait object bounds and whether `dyn` was explicit.
    TraitObject {
        /// Bounds retained from the trait object.
        bounds: Vec<GenericBoundIr>,
        /// Whether the source used the `dyn` keyword.
        has_dyn: bool,
    },
    /// Opaque `impl Trait` bounds.
    ImplTrait {
        /// Bounds retained from the opaque type.
        bounds: Vec<GenericBoundIr>,
    },
    /// Never type `!`.
    Never,
    /// Inferred type `_`.
    Infer,
    /// Type macro invocation.
    Macro,
    /// Type syntax not covered by the structured variants.
    Other,
}
