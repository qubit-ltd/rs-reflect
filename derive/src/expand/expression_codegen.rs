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

use super::generic_environment::GenericEnvironment;
use crate::ir::GenericBoundIr;
use crate::ir::GenericKindIr;
use crate::ir::GenericsIr;
use crate::ir::PathArgumentIr;
use crate::ir::PathArgumentsIr;
use crate::ir::TraitDeclarationIr;
use crate::ir::TypeIr;
use crate::ir::TypeKindIr;
use crate::ir::WherePredicateIr;

/// Maps a parsed declaration ABI into executable invocation metadata.
///
/// # Parameters
///
/// - `abi`: Source ABI spelling, or `None` for the Rust default.
/// - `span`: Source span used for unsupported ABI diagnostics.
/// - `facade`: Runtime facade path used in the generated expression.
///
/// # Returns
///
/// Returns tokens naming the corresponding codegen_v4 invocation ABI.
#[must_use]
pub(crate) fn function_abi(abi: Option<&str>, span: Span, facade: &TokenStream) -> TokenStream {
    match abi {
        Some("Rust") | None => quote!(#facade::__private::codegen_v4::invoke::FunctionAbi::Rust),
        Some("C") => quote!(#facade::__private::codegen_v4::invoke::FunctionAbi::C),
        Some("system") => quote!(#facade::__private::codegen_v4::invoke::FunctionAbi::System),
        Some(value) => {
            let value = LitStr::new(value, span);
            quote!(#facade::__private::codegen_v4::invoke::FunctionAbi::Other(#value))
        }
    }
}

fn function_pointer_abi(abi: Option<&str>, span: Span) -> TokenStream {
    match abi {
        Some("Rust") | None => quote!(None),
        Some(value) => {
            let value = LitStr::new(value, span);
            quote!(Some(#value))
        }
    }
}

/// Converts generic declaration facts into the runtime generic descriptor
/// model.
///
/// # Parameters
///
/// - `generics`: Parsed generic parameters and where predicates.
/// - `span`: Declaration span used for lifetime metadata.
/// - `facade`: Runtime facade path used in generated expressions.
///
/// # Returns
///
/// Returns tokens constructing the runtime generic definition descriptor.
#[must_use]
pub(crate) fn generic_definition(
    generics: &GenericsIr,
    span: Span,
    facade: &TokenStream,
) -> TokenStream {
    let environment = GenericEnvironment::from_generics(generics);
    let parameters = generics.params.iter().enumerate().map(|(index, parameter)| {
        let name = LitStr::new(&parameter.name, parameter.span);
        match parameter.kind {
            GenericKindIr::Lifetime => {
                let bounds = parameter.bounds.iter().filter_map(|bound| match bound {
                    GenericBoundIr::Lifetime(value) => Some(lifetime_expression(value, parameter.span, facade)),
                    _ => None,
                });
                let bounds = leak_slice(bounds);
                quote!(#facade::__private::codegen_v4::expression::GenericParameterMetadata::lifetime(
                    #name, #index, #bounds,
                ))
            }
            GenericKindIr::Type => {
                let subject = LitStr::new(&parameter.name, parameter.span);
                let bounds = generic_bounds(&parameter.bounds, &subject, parameter.span, &environment, facade);
                let bounds = leak_slice(bounds);
                let default = match parameter.default.as_ref() {
                    Some(crate::ir::GenericDefaultIr::Type(value)) => {
                        let value = type_expression(value, &environment, facade);
                        quote!(Some(#value))
                    }
                    _ => quote!(None),
                };
                quote!(#facade::__private::codegen_v4::expression::GenericParameterMetadata::type_parameter(
                    #name, #index, #bounds, #default,
                ))
            }
            GenericKindIr::Const => {
                let ty = parameter
                    .const_type
                    .as_ref()
                    .map(|ty| type_expression(ty, &environment, facade))
                    .unwrap_or_else(|| {
                        leaked_expression(quote!(#facade::__private::codegen_v4::expression::ExpressionData::Named {
                            path: "_", definition: None, arguments: &[],
                        }))
                    });
                let default = match parameter.default.as_ref() {
                    Some(crate::ir::GenericDefaultIr::Const(value)) => const_expression(value, &environment, facade),
                    _ => quote!(None),
                };
                quote!(#facade::__private::codegen_v4::expression::GenericParameterMetadata::const_parameter(
                    #name, #index, #ty, #default,
                ))
            }
        }
    });
    let predicates = generics.where_predicates.iter().flat_map(|predicate| match predicate {
        WherePredicateIr::Lifetime { lifetime, bounds, .. } => {
            let lifetime = lifetime_expression(lifetime, span, facade);
            let bounds = bounds.iter().map(|bound| lifetime_expression(bound, span, facade));
            bounds.map(|bound| quote!(#facade::__private::codegen_v4::expression::PredicateData::LifetimeOutlives {
                lifetime: #lifetime, outlives: #bound,
            })).collect::<Vec<_>>()
        }
        WherePredicateIr::Type {
            bounded_type,
            lifetimes,
            bounds,
            ..
        } => {
            let subject = type_expression(bounded_type, &environment, facade);
            let mut predicates = Vec::new();
            for bound in bounds {
                if let GenericBoundIr::Trait { path, modifier, lifetimes } = bound {
                    let trait_ = trait_expression_data(path, lifetimes, &environment, facade, span);
                    let modifier = trait_bound_modifier(*modifier, facade);
                    predicates.push(quote!(#facade::__private::codegen_v4::expression::PredicateData::TraitBound {
                        ty: #subject, trait_: #trait_, modifier: #modifier,
                    }));
                }
                if let GenericBoundIr::Lifetime(lifetime) = bound {
                    let lifetime = lifetime_expression(lifetime, span, facade);
                    predicates.push(quote!(#facade::__private::codegen_v4::expression::PredicateData::TypeOutlives {
                        ty: #subject, lifetime: #lifetime,
                    }));
                }
            }
            let outlives = lifetimes.iter().map(|lifetime| {
                let lifetime = lifetime_expression(lifetime, span, facade);
                quote!(#facade::__private::codegen_v4::expression::PredicateData::TypeOutlives {
                    ty: #subject, lifetime: #lifetime,
                })
            });
            predicates.extend(outlives);
            predicates
        }
        WherePredicateIr::Other(_) => Vec::new(),
    });
    let parameters = leak_slice(parameters);
    let predicates = leak_slice(predicates);
    quote!(#facade::__private::codegen_v4::expression::GenericDefinitionMetadata::new(
        #parameters,
        #predicates,
    ))
}

