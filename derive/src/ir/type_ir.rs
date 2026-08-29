//! `syn`-independent type and path representation.

use proc_macro2::{Span, TokenStream};

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
    pub(crate) name: String,
    pub(crate) arguments: PathArgumentsIr,
}

/// The syntactic form and values of one path segment's arguments.
#[derive(Clone, Debug)]
pub(crate) enum PathArgumentsIr {
    None,
    AngleBracketed(Vec<PathArgumentIr>),
    Parenthesized {
        inputs: Vec<TypeIr>,
        output: Option<Box<TypeIr>>,
    },
}

/// A generic argument attached to a path segment.
#[derive(Clone, Debug)]
pub(crate) enum PathArgumentIr {
    Lifetime(String),
    Type(TypeIr),
    Const(TokenStream),
    AssociatedType {
        name: String,
        ty: TypeIr,
    },
    AssociatedConst {
        name: String,
        value: TokenStream,
    },
    Constraint {
        name: String,
        bounds: Vec<GenericBoundIr>,
    },
    Other(TokenStream),
}

/// The semantic facts of a qualified-self path prefix.
#[derive(Clone, Debug)]
pub(crate) struct QualifiedSelfIr {
    pub(crate) ty: Box<TypeIr>,
    pub(crate) position: usize,
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
    Path(PathIr),
    Reference {
        lifetime: Option<String>,
        mutable: bool,
        element: Box<TypeIr>,
    },
    Tuple(Vec<TypeIr>),
    Slice(Box<TypeIr>),
    Array {
        element: Box<TypeIr>,
        length: TokenStream,
    },
    Pointer {
        mutable: bool,
        element: Box<TypeIr>,
    },
    BareFunction {
        lifetimes: Vec<String>,
        inputs: Vec<TypeIr>,
        output: Option<Box<TypeIr>>,
        is_unsafe: bool,
        abi: Option<String>,
        is_variadic: bool,
    },
    TraitObject {
        bounds: Vec<GenericBoundIr>,
        has_dyn: bool,
    },
    ImplTrait {
        bounds: Vec<GenericBoundIr>,
    },
    Never,
    Infer,
    Macro,
    Other,
}
