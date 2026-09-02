// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Runtime expression token generation.

use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use syn::Expr;
use syn::Lit;
use syn::LitInt;
use syn::LitStr;
use syn::UnOp;
use syn::parse2;

use crate::ir::GenericBoundIr;
use crate::ir::GenericKindIr;
use crate::ir::GenericsIr;
use crate::ir::PathArgumentIr;
use crate::ir::PathArgumentsIr;
use crate::ir::TraitDeclarationIr;
use crate::ir::TypeIr;
use crate::ir::TypeKindIr;
use crate::ir::WherePredicateIr;

/// Converts generic declaration facts into the runtime generic descriptor
/// model.
pub(crate) fn generic_definition(
    generics: &GenericsIr,
    span: Span,
    facade: &TokenStream,
) -> TokenStream {
    let parameters = generics.params.iter().map(|parameter| {
        let name = LitStr::new(&parameter.name, parameter.span);
        match parameter.kind {
            GenericKindIr::Lifetime => {
                let bounds = parameter.bounds.iter().filter_map(|bound| match bound {
                    GenericBoundIr::Lifetime(value) => Some(lifetime_expression(value, parameter.span, facade)),
                    _ => None,
                });
                quote!(#facade::__private::codegen_v1::expression::lifetime_parameter(
                    #name,
                    Box::new([#(#bounds),*]),
                    #facade::expression::DiagnosticText::default(),
                ))
            }
            GenericKindIr::Type => {
                let subject = LitStr::new(&parameter.name, parameter.span);
                let bounds = generic_bounds(&parameter.bounds, &subject, parameter.span, facade);
                let default = match parameter.default.as_ref() {
                    Some(crate::ir::GenericDefaultIr::Type(value)) => {
                        let value = type_expression(value, facade);
                        quote!(Some(#value))
                    }
                    _ => quote!(None),
                };
                quote!(#facade::__private::codegen_v1::expression::type_parameter(
                    #name,
                    Box::new([#(#bounds),*]),
                    #default,
                    #facade::expression::DiagnosticText::default(),
                ))
            }
            GenericKindIr::Const => {
                let ty = parameter.const_type.as_ref().map(|ty| type_expression(ty, facade)).unwrap_or_else(|| quote!(#facade::expression::TypeExpression::Concrete(#facade::__private::codegen_v1::expression::concrete(vec!["_".into()].into_boxed_slice(), vec![].into_boxed_slice(), #facade::expression::DiagnosticText::default()))));
                let default = match parameter.default.as_ref() {
                    Some(crate::ir::GenericDefaultIr::Const(value)) => const_expression(value, facade),
                    _ => quote!(None),
                };
                quote!(#facade::__private::codegen_v1::expression::const_generic_parameter(
                    #name,
                    #ty,
                    #default,
                    #facade::expression::DiagnosticText::default(),
                ))
            }
        }
    });
    let predicates = generics.where_predicates.iter().flat_map(|predicate| match predicate {
        WherePredicateIr::Lifetime { lifetime, bounds, .. } => {
            let lifetime = lifetime_expression(lifetime, span, facade);
            let bounds = bounds.iter().map(|bound| lifetime_expression(bound, span, facade));
            vec![quote!(#facade::__private::codegen_v1::expression::lifetime_outlives(
                #lifetime,
                Box::new([#(#bounds),*]),
            ))]
        }
        WherePredicateIr::Type { bounded_type, lifetimes, bounds, .. } => {
            let subject = type_expression(bounded_type, facade);
            let lifetimes = lifetimes.iter().map(|lifetime| lifetime_expression(lifetime, span, facade));
            let trait_bounds: Vec<_> = bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Trait { path, .. } => {
                    let path = LitStr::new(&path.source, span);
                    Some(quote!(#facade::expression::TypeExpression::Concrete(#facade::__private::codegen_v1::expression::concrete(vec![#path.into()].into_boxed_slice(), vec![].into_boxed_slice(), #facade::expression::DiagnosticText::from(#path)))))
                }
                _ => None,
            }).collect();
            let modifiers: Vec<_> = bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Trait { modifier, .. } => Some(match modifier {
                    crate::ir::TraitBoundModifierIr::None => quote!(#facade::expression::TraitBoundModifier::None),
                    crate::ir::TraitBoundModifierIr::Maybe => quote!(#facade::expression::TraitBoundModifier::Maybe),
                }),
                _ => None,
            }).collect();
            let type_bound = (!trait_bounds.is_empty()).then(|| {
                quote!(#facade::__private::codegen_v1::expression::type_bound(
                    #subject,
                    Box::new([#(#trait_bounds),*]),
                    Box::new([#(#modifiers),*]),
                    Box::new([#(#lifetimes),*]),
                ))
            });
            let outlives = bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Lifetime(lifetime) => {
                    let lifetime = lifetime_expression(lifetime, span, facade);
                    let subject = type_expression(bounded_type, facade);
                    Some(quote!(#facade::expression::PredicateDescriptor::TypeOutlives { ty: #subject, lifetime: #lifetime, diagnostic: #facade::expression::DiagnosticText::default() }))
                }
                _ => None,
            });
            type_bound.into_iter().chain(outlives).collect()
        }
        WherePredicateIr::Other(_) => Vec::new(),
    });
    quote!(#facade::expression::GenericDefinitionDescriptor::new(
        ::std::vec::Vec::from([#(#parameters),*]).into_boxed_slice(),
        ::std::vec::Vec::from([#(#predicates),*]).into_boxed_slice(),
    ))
}

/// Converts generic bounds into runtime predicate descriptors.
fn generic_bounds(
    bounds: &[GenericBoundIr],
    subject: &LitStr,
    span: Span,
    facade: &TokenStream,
) -> Vec<TokenStream> {
    bounds.iter().filter_map(move |bound| match bound {
        GenericBoundIr::Trait { path, lifetimes, modifier } => {
            let path = LitStr::new(&path.source, span);
            let lifetimes = lifetimes.iter().map(|lifetime| lifetime_expression(lifetime, span, facade));
            let modifier = match modifier {
                crate::ir::TraitBoundModifierIr::None => quote!(#facade::expression::TraitBoundModifier::None),
                crate::ir::TraitBoundModifierIr::Maybe => quote!(#facade::expression::TraitBoundModifier::Maybe),
            };
            Some(quote!(#facade::__private::codegen_v1::expression::type_bound(
                #facade::__private::codegen_v1::expression::parameter(#subject),
                Box::new([#facade::expression::TypeExpression::Concrete(#facade::__private::codegen_v1::expression::concrete(vec![#path.into()].into_boxed_slice(), vec![].into_boxed_slice(), #facade::expression::DiagnosticText::from(#path)))]),
                Box::new([#modifier]),
                Box::new([#(#lifetimes),*]),
            )))
        }
        GenericBoundIr::Lifetime(lifetime) => {
            let lifetime = lifetime_expression(lifetime, span, facade);
            Some(quote!(#facade::expression::PredicateDescriptor::TypeOutlives {
                ty: #facade::__private::codegen_v1::expression::parameter(#subject), lifetime: #lifetime,
                diagnostic: #facade::expression::DiagnosticText::default(),
            }))
        }
        GenericBoundIr::Other(_) => None,
    }).collect()
}

/// Converts source lifetime syntax into the runtime lifetime expression model.
pub(super) fn lifetime_expression(lifetime: &str, span: Span, facade: &TokenStream) -> TokenStream {
    if lifetime == "'static" {
        return quote!(#facade::expression::LifetimeExpression::Static);
    }
    let lifetime = LitStr::new(lifetime.trim_start_matches('\''), span);
    quote!(#facade::__private::codegen_v1::expression::named_lifetime(#lifetime))
}

/// Converts the type forms required by trait item descriptors into runtime
/// expressions.
pub(crate) fn type_expression(ty: &TypeIr, facade: &TokenStream) -> TokenStream {
    match &ty.kind {
        TypeKindIr::Never => quote!(#facade::expression::TypeExpression::Never),
        TypeKindIr::Path(path) => path_expression(path, ty, facade),
        TypeKindIr::Reference {
            lifetime,
            mutable,
            element,
        } => {
            let target = type_expression(element, facade);
            let lifetime = match lifetime.as_deref() {
                Some("'static") => {
                    quote!(#facade::expression::LifetimeExpression::Static)
                }
                Some(value) => {
                    let value = LitStr::new(value.trim_start_matches('\''), ty.span);
                    quote!(#facade::__private::codegen_v1::expression::named_lifetime(#value))
                }
                None => quote!(#facade::expression::LifetimeExpression::Elided),
            };
            quote!(#facade::expression::TypeExpression::Reference(
                #facade::expression::ReferenceTypeExpression::new(#lifetime, #mutable, #target)
            ))
        }
        TypeKindIr::Tuple(elements) => {
            let elements = elements
                .iter()
                .map(|element| type_expression(element, facade));
            quote!(#facade::expression::TypeExpression::Tuple(Box::new([#(#elements),*])))
        }
        TypeKindIr::Slice(element) => {
            let element = type_expression(element, facade);
            quote!(#facade::expression::TypeExpression::Slice(Box::new(#element)))
        }
        TypeKindIr::Array { element, length } => {
            let element = type_expression(element, facade);
            let length = const_expression_value(length, facade);
            quote!(#facade::expression::TypeExpression::Array(
                #facade::expression::ArrayTypeExpression::new(#element, #length)
            ))
        }
        TypeKindIr::Pointer { mutable, element } => {
            let target = type_expression(element, facade);
            quote!(#facade::expression::TypeExpression::RawPointer(
                #facade::expression::RawPointerTypeExpression::new(#mutable, #target)
            ))
        }
        TypeKindIr::BareFunction {
            lifetimes,
            inputs,
            output,
            is_unsafe,
            abi,
            is_variadic,
        } => {
            let higher_ranked_lifetimes = lifetimes
                .iter()
                .map(|value| lifetime_expression(value, ty.span, facade));
            let parameters = inputs.iter().map(|value| type_expression(value, facade));
            let return_type = output
                .as_deref()
                .map(|value| type_expression(value, facade))
                .unwrap_or_else(
                    || quote!(#facade::expression::TypeExpression::Tuple(Box::new([]))),
                );
            let safety = if *is_unsafe {
                quote!(#facade::expression::FunctionSafety::Unsafe)
            } else {
                quote!(#facade::expression::FunctionSafety::Safe)
            };
            let abi = match abi.as_deref() {
                Some("C") => quote!(#facade::expression::FunctionAbi::C),
                Some("system") => {
                    quote!(#facade::expression::FunctionAbi::System)
                }
                Some(value) => {
                    let value = LitStr::new(value, ty.span);
                    quote!(#facade::expression::FunctionAbi::Other(#value.into()))
                }
                None => quote!(#facade::expression::FunctionAbi::Rust),
            };
            quote!(#facade::expression::TypeExpression::FunctionPointer(
                #facade::expression::FunctionPointerExpression::new(
                    #abi,
                    #safety,
                    #is_variadic,
                    vec![#(#higher_ranked_lifetimes),*].into_boxed_slice(),
                    vec![#(#parameters),*].into_boxed_slice(),
                    #return_type,
                )
            ))
        }
        TypeKindIr::TraitObject { bounds, .. } => {
            let bounds = bound_predicates(bounds, facade, ty.span);
            quote!(#facade::expression::TypeExpression::TraitObject(
                #facade::expression::TraitObjectExpression::new(
                    vec![#(#bounds),*].into_boxed_slice(),
                )
            ))
        }
        TypeKindIr::ImplTrait { bounds } => {
            let bounds = bound_predicates(bounds, facade, ty.span);
            quote!(#facade::expression::TypeExpression::Opaque(
                #facade::expression::OpaqueTypeExpression::new(
                    vec![#(#bounds),*].into_boxed_slice(),
                )
            ))
        }
        _ => {
            let source = LitStr::new(&ty.source, ty.span);
            quote!(#facade::expression::TypeExpression::Concrete(
                #facade::__private::codegen_v1::expression::concrete(
                    vec![#source.into()].into_boxed_slice(),
                    vec![].into_boxed_slice(),
                    #facade::expression::DiagnosticText::from(#source),
                )
            ))
        }
    }
}

/// Converts one parsed path into a runtime type-expression token stream.
fn path_expression(path: &crate::ir::PathIr, ty: &TypeIr, facade: &TokenStream) -> TokenStream {
    let diagnostic = LitStr::new(&ty.source, ty.span);
    if path.qualified_self.is_none() && path.segments.len() == 1 && path.segments[0].name == "Self"
    {
        return quote!(#facade::expression::TypeExpression::SelfType);
    }
    if path.qualified_self.is_none()
        && path.segments.len() == 1
        && matches!(path.segments[0].arguments, PathArgumentsIr::None)
        && path.segments[0]
            .name
            .chars()
            .all(|value| value.is_ascii_uppercase())
    {
        let name = LitStr::new(&path.segments[0].name, ty.span);
        return quote!(#facade::__private::codegen_v1::expression::parameter(#name));
    }
    if let Some(qualified) = &path.qualified_self {
        let self_type = type_expression(&qualified.ty, facade);
        let item = path
            .segments
            .last()
            .map(|segment| LitStr::new(&segment.name, ty.span))
            .expect("qualified path has an item");
        let arguments = path
            .segments
            .last()
            .map(|segment| path_arguments(&segment.arguments, facade, ty.span))
            .unwrap_or_default();
        let trait_segments = path
            .segments
            .iter()
            .take(qualified.position)
            .map(|segment| LitStr::new(&segment.name, ty.span));
        let trait_path = if qualified.has_as {
            quote!(Some(Box::new(#facade::expression::TypeExpression::Concrete(
                #facade::__private::codegen_v1::expression::concrete(
                    vec![#(#trait_segments.into()),*].into_boxed_slice(),
                    vec![].into_boxed_slice(),
                    #facade::expression::DiagnosticText::default(),
                )
            ))))
        } else {
            quote!(None)
        };
        return quote!(#facade::expression::TypeExpression::Associated(
            #facade::expression::AssociatedTypeExpression::new(
                #self_type,
                #trait_path.map(|value| *value),
                #item,
                vec![#(#arguments),*].into_boxed_slice(),
            ).with_diagnostic(#diagnostic)
        ));
    }
    let segments = path
        .segments
        .iter()
        .map(|segment| LitStr::new(&segment.name, ty.span));
    let arguments = path
        .segments
        .last()
        .map(|segment| path_arguments(&segment.arguments, facade, ty.span))
        .unwrap_or_default();
    quote!(#facade::expression::TypeExpression::Concrete(
        #facade::__private::codegen_v1::expression::concrete(
            vec![#(#segments.into()),*].into_boxed_slice(),
            vec![#(#arguments),*].into_boxed_slice(),
            #facade::expression::DiagnosticText::from(#diagnostic),
        )
    ))
}

/// Converts parsed path arguments into runtime generic-argument expressions.
fn path_arguments(
    arguments: &PathArgumentsIr,
    facade: &TokenStream,
    span: Span,
) -> Vec<TokenStream> {
    match arguments {
        PathArgumentsIr::None => Vec::new(),
        PathArgumentsIr::Parenthesized { inputs, output } => {
            let inputs = inputs.iter().map(|value| type_expression(value, facade));
            let output = output.as_deref().map(|value| type_expression(value, facade)).unwrap_or_else(|| quote!(#facade::expression::TypeExpression::Tuple(Box::new([]))));
            vec![quote!(#facade::expression::GenericArgument::Type(
                #facade::expression::TypeExpression::FunctionPointer(
                    #facade::expression::FunctionPointerExpression::new(
                        #facade::expression::FunctionAbi::Rust,
                        #facade::expression::FunctionSafety::Safe,
                        false,
                        vec![].into_boxed_slice(),
                        vec![#(#inputs),*].into_boxed_slice(),
                        #output,
                    )
                )
            ))]
        }
        PathArgumentsIr::AngleBracketed(values) => values.iter().filter_map(|value| match value {
            PathArgumentIr::Lifetime(value) => { let value = lifetime_expression(value, span, facade); Some(quote!(#facade::expression::GenericArgument::Lifetime(#value))) }
            PathArgumentIr::Type(value) => { let value = type_expression(value, facade); Some(quote!(#facade::expression::GenericArgument::Type(#value))) }
            PathArgumentIr::Const(value) => { let value = const_expression_value(value, facade); let source = LitStr::new(&value.to_string(), span); Some(quote!(#facade::expression::GenericArgument::Const(#facade::expression::ConstGenericArgument::new(#facade::expression::TypeExpression::Concrete(#facade::__private::codegen_v1::expression::concrete(vec!["_".into()].into_boxed_slice(), vec![].into_boxed_slice(), #facade::expression::DiagnosticText::default())), #value, #source)))) }
            PathArgumentIr::AssociatedType { name, ty } => { let name = LitStr::new(name, span); let value = type_expression(ty, facade); Some(quote!(#facade::__private::codegen_v1::expression::associated_type(#name, #value))) }
            _ => None,
        }).collect(),
    }
}

/// Materializes direct external-supertrait arguments at the trait hook's
/// concrete instance.
pub(super) fn external_supertrait_arguments(
    arguments: &PathArgumentsIr,
    declaration: &TraitDeclarationIr,
    facade: &TokenStream,
) -> Vec<TokenStream> {
    let PathArgumentsIr::AngleBracketed(values) = arguments else {
        return path_arguments(arguments, facade, declaration.span);
    };
    values
        .iter()
        .filter_map(|value| match value {
            PathArgumentIr::Type(
                value @ TypeIr {
                    kind: TypeKindIr::Path(path),
                    ..
                },
            ) if path.segments.len() == 1 => {
                let name = &path.segments[0].name;
                if let Some(parameter) = declaration
                    .generics
                    .params
                    .iter()
                    .find(|parameter| parameter.kind == GenericKindIr::Type && parameter.name == *name)
                {
                    let identifier = Ident::new(&parameter.name, parameter.span);
                    Some(quote!(#facade::expression::GenericArgument::Type(
                        #facade::expression::TypeExpression::Concrete(
                            #facade::__private::codegen_v1::expression::concrete(
                                vec![std::any::type_name::<#identifier>().into()].into_boxed_slice(),
                                vec![].into_boxed_slice(),
                                #facade::expression::DiagnosticText::from(std::any::type_name::<#identifier>()),
                            ),
                        ),
                    )))
                } else {
                    path_arguments(
                        &PathArgumentsIr::AngleBracketed(vec![PathArgumentIr::Type(value.clone())]),
                        facade,
                        declaration.span,
                    )
                    .into_iter()
                    .next()
                }
            }
            value => path_arguments(
                &PathArgumentsIr::AngleBracketed(vec![value.clone()]),
                facade,
                declaration.span,
            )
            .into_iter()
            .next(),
        })
        .collect()
}

/// Converts trait-object or opaque-type bounds into runtime predicates.
fn bound_predicates(
    bounds: &[GenericBoundIr],
    facade: &TokenStream,
    span: Span,
) -> Vec<TokenStream> {
    bounds
        .iter()
        .filter_map(|bound| match bound {
            GenericBoundIr::Trait {
                path,
                modifier,
                lifetimes,
            } => {
                let source = LitStr::new(&path.source, span);
                let modifier = match modifier {
                    crate::ir::TraitBoundModifierIr::None => {
                        quote!(#facade::expression::TraitBoundModifier::None)
                    }
                    crate::ir::TraitBoundModifierIr::Maybe => {
                        quote!(#facade::expression::TraitBoundModifier::Maybe)
                    }
                };
                let lifetimes = lifetimes
                    .iter()
                    .map(|value| lifetime_expression(value, span, facade));
                Some(
                    quote!(#facade::__private::codegen_v1::expression::type_bound(
                        #facade::expression::TypeExpression::SelfType,
                        Box::new([#facade::expression::TypeExpression::Concrete(
                            #facade::__private::codegen_v1::expression::concrete(
                                vec![#source.into()].into_boxed_slice(),
                                vec![].into_boxed_slice(),
                                #facade::expression::DiagnosticText::from(#source),
                            ),
                        )]),
                        Box::new([#modifier]),
                        Box::new([#(#lifetimes),*]),
                    )),
                )
            }
            GenericBoundIr::Lifetime(value) => {
                let value = lifetime_expression(value, span, facade);
                Some(
                    quote!(#facade::expression::PredicateDescriptor::TypeOutlives {
                        ty: #facade::expression::TypeExpression::SelfType,
                        lifetime: #value,
                        diagnostic: #facade::expression::DiagnosticText::default(),
                    }),
                )
            }
            _ => None,
        })
        .collect()
}

/// Converts a parsed const expression into structural runtime metadata.
fn const_expression_value(value: &TokenStream, facade: &TokenStream) -> TokenStream {
    let source = value.to_string();
    if let Ok(identifier) = parse2::<Ident>(value.clone()) {
        let name = LitStr::new(&identifier.to_string(), identifier.span());
        return quote!(#facade::__private::codegen_v1::expression::const_parameter(#name));
    }
    if let Ok(value) = parse2::<Lit>(value.clone()) {
        match value {
            Lit::Bool(value) => {
                let value = value.value;
                return quote!(#facade::expression::ConstExpression::Boolean(#value));
            }
            Lit::Char(value) => {
                let value = value.value();
                return quote!(#facade::expression::ConstExpression::Character(#value));
            }
            Lit::Int(value) => {
                if let Ok(value) = value.base10_parse::<u128>() {
                    return quote!(#facade::expression::ConstExpression::UnsignedInteger(#value));
                }
            }
            _ => {}
        }
    }
    if let Ok(Expr::Unary(value)) = parse2::<Expr>(value.clone())
        && matches!(value.op, UnOp::Neg(_))
        && let Expr::Lit(literal) = *value.expr
        && let Lit::Int(value) = literal.lit
        && let Ok(value) = value.base10_parse::<i128>()
    {
        let value = -value;
        return quote!(#facade::expression::ConstExpression::SignedInteger(#value));
    }
    let source = LitStr::new(&source, Span::call_site());
    quote!(compile_error!(
        concat!("unsupported const expression in #[reflect] trait: ", #source)
    ))
}

/// Converts the literal const-default subset that has a structural runtime
/// representation.
pub(super) fn const_expression(value: &TokenStream, facade: &TokenStream) -> TokenStream {
    let expression = parse2::<Expr>(value.clone());
    let expression = match expression {
        Ok(expression) => expression,
        Err(_) => return unsupported_const_default(value),
    };
    match expression {
        Expr::Lit(expression) => match expression.lit {
            Lit::Bool(value) => {
                let value = value.value;
                quote!(Some(#facade::expression::ConstExpression::Boolean(#value)))
            }
            Lit::Char(value) => {
                let value = value.value();
                quote!(Some(#facade::expression::ConstExpression::Character(#value)))
            }
            Lit::Int(value) => integer_const_expression(&value, false, facade)
                .unwrap_or_else(|| unsupported_const_default(value.to_token_stream())),
            _ => unsupported_const_default(value),
        },
        Expr::Unary(expression) if matches!(expression.op, UnOp::Neg(_)) => {
            match expression.expr.as_ref() {
                Expr::Lit(expression) => match &expression.lit {
                    Lit::Int(value) => integer_const_expression(value, true, facade)
                        .unwrap_or_else(|| unsupported_const_default(value.to_token_stream())),
                    _ => unsupported_const_default(value),
                },
                _ => unsupported_const_default(value),
            }
        }
        _ => unsupported_const_default(value),
    }
}

/// Converts an integer literal without relying on its whitespace-normalized
/// token rendering.
fn integer_const_expression(
    value: &LitInt,
    negative: bool,
    facade: &TokenStream,
) -> Option<TokenStream> {
    let suffix = value.suffix();
    let signed = negative || matches!(suffix, "i8" | "i16" | "i32" | "i64" | "i128" | "isize");
    if signed {
        let magnitude = value.base10_parse::<i128>().ok()?;
        let value = if negative {
            magnitude.checked_neg()?
        } else {
            magnitude
        };
        Some(quote!(Some(#facade::expression::ConstExpression::SignedInteger(#value))))
    } else {
        let value = value.base10_parse::<u128>().ok()?;
        Some(quote!(Some(#facade::expression::ConstExpression::UnsignedInteger(#value))))
    }
}

/// Emits a deterministic compile error for const defaults without a runtime
/// structural value.
fn unsupported_const_default(value: impl ToTokens) -> TokenStream {
    let source = LitStr::new(&value.into_token_stream().to_string(), Span::call_site());
    quote!(compile_error!(
        concat!("unsupported non-literal const default in #[reflect] trait: ", #source)
    ))
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;

    use super::const_expression;

    /// Verifies that literals retain their structural value rather than their
    /// token spelling.
    #[test]
    fn test_const_default_accepts_signed_suffixed_and_escaped_literals() {
        let facade = quote!(qubit_reflect);
        let signed = const_expression(&quote!(-7i16), &facade).to_string();
        let unsigned = const_expression(&quote!(42u8), &facade).to_string();
        let escaped = const_expression(&quote!('\n'), &facade).to_string();

        assert!(signed.contains("SignedInteger"));
        assert!(signed.contains("- 7i128"));
        assert!(unsigned.contains("UnsignedInteger"));
        assert!(unsigned.contains("42u128"));
        assert!(escaped.contains("Character"));
        assert!(escaped.contains("'\\n'"));
    }

    /// Verifies that symbolic defaults fail explicitly instead of becoming
    /// symbolic values.
    #[test]
    fn test_const_default_rejects_non_literal_expression() {
        let value: TokenStream = quote!(DEFAULT_LIMIT);
        let rendered = const_expression(&value, &quote!(qubit_reflect)).to_string();

        assert!(rendered.contains("unsupported non-literal const default"));
        assert!(rendered.contains("DEFAULT_LIMIT"));
    }
}