/// Converts generic bounds into structural predicates over one type parameter.
///
/// # Parameters
///
/// - `bounds`: Bounds attached to one generic parameter.
/// - `subject`: Parameter name represented by those bounds.
/// - `span`: Source span used for lifetime expressions.
/// - `environment`: Generic names visible at the metadata site.
/// - `facade`: Runtime facade path used in generated expressions.
///
/// # Returns
///
/// Returns static predicate tokens in source order.
#[must_use]
fn generic_bounds(
    bounds: &[GenericBoundIr],
    subject: &LitStr,
    span: Span,
    environment: &GenericEnvironment,
    facade: &TokenStream,
) -> Vec<TokenStream> {
    bounds
        .iter()
        .filter_map(|bound| match bound {
            GenericBoundIr::Trait { path, modifier, lifetimes } => {
                let subject = leaked_expression(quote!(#facade::__private::codegen_v4::expression::ExpressionData::ParameterName(#subject)));
                let trait_ = trait_expression_data(path, lifetimes, environment, facade, span);
                let modifier = trait_bound_modifier(*modifier, facade);
                Some(quote!(#facade::__private::codegen_v4::expression::PredicateData::TraitBound {
                    ty: #subject, trait_: #trait_, modifier: #modifier,
                }))
            }
            GenericBoundIr::Lifetime(lifetime) => {
                let subject = leaked_expression(quote!(#facade::__private::codegen_v4::expression::ExpressionData::ParameterName(#subject)));
                let lifetime = lifetime_expression(lifetime, span, facade);
                Some(quote!(#facade::__private::codegen_v4::expression::PredicateData::TypeOutlives {
                    ty: #subject, lifetime: #lifetime,
                }))
            }
            GenericBoundIr::Other(_) => None,
        })
        .collect()
}

fn leak_slice(values: impl IntoIterator<Item = TokenStream>) -> TokenStream {
    let values = values.into_iter().collect::<Vec<_>>();
    quote!(::std::boxed::Box::leak(
        ::std::vec![#(#values),*].into_boxed_slice()
    ))
}

fn leaked_expression(expression: TokenStream) -> TokenStream {
    quote!(::std::boxed::Box::leak(::std::boxed::Box::new(#expression)))
}

fn trait_bound_modifier(
    modifier: crate::ir::TraitBoundModifierIr,
    facade: &TokenStream,
) -> TokenStream {
    match modifier {
        crate::ir::TraitBoundModifierIr::None => {
            quote!(#facade::__private::codegen_v4::expression::TraitBoundModifier::None)
        }
        crate::ir::TraitBoundModifierIr::Maybe => {
            quote!(#facade::__private::codegen_v4::expression::TraitBoundModifier::Maybe)
        }
    }
}

fn trait_expression_data(
    path: &crate::ir::PathIr,
    lifetime_binders: &[String],
    environment: &GenericEnvironment,
    facade: &TokenStream,
    span: Span,
) -> TokenStream {
    let name = LitStr::new(&path.source, path.span);
    let arguments = path
        .segments
        .iter()
        .flat_map(|segment| path_arguments(&segment.arguments, environment, facade, path.span));
    let arguments = leak_slice(arguments);
    let lifetimes = lifetime_binders
        .iter()
        .map(|value| lifetime_expression(value, span, facade));
    let lifetimes = leak_slice(lifetimes);
    quote!(::std::boxed::Box::leak(::std::boxed::Box::new(
        #facade::__private::codegen_v4::expression::TraitExpressionData::new(#name, #arguments, #lifetimes)
    )))
}

/// Converts source lifetime syntax into the runtime lifetime expression model.
///
/// # Parameters
///
/// - `lifetime`: Rust lifetime spelling, including its leading apostrophe when
///   named.
/// - `span`: Source span associated with the lifetime.
/// - `facade`: Runtime facade path used in the generated expression.
///
/// # Returns
///
/// Returns tokens for the static, named, or otherwise named lifetime
/// expression.
#[must_use]
pub(super) fn lifetime_expression(lifetime: &str, span: Span, facade: &TokenStream) -> TokenStream {
    if lifetime == "'static" {
        return quote!(#facade::__private::codegen_v4::expression::LifetimeExpression::Static);
    }
    let lifetime = LitStr::new(lifetime.trim_start_matches('\''), span);
    quote!(#facade::__private::codegen_v4::expression::LifetimeExpression::Named(#lifetime))
}

/// Converts the type forms required by trait item descriptors into runtime
/// expressions.
///
/// # Parameters
///
/// - `ty`: Parsed Rust type structure.
/// - `environment`: Generic names visible at the type's metadata site.
/// - `facade`: Runtime facade path used in generated expressions.
///
/// # Returns
///
/// Returns tokens representing the type expression in runtime metadata.
#[must_use]
pub(crate) fn type_expression(
    ty: &TypeIr,
    environment: &GenericEnvironment,
    facade: &TokenStream,
) -> TokenStream {
    match &ty.kind {
        TypeKindIr::Never => leaked_expression(
            quote!(#facade::__private::codegen_v4::expression::ExpressionData::Never),
        ),
        TypeKindIr::Path(path) => path_expression(path, ty, environment, facade),
        TypeKindIr::Reference {
            lifetime,
            mutable,
            element,
        } => {
            let target = type_expression(element, environment, facade);
            let lifetime = match lifetime.as_deref() {
                Some("'static") => {
                    quote!(Some(#facade::__private::codegen_v4::expression::LifetimeExpression::Static))
                }
                Some(value) => {
                    let value = LitStr::new(value.trim_start_matches('\''), ty.span);
                    quote!(Some(#facade::__private::codegen_v4::expression::LifetimeExpression::Named(#value)))
                }
                None => quote!(None),
            };
            leaked_expression(
                quote!(#facade::__private::codegen_v4::expression::ExpressionData::Reference {
                    lifetime: #lifetime,
                    mutable: #mutable,
                    target: #target,
                }),
            )
        }
        TypeKindIr::Tuple(elements) => {
            let elements = elements
                .iter()
                .map(|element| type_expression(element, environment, facade));
            let elements = leak_slice(elements);
            leaked_expression(
                quote!(#facade::__private::codegen_v4::expression::ExpressionData::Tuple(#elements)),
            )
        }
        TypeKindIr::Slice(element) => {
            let element = type_expression(element, environment, facade);
            leaked_expression(
                quote!(#facade::__private::codegen_v4::expression::ExpressionData::Slice(#element)),
            )
        }
        TypeKindIr::Array { element, length } => {
            let element = type_expression(element, environment, facade);
            let length = const_expression_value(length, environment, facade);
            leaked_expression(
                quote!(#facade::__private::codegen_v4::expression::ExpressionData::Array {
                    element: #element, length: #length,
                }),
            )
        }
        TypeKindIr::Pointer { mutable, element } => {
            let target = type_expression(element, environment, facade);
            leaked_expression(
                quote!(#facade::__private::codegen_v4::expression::ExpressionData::RawPointer {
                    mutable: #mutable, target: #target,
                }),
            )
        }
        TypeKindIr::BareFunction {
            lifetimes,
            inputs,
            output,
            is_unsafe,
            abi,
            is_variadic,
        } => {
            let parameters = inputs
                .iter()
                .map(|value| type_expression(value, environment, facade));
            let parameters = leak_slice(parameters);
            let higher_ranked_lifetimes = lifetimes
                .iter()
                .map(|value| lifetime_expression(value, ty.span, facade));
            let higher_ranked_lifetimes = leak_slice(higher_ranked_lifetimes);
            let return_type = output
                .as_deref()
                .map(|value| type_expression(value, environment, facade))
                .map(|value| quote!(Some(#value)));
            let return_type = return_type.unwrap_or_else(|| quote!(None));
            let abi = function_pointer_abi(abi.as_deref(), ty.span);
            let data = quote!(#facade::__private::codegen_v4::expression::FunctionPointerData::new(
                #parameters, #return_type, #is_unsafe, #abi, #is_variadic, #higher_ranked_lifetimes,
            ));
            let data = leaked_expression(data);
            leaked_expression(
                quote!(#facade::__private::codegen_v4::expression::ExpressionData::FunctionPointer(#data)),
            )
        }
        TypeKindIr::TraitObject { bounds, .. } => {
            let traits = bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Trait {
                    path, lifetimes, ..
                } => {
                    let trait_data =
                        trait_expression_data(path, lifetimes, environment, facade, ty.span);
                    Some(quote!(*#trait_data))
                }
                _ => None,
            });
            let traits = leak_slice(traits);
            let lifetime = bounds
                .iter()
                .find_map(|bound| match bound {
                    GenericBoundIr::Lifetime(value) => {
                        Some(lifetime_expression(value, ty.span, facade))
                    }
                    _ => None,
                })
                .map(|value| quote!(Some(#value)))
                .unwrap_or_else(|| quote!(None));
            let data = leaked_expression(
                quote!(#facade::__private::codegen_v4::expression::TraitObjectData::new(
                    #traits, #lifetime,
                )),
            );
            leaked_expression(
                quote!(#facade::__private::codegen_v4::expression::ExpressionData::TraitObject(#data)),
            )
        }
        TypeKindIr::ImplTrait { bounds } => {
            let bounds = bound_predicates(bounds, environment, facade, ty.span);
            let bounds = leak_slice(bounds);
            let data = leaked_expression(
                quote!(#facade::__private::codegen_v4::expression::OpaqueTypeData::new(#bounds)),
            );
            leaked_expression(
                quote!(#facade::__private::codegen_v4::expression::ExpressionData::Opaque(#data)),
            )
        }
        _ => {
            let source = LitStr::new(&ty.source, ty.span);
            leaked_expression(
                quote!(#facade::__private::codegen_v4::expression::ExpressionData::Named {
                    path: #source, definition: None, arguments: &[],
                }),
            )
        }
    }
}

/// Converts one parsed path into a runtime type-expression token stream.
///
/// # Parameters
///
/// - `path`: Structured path and generic arguments.
/// - `ty`: Full parsed type, used for source diagnostics.
/// - `environment`: Generic names visible at the type's metadata site.
/// - `facade`: Runtime facade path used in generated expressions.
///
/// # Returns
///
/// Returns tokens for the structural, associated, or concrete path expression.
///
/// # Panics
///
/// Panics if a qualified path has no associated item segment, which violates
/// the parser's validated path invariant.
#[must_use]
fn path_expression(
    path: &crate::ir::PathIr,
    ty: &TypeIr,
    environment: &GenericEnvironment,
    facade: &TokenStream,
) -> TokenStream {
    if path.qualified_self.is_none() && path.segments.len() == 1 && path.segments[0].name == "Self"
    {
        return leaked_expression(
            quote!(#facade::__private::codegen_v4::expression::ExpressionData::SelfType),
        );
    }
    if path.qualified_self.is_none()
        && path.segments.len() == 1
        && matches!(path.segments[0].arguments, PathArgumentsIr::None)
        && environment.is_type_parameter(&path.segments[0].name)
    {
        let name = LitStr::new(&path.segments[0].name, ty.span);
        return leaked_expression(
            quote!(#facade::__private::codegen_v4::expression::ExpressionData::ParameterName(#name)),
        );
    }
    if path.qualified_self.is_none() && path.segments.len() == 2 {
        let owner = &path.segments[0].name;
        if owner == "Self" || environment.is_type_parameter(owner) {
            let self_type = if owner == "Self" {
                leaked_expression(
                    quote!(#facade::__private::codegen_v4::expression::ExpressionData::SelfType),
                )
            } else {
                let owner = LitStr::new(owner, ty.span);
                leaked_expression(
                    quote!(#facade::__private::codegen_v4::expression::ExpressionData::ParameterName(#owner)),
                )
            };
            let item_segment = &path.segments[1];
            let item = LitStr::new(&item_segment.name, ty.span);
            return leaked_expression(
                quote!(#facade::__private::codegen_v4::expression::ExpressionData::AssociatedType {
                    self_type: #self_type, trait_: None, name: #item,
                }),
            );
        }
    }
    if let Some(qualified) = &path.qualified_self {
        let self_type = type_expression(&qualified.ty, environment, facade);
        let item = path
            .segments
            .last()
            .map(|segment| LitStr::new(&segment.name, ty.span))
            .expect("qualified path has an item");
        let trait_ = if qualified.has_as {
            let trait_name = LitStr::new(
                &path
                    .segments
                    .iter()
                    .take(qualified.position)
                    .map(|segment| segment.name.as_str())
                    .collect::<Vec<_>>()
                    .join("::"),
                ty.span,
            );
            let arguments = path
                .segments
                .iter()
                .take(qualified.position)
                .flat_map(|segment| {
                    path_arguments(&segment.arguments, environment, facade, ty.span)
                });
            let arguments = leak_slice(arguments);
            quote!(Some(::std::boxed::Box::leak(::std::boxed::Box::new(
                #facade::__private::codegen_v4::expression::TraitExpressionData::new(#trait_name, #arguments, &[])
            )) as &'static #facade::__private::codegen_v4::expression::TraitExpressionData))
        } else {
            quote!(None)
        };
        return leaked_expression(
            quote!(#facade::__private::codegen_v4::expression::ExpressionData::AssociatedType {
                self_type: #self_type, trait_: #trait_, name: #item,
            }),
        );
    }
    let source = LitStr::new(&ty.source, ty.span);
    let arguments = path
        .segments
        .iter()
        .flat_map(|segment| path_arguments(&segment.arguments, environment, facade, ty.span));
    let arguments = leak_slice(arguments);
    leaked_expression(
        quote!(#facade::__private::codegen_v4::expression::ExpressionData::Named {
            path: #source, definition: None, arguments: #arguments,
        }),
    )
}

/// Converts a parsed trait path using the same structural path encoder as a
/// type expression.
///
/// # Parameters
///
/// - `path`: Structured trait path and generic arguments.
/// - `environment`: Generic names visible at the trait metadata site.
/// - `facade`: Runtime facade path used in generated expressions.
///
/// # Returns
///
/// Returns the trait path as a runtime type expression.
#[must_use]
pub(crate) fn path_type_expression(
    path: &crate::ir::PathIr,
    environment: &GenericEnvironment,
    facade: &TokenStream,
) -> TokenStream {
    let ty = TypeIr {
        source: path.source.clone(),
        tokens: path.tokens.clone(),
        kind: TypeKindIr::Path(path.clone()),
        span: path.span,
    };
    path_expression(path, &ty, environment, facade)
}

/// Converts parsed path arguments into runtime generic-argument expressions.
///
/// # Parameters
///
/// - `arguments`: Parsed angle-bracketed or parenthesized arguments.
/// - `environment`: Generic names visible at this metadata site.
/// - `facade`: Runtime facade path used in generated expressions.
/// - `span`: Source span used for synthesized literal metadata.
///
/// # Returns
///
/// Returns supported arguments in source order; unsupported forms are omitted.
#[must_use]
pub(crate) fn path_arguments(
    arguments: &PathArgumentsIr,
    environment: &GenericEnvironment,
    facade: &TokenStream,
    span: Span,
) -> Vec<TokenStream> {
    match arguments {
        PathArgumentsIr::None => Vec::new(),
        PathArgumentsIr::Parenthesized { inputs, output } => {
            let inputs = inputs.iter().map(|value| type_expression(value, environment, facade));
            let inputs = leak_slice(inputs);
            let output = output.as_deref().map(|value| type_expression(value, environment, facade));
            let output = output.map(|value| quote!(Some(#value))).unwrap_or_else(|| quote!(None));
            let abi = quote!(None);
            let lifetimes = leak_slice(std::iter::empty());
            let function = leaked_expression(quote!(#facade::__private::codegen_v4::expression::FunctionPointerData::new(
                #inputs, #output, false, #abi, false, #lifetimes,
            )));
            let function = leaked_expression(quote!(#facade::__private::codegen_v4::expression::ExpressionData::FunctionPointer(#function)));
            vec![
                quote!(#facade::__private::codegen_v4::expression::GenericExpressionArgument::Type(#function)),
            ]
        }
        PathArgumentsIr::AngleBracketed(values) => values
            .iter()
            .filter_map(|value| match value {
                PathArgumentIr::Lifetime(value) => {
                    let value = lifetime_expression(value, span, facade);
                    Some(quote!(#facade::__private::codegen_v4::expression::GenericExpressionArgument::Lifetime(#value)))
                }
                PathArgumentIr::Type(value) => {
                    if let TypeKindIr::Path(path) = &value.kind
                        && path.qualified_self.is_none()
                        && path.segments.len() == 1
                        && matches!(path.segments[0].arguments, PathArgumentsIr::None)
                        && environment.is_const_parameter(&path.segments[0].name)
                    {
                        let name = LitStr::new(&path.segments[0].name, value.span);
                        return Some(quote!(#facade::__private::codegen_v4::expression::GenericExpressionArgument::Const(
                            #facade::__private::codegen_v4::expression::ConstExpression::Parameter(#name)
                        )));
                    }
                    let value = type_expression(value, environment, facade);
                    Some(quote!(#facade::__private::codegen_v4::expression::GenericExpressionArgument::Type(#value)))
                }
                PathArgumentIr::Const(value) => {
                    let value = const_expression_value(value, environment, facade);
                    Some(quote!(#facade::__private::codegen_v4::expression::GenericExpressionArgument::Const(#value)))
                }
                PathArgumentIr::AssociatedType { name, ty } => {
                    let name = LitStr::new(name, span);
                    let value = type_expression(ty, environment, facade);
                    Some(quote!(#facade::__private::codegen_v4::expression::GenericExpressionArgument::AssociatedType {
                        name: #name, value: #value,
                    }))
                }
                PathArgumentIr::Constraint { name, bounds } => {
                    let name = LitStr::new(name, span);
                    let bounds = bound_predicates(bounds, environment, facade, span);
                    let bounds = leak_slice(bounds);
                    Some(quote!(#facade::__private::codegen_v4::expression::GenericExpressionArgument::AssociatedTypeBound {
                        name: #name, bounds: #bounds,
                    }))
                }
                _ => None,
            })
            .collect(),
    }
}

/// Materializes direct external-supertrait arguments at the trait hook's
/// concrete instance.
///
/// # Parameters
///
/// - `arguments`: Parsed arguments from the direct supertrait path.
/// - `declaration`: Trait declaration whose generic scope supplies concrete
///   types.
/// - `facade`: Runtime facade path used in generated expressions.
///
/// # Returns
///
/// Returns runtime generic arguments with direct trait parameters materialized.
#[must_use]
pub(super) fn external_supertrait_arguments(
    arguments: &PathArgumentsIr,
    declaration: &TraitDeclarationIr,
    facade: &TokenStream,
) -> Vec<TokenStream> {
    let environment = GenericEnvironment::from_generics(&declaration.generics);
    let PathArgumentsIr::AngleBracketed(values) = arguments else {
        return path_arguments(arguments, &environment, facade, declaration.span);
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
                    let expression = leaked_expression(quote!(#facade::__private::codegen_v4::expression::ExpressionData::Named {
                        path: std::any::type_name::<#identifier>(), definition: None, arguments: &[],
                    }));
                    Some(quote!(#facade::__private::codegen_v4::expression::GenericExpressionArgument::Type(#expression)))
                } else {
                    path_arguments(
                        &PathArgumentsIr::AngleBracketed(vec![PathArgumentIr::Type(value.clone())]),
                        &environment,
                        facade,
                        declaration.span,
                    )
                    .into_iter()
                    .next()
                }
            }
            value => path_arguments(
                &PathArgumentsIr::AngleBracketed(vec![value.clone()]),
                &environment,
                facade,
                declaration.span,
            )
            .into_iter()
            .next(),
        })
        .collect()
}

/// Converts trait-object or opaque-type bounds into runtime predicates.
///
/// # Parameters
///
/// - `bounds`: Parsed trait and lifetime bounds.
/// - `environment`: Generic names visible at this metadata site.
/// - `facade`: Runtime facade path used in generated expressions.
/// - `span`: Source span used for lifetime expressions.
///
/// # Returns
///
/// Returns supported trait and outlives predicates in source order.
fn bound_predicates(
    bounds: &[GenericBoundIr],
    environment: &GenericEnvironment,
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
                let path = trait_expression_data(path, lifetimes, environment, facade, span);
                let modifier = trait_bound_modifier(*modifier, facade);
                let self_type = leaked_expression(
                    quote!(#facade::__private::codegen_v4::expression::ExpressionData::SelfType),
                );
                Some(
                    quote!(#facade::__private::codegen_v4::expression::PredicateData::TraitBound {
                        ty: #self_type,
                        trait_: #path, modifier: #modifier,
                    }),
                )
            }
            GenericBoundIr::Lifetime(value) => {
                let value = lifetime_expression(value, span, facade);
                let self_type = leaked_expression(
                    quote!(#facade::__private::codegen_v4::expression::ExpressionData::SelfType),
                );
                Some(
                    quote!(#facade::__private::codegen_v4::expression::PredicateData::TypeOutlives {
                        ty: #self_type,
                        lifetime: #value,
                    }),
                )
            }
            _ => None,
        })
        .collect()
}

/// Converts a parsed const expression into structural runtime metadata.
///
/// # Parameters
///
/// - `value`: Parsed const expression tokens.
/// - `environment`: Generic names visible at this metadata site.
/// - `facade`: Runtime facade path used in generated expressions.
///
/// # Returns
///
/// Returns structural tokens for supported expressions or a compile error for
/// unsupported syntax.
#[must_use]
fn const_expression_value(
    value: &TokenStream,
    environment: &GenericEnvironment,
    facade: &TokenStream,
) -> TokenStream {
    let source = value.to_string();
    if let Ok(identifier) = parse2::<Ident>(value.clone()) {
        let name = LitStr::new(&identifier.to_string(), identifier.span());
        if environment.is_const_parameter(&identifier.to_string()) {
            return quote!(#facade::__private::codegen_v4::expression::ConstExpression::Parameter(#name));
        }
        return quote!(#facade::__private::codegen_v4::expression::ConstExpression::Path(#name));
    }
    if let Ok(value) = parse2::<Lit>(value.clone()) {
        match value {
            Lit::Bool(value) => {
                let value = value.value;
                return quote!(#facade::__private::codegen_v4::expression::ConstExpression::Boolean(#value));
            }
            Lit::Char(value) => {
                let value = value.value();
                return quote!(#facade::__private::codegen_v4::expression::ConstExpression::Character(#value));
            }
            Lit::Int(value) => {
                if let Ok(value) = value.base10_parse::<u128>() {
                    return quote!(#facade::__private::codegen_v4::expression::ConstExpression::UnsignedInteger(#value));
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
        return quote!(#facade::__private::codegen_v4::expression::ConstExpression::SignedInteger(#value));
    }
    if let Ok(Expr::Path(path)) = parse2::<Expr>(value.clone()) {
        let segments = path
            .path
            .segments
            .iter()
            .map(|segment| LitStr::new(&segment.ident.to_string(), segment.ident.span()));
        let path = LitStr::new(
            &segments
                .map(|segment| segment.value())
                .collect::<Vec<_>>()
                .join("::"),
            Span::call_site(),
        );
        return quote!(#facade::__private::codegen_v4::expression::ConstExpression::Path(#path));
    }
    let source = LitStr::new(&source, Span::call_site());
    quote!(compile_error!(
        concat!("unsupported const expression in #[reflect] trait: ", #source)
    ))
}

/// Converts the literal const-default subset that has a structural runtime
/// representation.
///
/// # Parameters
///
/// - `value`: Const default tokens from the trait declaration.
/// - `environment`: Generic names visible at the default's metadata site.
/// - `facade`: Runtime facade path used in generated expressions.
///
/// # Returns
///
/// Returns tokens for `Some(const expression)`, or compile-error tokens for
/// unsupported defaults.
#[must_use]
pub(super) fn const_expression(
    value: &TokenStream,
    environment: &GenericEnvironment,
    facade: &TokenStream,
) -> TokenStream {
    let expression = parse2::<Expr>(value.clone());
    let expression = match expression {
        Ok(expression) => expression,
        Err(_) => return unsupported_const_default(value),
    };
    match expression {
        Expr::Lit(expression) => match expression.lit {
            Lit::Bool(value) => {
                let value = value.value;
                quote!(Some(#facade::__private::codegen_v4::expression::ConstExpression::Boolean(#value)))
            }
            Lit::Char(value) => {
                let value = value.value();
                quote!(Some(#facade::__private::codegen_v4::expression::ConstExpression::Character(#value)))
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
        Expr::Path(path) if path.path.segments.len() == 1 => {
            let identifier = path.path.segments[0].ident.to_string();
            let name = LitStr::new(&identifier, path.path.segments[0].ident.span());
            if environment.is_const_parameter(&identifier) {
                quote!(Some(#facade::__private::codegen_v4::expression::ConstExpression::Parameter(#name)))
            } else {
                quote!(Some(#facade::__private::codegen_v4::expression::ConstExpression::Path(#name)))
            }
        }
        Expr::Path(path) => {
            let segments = path
                .path
                .segments
                .iter()
                .map(|segment| LitStr::new(&segment.ident.to_string(), segment.ident.span()));
            let path = LitStr::new(
                &segments
                    .map(|segment| segment.value())
                    .collect::<Vec<_>>()
                    .join("::"),
                Span::call_site(),
            );
            quote!(Some(#facade::__private::codegen_v4::expression::ConstExpression::Path(#path)))
        }
        _ => unsupported_const_default(value),
    }
}

/// Converts an integer literal without relying on its whitespace-normalized
/// token rendering.
///
/// # Parameters
///
/// - `value`: Parsed integer literal, including any suffix.
/// - `negative`: Whether unary negation applies to the literal.
/// - `facade`: Runtime facade path used in generated expressions.
///
/// # Returns
///
/// Returns signed or unsigned structural tokens when the value fits, or `None`
/// when parsing overflows.
#[must_use]
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
        Some(
            quote!(Some(#facade::__private::codegen_v4::expression::ConstExpression::SignedInteger(#value))),
        )
    } else {
        let value = value.base10_parse::<u128>().ok()?;
        Some(
            quote!(Some(#facade::__private::codegen_v4::expression::ConstExpression::UnsignedInteger(#value))),
        )
    }
}

/// Emits a deterministic compile error for const defaults without a runtime
/// structural value.
///
/// # Parameters
///
/// - `value`: Unsupported source expression retained for the diagnostic.
///
/// # Returns
///
/// Returns tokens that produce a compile-time error at the user's invocation.
#[must_use]
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
    use syn::Type;
    use syn::parse_str;

    use super::const_expression;
    use super::const_expression_value;
    use super::type_expression;
    use crate::expand::generic_environment::GenericEnvironment;
    use crate::parse::convert_type;

    /// Converts one Rust type into generated structural metadata text.
    fn render_type(source: &str, environment: &GenericEnvironment) -> String {
        let ty = parse_str::<Type>(source).expect("test type must parse");
        type_expression(&convert_type(&ty), environment, &quote!(qubit_reflect)).to_string()
    }

    /// Verifies parameter classification comes only from the explicit scope.
    #[test]
    fn test_type_expression_uses_explicit_generic_environment() {
        let environment = GenericEnvironment::new().with_type_parameter("item");

        assert!(
            render_type("item", &environment).contains("ParameterName"),
            "generated expression should contain parameter"
        );
        assert!(
            render_type("HTTP", &environment).contains("Named"),
            "generated expression should contain Named"
        );
        assert!(
            render_type("item::Output", &environment).contains("Associated"),
            "generated expression should contain Associated"
        );
        assert!(
            render_type("Self::Output", &environment).contains("Associated"),
            "generated expression should contain Associated"
        );
    }

    #[test]
    fn test_type_expression_emits_codegen_v4_for_unreflected_type() {
        let ty = parse_str::<Type>("Unreflected<'a, T, 3>").expect("test type must parse");
        let environment = GenericEnvironment::new().with_type_parameter("T");
        let generated =
            type_expression(&convert_type(&ty), &environment, &quote!(qubit_reflect)).to_string();

        assert!(
            generated.contains("codegen_v4"),
            "type expressions must use the v4 protocol"
        );
        assert!(
            !generated.contains("codegen_v3"),
            "type expressions must not use the v3 protocol"
        );
        assert!(
            generated.contains("Unreflected"),
            "unreflected types remain represented structurally"
        );
    }

    /// Verifies generic arguments remain attached to every path segment.
    #[test]
    fn test_type_expression_preserves_arguments_on_each_path_segment() {
        let rendered = render_type("Outer<u8>::Inner<u16>", &GenericEnvironment::new());

        assert!(
            rendered.contains("ExpressionData :: Named"),
            "paths should emit a named expression node"
        );
        assert!(
            rendered.contains("u8"),
            "generated expression should contain u8"
        );
        assert!(
            rendered.contains("u16"),
            "generated expression should contain u16"
        );
    }

    /// Verifies const parameters passed as type-shaped path arguments retain
    /// their const kind in generic impl metadata.
    #[test]
    fn test_type_expression_classifies_const_generic_path_arguments() {
        let environment = GenericEnvironment::new()
            .with_type_parameter("item")
            .with_const_parameter("capacity");
        let rendered = render_type("Service<item, capacity>", &environment);

        assert!(
            rendered.contains("GenericExpressionArgument :: Const"),
            "const generic argument should be emitted as a const"
        );
        assert!(
            rendered.contains("ConstExpression :: Parameter (\"capacity\")"),
            "const parameter should retain its symbolic identity"
        );
    }

    /// Verifies qualified projections and associated constraints stay
    /// structural.
    #[test]
    fn test_type_expression_preserves_qualified_projection_and_constraint() {
        let environment = GenericEnvironment::new().with_type_parameter("item");
        let projection = render_type("<item as Trait<u8>>::Output", &environment);
        let constraint = render_type("Iterator<Item: Display>", &environment);

        assert!(
            projection.contains("AssociatedType"),
            "generated expression should contain AssociatedType"
        );
        assert!(
            projection.contains("Trait"),
            "generated expression should contain Trait"
        );
        assert!(
            projection.contains("u8"),
            "generated expression should contain u8"
        );
        assert!(
            constraint.contains("AssociatedTypeBound"),
            "generated expression should contain AssociatedTypeBound"
        );
        assert!(
            constraint.contains("Display"),
            "generated expression should contain Display"
        );
    }

    #[test]
    fn test_type_expression_retains_higher_ranked_trait_lifetimes() {
        let rendered = render_type("dyn for<'a> Trait<'a>", &GenericEnvironment::new());
        assert!(rendered.contains("TraitExpressionData :: new"));
        assert!(rendered.contains("LifetimeExpression :: Named (\"a\")"));
    }

    /// Verifies const parameters and const item paths remain distinct.
    #[test]
    fn test_const_expression_uses_explicit_generic_environment() {
        let environment = GenericEnvironment::new().with_const_parameter("limit");
        let parameter =
            const_expression_value(&quote!(limit), &environment, &quote!(qubit_reflect));
        let item = const_expression_value(
            &quote!(limits::DEFAULT),
            &environment,
            &quote!(qubit_reflect),
        );

        assert!(
            parameter
                .to_string()
                .contains("ConstExpression :: Parameter"),
            "generated expression should contain const parameter"
        );
        assert!(
            item.to_string().contains("ConstExpression :: Path"),
            "generated expression should contain const path"
        );
        assert!(
            item.to_string().contains("DEFAULT"),
            "generated expression should contain DEFAULT"
        );
    }

    /// Verifies that literals retain their structural value rather than their
    /// token spelling.
    #[test]
    fn test_const_default_accepts_signed_suffixed_and_escaped_literals() {
        let facade = quote!(qubit_reflect);
        let environment = GenericEnvironment::new();
        let signed = const_expression(&quote!(-7i16), &environment, &facade).to_string();
        let unsigned = const_expression(&quote!(42u8), &environment, &facade).to_string();
        let escaped = const_expression(&quote!('\n'), &environment, &facade).to_string();

        assert!(
            signed.contains("SignedInteger"),
            "generated expression should contain SignedInteger"
        );
        assert!(
            signed.contains("- 7i128"),
            "generated expression should contain - 7i128"
        );
        assert!(
            unsigned.contains("UnsignedInteger"),
            "generated expression should contain UnsignedInteger"
        );
        assert!(
            unsigned.contains("42u128"),
            "generated expression should contain 42u128"
        );
        assert!(
            escaped.contains("Character"),
            "generated expression should contain Character"
        );
        assert!(
            escaped.contains("'\\n'"),
            "generated expression should contain '\\n'"
        );
    }

    /// Verifies that symbolic defaults retain const-item path identity.
    #[test]
    fn test_const_default_preserves_const_item_path() {
        let value: TokenStream = quote!(DEFAULT_LIMIT);
        let rendered = const_expression(&value, &GenericEnvironment::new(), &quote!(qubit_reflect))
            .to_string();

        assert!(
            rendered.contains("ConstExpression :: Path"),
            "generated expression should contain const path"
        );
        assert!(
            rendered.contains("DEFAULT_LIMIT"),
            "generated expression should contain DEFAULT_LIMIT"
        );
    }
}
