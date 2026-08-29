//! Token expansion for validated reflection declarations.

mod traits;

use proc_macro2::TokenStream;

use crate::ir::DeclarationIr;

/// Expands one validated declaration while retaining unsupported staged declarations verbatim.
pub(crate) fn expand(declaration: DeclarationIr) -> TokenStream {
    match declaration {
        DeclarationIr::Type(_) => TokenStream::new(),
        DeclarationIr::Trait(declaration) => traits::expand(declaration),
        DeclarationIr::Impl(declaration) => declaration.retained_tokens,
    }
}
