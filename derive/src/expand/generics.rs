//! Expansion helpers for generic root-instance metadata.

use proc_macro2::TokenStream;
use quote::quote;

use crate::ir::GenericKindIr;
use crate::ir::TypeDeclarationIr;

/// Emits the concrete generic view for the current monomorphized root.
pub(crate) fn concrete_descriptor(declaration: &TypeDeclarationIr, facade: &TokenStream) -> TokenStream {
    if declaration.generics.params.is_empty() {
        return TokenStream::new();
    }
    let definition = super::traits::generic_definition(&declaration.generics, declaration.span, facade);
    let arguments = declaration.generics.params.iter().filter_map(|parameter| match parameter.kind {
        GenericKindIr::Lifetime => None,
        GenericKindIr::Type => {
            let name = syn::Ident::new(&parameter.name, parameter.span);
            Some(quote!(#facade::expression::GenericArgument::Type(
                #facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression {
                    path: Box::new([::std::any::type_name::<#name>().into()]),
                    arguments: Box::new([]),
                    diagnostic: #facade::expression::DiagnosticText::from(::std::any::type_name::<#name>()),
                }),
            )))
        }
        GenericKindIr::Const => {
            let name = syn::Ident::new(&parameter.name, parameter.span);
            let source = parameter.const_type.as_ref()?.source.as_str();
            let value = match source {
                "bool" => quote!(#facade::expression::ConstExpression::Boolean(#name)),
                "char" => quote!(#facade::expression::ConstExpression::Character(#name)),
                "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => {
                    quote!(#facade::expression::ConstExpression::SignedInteger(#name as i128))
                }
                "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => {
                    quote!(#facade::expression::ConstExpression::UnsignedInteger(#name as u128))
                }
                _ => return None,
            };
            let source = syn::LitStr::new(source, parameter.span);
            Some(quote!(#facade::expression::GenericArgument::Const(
                #facade::expression::ConstGenericArgument {
                    declared_type: Box::new(#facade::expression::TypeExpression::Concrete(
                        #facade::expression::ConcreteTypeExpression {
                            path: Box::new([#source.into()]), arguments: Box::new([]),
                            diagnostic: #facade::expression::DiagnosticText::from(#source),
                        },
                    )),
                    value: #value,
                    normalized_diagnostic: stringify!(#name).into(),
                },
            )))
        }
    });
    quote!(#facade::descriptor::ConcreteGenericDescriptor::new(
        ::std::boxed::Box::leak(::std::boxed::Box::new(#definition)),
        ::std::boxed::Box::leak(::std::vec![#(#arguments),*].into_boxed_slice()),
    ))
}
