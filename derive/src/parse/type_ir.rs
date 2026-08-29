//! Conversion from `syn` types and paths into semantic IR.

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Path, Type};

use crate::ir::{PathIr, TypeIr, TypeKindIr};

/// Converts a Rust path while retaining source tokens and structured segments.
pub(super) fn convert_path(path: &Path) -> PathIr {
    let tokens = path.to_token_stream();
    PathIr {
        source: tokens.to_string(),
        segments: path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect(),
        tokens,
        span: path.span(),
    }
}

/// Converts a `syn::Type` at the parser boundary so later stages never store it.
pub(super) fn convert_type(ty: &Type) -> TypeIr {
    let tokens = ty.to_token_stream();
    let kind = match ty {
        Type::Path(path) => TypeKindIr::Path(convert_path(&path.path)),
        Type::Reference(reference) => TypeKindIr::Reference {
            mutable: reference.mutability.is_some(),
            element: Box::new(convert_type(&reference.elem)),
        },
        Type::Tuple(tuple) => TypeKindIr::Tuple(tuple.elems.iter().map(convert_type).collect()),
        Type::Slice(slice) => TypeKindIr::Slice(Box::new(convert_type(&slice.elem))),
        Type::Array(array) => TypeKindIr::Array {
            element: Box::new(convert_type(&array.elem)),
            length: array.len.to_token_stream(),
        },
        Type::Ptr(pointer) => TypeKindIr::Pointer {
            mutable: pointer.mutability.is_some(),
            element: Box::new(convert_type(&pointer.elem)),
        },
        Type::BareFn(_) => TypeKindIr::BareFunction,
        Type::TraitObject(_) => TypeKindIr::TraitObject,
        Type::ImplTrait(_) => TypeKindIr::ImplTrait,
        Type::Never(_) => TypeKindIr::Never,
        Type::Infer(_) => TypeKindIr::Infer,
        Type::Macro(_) => TypeKindIr::Macro,
        Type::Group(group) => convert_type(&group.elem).kind,
        Type::Paren(paren) => convert_type(&paren.elem).kind,
        _ => TypeKindIr::Other,
    };
    TypeIr {
        source: tokens.to_string(),
        tokens,
        kind,
        span: ty.span(),
    }
}
