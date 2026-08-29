//! Token expansion for validated reflection declarations.

mod traits;
mod impls;
mod structs;
mod enums;

use proc_macro2::TokenStream;

use crate::ir::DeclarationIr;

/// Expands one validated declaration while retaining unsupported staged declarations verbatim.
pub(crate) fn expand(declaration: DeclarationIr) -> TokenStream {
    match declaration {
        DeclarationIr::Type(declaration) => match declaration.kind {
            crate::ir::TypeDeclarationKindIr::Struct => structs::expand(declaration),
            crate::ir::TypeDeclarationKindIr::Enum => enums::expand(declaration),
            crate::ir::TypeDeclarationKindIr::Union => TokenStream::new(),
        },
        DeclarationIr::Trait(declaration) => traits::expand(declaration),
        DeclarationIr::Impl(declaration) => impls::expand_impl(declaration),
    }
}

/// Resolves the runtime facade without coupling the proc-macro crate to it.
fn facade_path() -> Option<TokenStream> {
    match proc_macro_crate::crate_name("qubit-reflect") {
        Ok(proc_macro_crate::FoundCrate::Itself) => Some(quote::quote!(crate)),
        Ok(proc_macro_crate::FoundCrate::Name(name)) => {
            let ident = proc_macro2::Ident::new(&name, proc_macro2::Span::call_site());
            Some(quote::quote!(::#ident))
        }
        Err(_) => None,
    }
}
