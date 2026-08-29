//! `syn`-independent type and path representation.

use proc_macro2::{Span, TokenStream};

/// A Rust path retained with both structured segments and source tokens.
#[derive(Clone, Debug)]
pub(crate) struct PathIr {
    /// A normalized diagnostic rendering of the path.
    pub(crate) source: String,
    /// Identifier segments in source order.
    pub(crate) segments: Vec<String>,
    /// Original path tokens with their source spans.
    pub(crate) tokens: TokenStream,
    /// The complete source span of the path.
    pub(crate) span: Span,
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
    BareFunction,
    TraitObject,
    ImplTrait,
    Never,
    Infer,
    Macro,
    Other,
}
