// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Expansion of distributed registration fragments for `#[reflect_impl]`.

mod internal;
mod invocation_adapter;
mod specialization_codegen;

use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use specialization_codegen::typed_extension_receiver_type;

use crate::expand::ExpansionContext;
use crate::expand::generic_environment::GenericEnvironment;
use crate::ir::ImplDeclarationIr;
use crate::ir::MethodIr;
use crate::ir::ParameterPatternKindIr;
use crate::ir::ReceiverKindIr;
use crate::ir::ReturnTypeIr;
use crate::ir::TypeKindIr;
use crate::ir::VisibilityIr;

/// Emits typed codegen_v4 metadata for one implementation method.
fn method_metadata(
    method: &MethodIr,
    environment: &GenericEnvironment,
    target: &TokenStream,
    namespace: &TokenStream,
    facade: &TokenStream,
    symbolic_target: bool,
    trait_impl: bool,
) -> TokenStream {
    let method_environment = environment.clone().with_generics(&method.generics);
    let query = method.name.to_string();
    let query = syn::LitStr::new(&query, method.span);
    let visibility = if trait_impl {
        quote!(#facade::__private::codegen_v4::invoke::MethodVisibility::Public)
    } else {
        match method.visibility {
            VisibilityIr::Public => {
                quote!(#facade::__private::codegen_v4::invoke::MethodVisibility::Public)
            }
            VisibilityIr::Restricted(_) => {
                quote!(#facade::__private::codegen_v4::invoke::MethodVisibility::Restricted)
            }
            VisibilityIr::Crate | VisibilityIr::Super | VisibilityIr::SelfValue | VisibilityIr::Inherited => {
                quote!(#facade::__private::codegen_v4::invoke::MethodVisibility::Private)
            }
        }
    };
    let receiver = match &method.receiver {
        Some(receiver_ir) => {
            let (receiver_ty, kind) = match receiver_ir.kind {
                ReceiverKindIr::Value => (target.clone(), quote!(Owned)),
                ReceiverKindIr::SharedReference => (target.clone(), quote!(Shared)),
                ReceiverKindIr::MutableReference => (target.clone(), quote!(Mutable)),
                ReceiverKindIr::Typed => {
                    match super::invocation::analysis::typed_pinned_receiver_mutable(receiver_ir) {
                        Some(true) => (target.clone(), quote!(PinnedMutable)),
                        Some(false) => (target.clone(), quote!(PinnedShared)),
                        None => (receiver_ir.ty.tokens.clone(), quote!(Owned)),
                    }
                }
            };
            let receiver_kind = quote!(#facade::__private::codegen_v4::invoke::ReceiverKind::#kind);
            if symbolic_target {
                let expression = if matches!(receiver_ir.kind, ReceiverKindIr::Typed) {
                    super::traits::type_expression(&receiver_ir.ty, &method_environment, facade)
                } else {
                    target.clone()
                };
                quote!(Some(#facade::__private::codegen_v4::invoke::ReceiverDescriptor::trait_self(
                    #receiver_kind,
                    #expression,
                )))
            } else {
                let receiver_descriptor = quote!(#facade::__private::codegen_v4::invoke::ReceiverDescriptor::for_type::<#receiver_ty>(#receiver_kind));
                if matches!(receiver_ir.kind, ReceiverKindIr::Typed) {
                    let expression = super::traits::type_expression(&receiver_ir.ty, &method_environment, facade);
                    quote!(Some(#receiver_descriptor.with_expression(#expression)))
                } else {
                    quote!(Some(#receiver_descriptor))
                }
            }
        }
        None => quote!(None),
    };
    let parameters = method.parameters.iter().map(|parameter| {
        let pattern = match parameter.pattern.kind {
            ParameterPatternKindIr::Identifier => {
                let name = syn::LitStr::new(parameter.name.as_deref().unwrap_or(""), parameter.span);
                quote!(#facade::__private::codegen_v4::invoke::ParameterPattern::Identifier(#name))
            }
            ParameterPatternKindIr::Wildcard => {
                quote!(#facade::__private::codegen_v4::invoke::ParameterPattern::Wildcard)
            }
            ParameterPatternKindIr::Destructure => {
                let pattern = syn::LitStr::new(&parameter.pattern.source, parameter.span);
                quote!(#facade::__private::codegen_v4::invoke::ParameterPattern::DestructurePattern(#pattern))
            }
        };
        let passing = match &parameter.ty.kind {
            TypeKindIr::Reference { mutable: true, .. } => {
                quote!(#facade::__private::codegen_v4::invoke::PassingMode::Mutable)
            }
            TypeKindIr::Reference { .. } => {
                quote!(#facade::__private::codegen_v4::invoke::PassingMode::Shared)
            }
            _ => quote!(#facade::__private::codegen_v4::invoke::PassingMode::Owned),
        };
        let expression = super::traits::type_expression(&parameter.ty, &method_environment, facade);
        let concrete_type = parameter_concrete_type(&parameter.ty, &method_environment)
            .map(|ty| {
                let ty = &ty.tokens;
                quote!(.with_concrete_type::<#ty>())
            })
            .unwrap_or_default();
        let index = parameter.index;
        quote!(#facade::__private::codegen_v4::invoke::ParameterMetadata::new(#index, #pattern, #passing, #expression) #concrete_type)
    });
    let output = match &method.return_type {
        ReturnTypeIr::Unit => {
            quote!(#facade::__private::codegen_v4::invoke::ReturnMetadata::unit())
        }
        ReturnTypeIr::Type(ty) => {
            let expression = super::traits::type_expression(ty, &method_environment, facade);
            let kind = if method.qualifiers.is_async {
                quote!(Future)
            } else {
                match ty.kind {
                    TypeKindIr::Never => quote!(Never),
                    TypeKindIr::Reference { mutable: true, .. } => quote!(Mutable),
                    TypeKindIr::Reference { .. } => quote!(Shared),
                    TypeKindIr::ImplTrait { .. } => quote!(Opaque),
                    _ => quote!(Owned),
                }
            };
            quote!(#facade::__private::codegen_v4::invoke::ReturnMetadata::new(
                #facade::__private::codegen_v4::invoke::ReturnKind::#kind,
                #expression,
            ))
        }
    };
    let qualifiers = &method.qualifiers;
    let generics = super::traits::generic_definition(&method.generics, method.span, facade);
    let is_const = qualifiers.is_const;
    let is_async = qualifiers.is_async;
    let is_unsafe = qualifiers.is_unsafe;
    let is_variadic = qualifiers.is_variadic;
    let abi = qualifiers
        .abi
        .as_deref()
        .map(|abi| crate::expand::expression_codegen::function_abi(Some(abi), method.span, facade));
    let abi = abi.map_or_else(|| quote!(None), |abi| quote!(Some(#abi)));
    let has_default = method.has_default;
    let is_generic = !method.generics.params.is_empty();
    quote! {
        #facade::__private::codegen_v4::invoke::MethodMetadata::from_spec(
            #facade::__private::codegen_v4::invoke::MethodMetadataSpec::new(
                #query,
                #namespace,
                #visibility,
                #receiver,
                ::std::boxed::Box::leak(::std::vec![#(#parameters),*].into_boxed_slice()),
                ::std::boxed::Box::leak(::std::boxed::Box::new(#output)),
                #facade::__private::codegen_v4::invoke::MethodQualifiers::new(
                    #is_const, #is_async, #is_unsafe, #is_variadic,
                ).with_abi(#abi),
                ::std::boxed::Box::leak(::std::boxed::Box::new(#generics)),
                #is_generic,
                #has_default,
            ),
        )
    }
}

fn parameter_concrete_type<'a>(
    parameter_type: &'a crate::ir::TypeIr,
    environment: &GenericEnvironment,
) -> Option<&'a crate::ir::TypeIr> {
    let concrete_type = match &parameter_type.kind {
        TypeKindIr::Reference { element, .. } => element.as_ref(),
        _ => parameter_type,
    };
    is_static_concrete_type(concrete_type, environment).then_some(concrete_type)
}

fn is_static_concrete_type(ty: &crate::ir::TypeIr, environment: &GenericEnvironment) -> bool {
    use crate::ir::PathArgumentIr;
    use crate::ir::PathArgumentsIr;

    match &ty.kind {
        TypeKindIr::Path(path) => {
            if path
                .qualified_self
                .as_ref()
                .is_some_and(|self_type| !is_static_concrete_type(&self_type.ty, environment))
            {
                return false;
            }
            path.segments.iter().all(|segment| {
                !environment.is_type_parameter(&segment.name)
                    && match &segment.arguments {
                        PathArgumentsIr::None => true,
                        PathArgumentsIr::AngleBracketed(arguments) => arguments.iter().all(|argument| match argument {
                            PathArgumentIr::Lifetime(lifetime) => lifetime == "'static",
                            PathArgumentIr::Type(ty) => is_static_concrete_type(ty, environment),
                            PathArgumentIr::Const(value) => !contains_generic_const(value, environment),
                            PathArgumentIr::AssociatedType { .. }
                            | PathArgumentIr::AssociatedConst { .. }
                            | PathArgumentIr::Constraint { .. }
                            | PathArgumentIr::Other(_) => false,
                        }),
                        PathArgumentsIr::Parenthesized { inputs, output } => {
                            inputs.iter().all(|input| is_static_concrete_type(input, environment))
                                && output
                                    .as_ref()
                                    .is_none_or(|output| is_static_concrete_type(output, environment))
                        }
                    }
            })
        }
        TypeKindIr::Tuple(elements) => elements
            .iter()
            .all(|element| is_static_concrete_type(element, environment)),
        TypeKindIr::Array { element, length } => {
            is_static_concrete_type(element, environment) && !contains_generic_const(length, environment)
        }
        TypeKindIr::Pointer { element, .. } => is_static_concrete_type(element, environment),
        TypeKindIr::Reference { .. }
        | TypeKindIr::Slice(_)
        | TypeKindIr::BareFunction { .. }
        | TypeKindIr::TraitObject { .. }
        | TypeKindIr::ImplTrait { .. }
        | TypeKindIr::Never
        | TypeKindIr::Infer
        | TypeKindIr::Macro
        | TypeKindIr::Other => false,
    }
}

fn contains_generic_const(tokens: &TokenStream, environment: &GenericEnvironment) -> bool {
    tokens
        .to_string()
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .any(|identifier| environment.is_const_parameter(identifier))
}

/// Expands an impl unchanged and submits a lazily-built implementation
/// fragment.
///
/// The descriptor graph is deliberately constructed during registry
/// initialization, not from an inventory constructor. This keeps user code out
/// of linker startup and uses the T12 registration protocol exclusively.
///
/// # Parameters
///
/// - `declaration`: Validated inherent or trait implementation.
/// - `context`: Expansion context containing the runtime facade.
///
/// # Returns
///
/// Returns the retained impl and generated registration metadata.
#[must_use]
pub(crate) fn expand_impl(declaration: ImplDeclarationIr, context: &ExpansionContext) -> TokenStream {
    if declaration.reflected_trait.is_some()
        && let Some(method) = declaration
            .methods
            .iter()
            .chain(&declaration.skipped_methods)
            .find(|method| method.name == "__qbr_trait_application_metadata")
    {
        return syn::Error::new(
            method.span,
            "`__qbr_trait_application_metadata` is reserved by `#[Reflect]` traits",
        )
        .into_compile_error();
    }
    if !declaration.generics.params.is_empty() {
        let retained = declaration.retained_tokens.clone();
        let definition = expand_generic_impl_definition(&declaration, context.facade(), context);
        quote!(#retained #definition)
    } else {
        expand_concrete_impl(
            declaration,
            quote!(::std::vec![]),
            None,
            None,
            Vec::new(),
            context.facade(),
            context,
        )
    }
}

/// Emits one declaration-level registration for a generic inherent impl.
///
/// This payload deliberately has no `TypeId`: it describes the symbolic impl
/// declaration and cannot enter a concrete type's effective view.
///
/// # Parameters
///
/// - `declaration`: Generic impl whose symbolic definition is registered.
/// - `facade`: Runtime facade path used in generated references.
/// - `context`: Expansion context used to fingerprint the declaration.
///
/// # Returns
///
/// Returns generated definition-provider and registration tokens.
#[must_use]
fn expand_generic_impl_definition(
    declaration: &ImplDeclarationIr,
    facade: &TokenStream,
    context: &ExpansionContext,
) -> TokenStream {
    let fingerprint = context.fingerprint(&declaration.retained_tokens.to_string());
    let module = generic_impl_definition_module(declaration);
    let marker = format_ident!("__QubitReflectImplDef_{fingerprint:016x}");
    let provider = format_ident!("__qubit_reflect_impl_definition_provider_{fingerprint:016x}");
    let codegen = quote!(#facade::__private::codegen_v4);
    let environment = GenericEnvironment::from_generics(&declaration.generics);
    let target = super::traits::type_expression(&declaration.target_type, &environment, facade);
    let generics = super::traits::generic_definition(&declaration.generics, declaration.span, facade);
    let target_source = syn::LitStr::new(&declaration.target_type.source, declaration.span);
    let namespace = if let Some(path) = &declaration.trait_path {
        let trait_name = if let Some(provider) = trait_definition_provider(declaration) {
            quote!(#provider().path())
        } else {
            let source = syn::LitStr::new(&trait_source_path(path), declaration.span);
            quote!(#source)
        };
        quote!(#codegen::invoke::MethodNamespaceMetadata::Trait {
            trait_: #trait_name,
            implementation: #target_source,
        })
    } else {
        quote!(#codegen::invoke::MethodNamespaceMetadata::Inherent {
            implementation: #target_source,
        })
    };
    let methods: Vec<_> = declaration
        .methods
        .iter()
        .map(|method| {
            method_metadata(
                method,
                &environment,
                &target,
                &namespace,
                facade,
                true,
                declaration.trait_path.is_some(),
            )
        })
        .collect();
    let associated_types: Vec<_> = declaration
        .associated_types
        .iter()
        .map(|item| {
            let name = syn::LitStr::new(&item.name.to_string(), item.span);
            let value = item
                .value
                .as_ref()
                .map(|value| super::traits::type_expression(value, &environment, facade));
            let value = value.map_or_else(|| quote!(None), |value| quote!(Some(#value)));
            quote!(#codegen::traits::associated_type(
                #name,
                #codegen::traits::TraitVisibility::Public,
                &[],
                #value,
            ))
        })
        .collect();
    let associated_consts: Vec<_> = declaration
        .associated_consts
        .iter()
        .map(|item| {
            let name = syn::LitStr::new(&item.name.to_string(), item.span);
            let ty = super::traits::type_expression(&item.ty, &environment, facade);
            let value = item
                .value
                .as_ref()
                .map(|value| super::expression_codegen::const_expression(value, &environment, facade));
            let value = value.unwrap_or_else(|| quote!(None));
            quote!(#codegen::traits::associated_const(
                #name,
                #codegen::traits::TraitVisibility::Public,
                #ty,
                #value,
            ))
        })
        .collect();
    let trait_definition_provider = trait_definition_provider(declaration);
    let implemented_trait = match &declaration.trait_path {
        Some(path) => {
            let name = syn::LitStr::new(&path.source, path.span);
            let arguments = path.segments.iter().flat_map(|segment| {
                super::expression_codegen::path_arguments(&segment.arguments, &environment, facade, path.span)
            });
            let expression = quote!(::std::boxed::Box::leak(::std::boxed::Box::new(
                #codegen::expression::TraitExpressionData::new(
                    #name,
                    ::std::boxed::Box::leak(::std::vec![#(#arguments),*].into_boxed_slice()),
                    &[],
                ),
            )));
            let metadata = if let Some(provider) = &trait_definition_provider {
                quote!(#codegen::traits::trait_expression_for(#expression, #provider()))
            } else {
                quote!(#codegen::traits::unresolved_trait_expression(#expression))
            };
            quote!(Some(::std::boxed::Box::leak(::std::boxed::Box::new(#metadata))))
        }
        None => quote!(None),
    };
    let kind = if declaration.trait_path.is_some() {
        quote!(#codegen::impls::Kind::Trait)
    } else {
        quote!(#codegen::impls::Kind::Inherent)
    };
    quote! {
        #[doc(hidden)]
        mod #module {
            use super::*;

            #[doc(hidden)]
            pub struct #marker;

            fn #provider() -> &'static #codegen::impls::ImplDefinitionSpec<#marker> {
                static VALUE: ::std::sync::OnceLock<#codegen::impls::ImplDefinitionSpec<#marker>> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| {
                    let generics = #generics;
                    let methods = ::std::boxed::Box::leak(::std::vec![#(#methods),*].into_boxed_slice());
                    let associated_types = ::std::boxed::Box::leak(::std::vec![#(#associated_types),*].into_boxed_slice());
                    let associated_consts = ::std::boxed::Box::leak(::std::vec![#(#associated_consts),*].into_boxed_slice());
                    #codegen::impls::ImplDefinitionSpec::new(
                        #kind,
                        #target,
                        #implemented_trait,
                        ::std::boxed::Box::leak(::std::boxed::Box::new(generics)),
                        #codegen::impls::Qualifiers::new(false),
                        methods,
                        associated_types,
                        associated_consts,
                    )
                })
            }

            #codegen::impls::submit_impl_definition!(#marker, #provider);
        }
    }
}

/// Returns the stable generated module name owning one generic impl
/// definition.
///
/// # Parameters
///
/// - `declaration`: Generic impl whose source location and tokens identify the
///   module.
///
/// # Returns
///
/// Returns the deterministic hidden module identifier.
#[must_use]
fn generic_impl_definition_module(declaration: &ImplDeclarationIr) -> Ident {
    let fingerprint = super::context::fingerprint(&declaration.retained_tokens.to_string());
    let location = declaration.span.start();
    format_ident!(
        "__qubit_reflect_impl_definition_{fingerprint:x}_{}_{}",
        location.line,
        location.column,
    )
}

/// Expands one concrete implementation registration fragment.
///
/// # Parameters
///
/// - `declaration`: Concrete impl declaration to emit.
/// - `impl_arguments`: Runtime generic arguments retained for specialization.
/// - `shared_definition`: Optional shared generic impl-definition provider.
/// - `applicability_witness`: Optional compile-time specialization proof.
/// - `specialized_associated_type_resolver_arms`: Resolver arms for specialized
///   associated types.
/// - `facade`: Runtime facade path used in generated references.
/// - `context`: Expansion context used to analyze invocation support.
///
/// # Returns
///
/// Returns the retained impl and generated concrete registration module.
#[must_use]
fn expand_concrete_impl(
    declaration: ImplDeclarationIr,
    _impl_arguments: TokenStream,
    _shared_definition: Option<TokenStream>,
    _applicability_witness: Option<TokenStream>,
    _specialized_associated_type_resolver_arms: Vec<TokenStream>,
    facade: &TokenStream,
    context: &ExpansionContext,
) -> TokenStream {
    let facade = facade.clone();
    let facade_identifier = syn::parse2::<syn::Path>(facade.clone())
        .expect("the resolved runtime facade is a Rust path")
        .segments
        .last()
        .expect("the resolved runtime facade path is non-empty")
        .ident
        .clone();
    let codegen = quote!(#facade::__private::codegen_v4);
    let target = declaration.target_type.tokens.clone();
    let environment = GenericEnvironment::from_generics(&declaration.generics);
    let fingerprint = context.fingerprint(&declaration.retained_tokens.to_string());
    let location = declaration.span.start();
    let module = format_ident!(
        "__qubit_reflect_impl_{fingerprint:x}_{}_{}",
        location.line,
        location.column,
    );
    let marker = format_ident!("__QubitReflectImplDef_{fingerprint:016x}");
    let target_expression = super::traits::type_expression(&declaration.target_type, &environment, &facade);
    let generic_definition = super::traits::generic_definition(&declaration.generics, declaration.span, &facade);
    let implementation_name = syn::LitStr::new(&declaration.target_type.source, declaration.span);
    let namespace = match &declaration.trait_path {
        Some(path) => {
            let trait_name = if let Some(provider) = trait_definition_provider(&declaration) {
                quote!(#provider().path())
            } else {
                let source = syn::LitStr::new(&trait_source_path(path), declaration.span);
                quote!(#source)
            };
            quote!(#codegen::invoke::MethodNamespaceMetadata::Trait {
                trait_: #trait_name,
                implementation: #implementation_name,
            })
        }
        None => quote!(#codegen::invoke::MethodNamespaceMetadata::Inherent {
            implementation: #implementation_name,
        }),
    };
    let methods: Vec<_> = declaration
        .methods
        .iter()
        .map(|method| {
            method_metadata(
                method,
                &environment,
                &target,
                &namespace,
                &facade,
                false,
                declaration.trait_path.is_some(),
            )
        })
        .collect();
    let associated_types: Vec<_> = declaration
        .associated_types
        .iter()
        .map(|item| {
            let name = syn::LitStr::new(&item.name.to_string(), item.span);
            let value = item
                .value
                .as_ref()
                .map(|value| super::traits::type_expression(value, &environment, &facade));
            let value = value.map_or_else(|| quote!(None), |value| quote!(Some(#value)));
            quote!(#codegen::traits::associated_type(
                #name,
                #codegen::traits::TraitVisibility::Public,
                &[],
                #value,
            ))
        })
        .collect();
    let associated_consts: Vec<_> = declaration
        .associated_consts
        .iter()
        .map(|item| {
            let name = syn::LitStr::new(&item.name.to_string(), item.span);
            let ty = super::traits::type_expression(&item.ty, &environment, &facade);
            let value = item
                .value
                .as_ref()
                .map(|value| super::expression_codegen::const_expression(value, &environment, &facade));
            let value = value.unwrap_or_else(|| quote!(None));
            quote!(#codegen::traits::associated_const(
                #name,
                #codegen::traits::TraitVisibility::Public,
                #ty,
                #value,
            ))
        })
        .collect();
    let kind = if declaration.trait_path.is_some() {
        quote!(#codegen::impls::Kind::Trait)
    } else {
        quote!(#codegen::impls::Kind::Inherent)
    };
    let reflected_trait_path = declaration.reflected_trait.as_ref();
    let trait_definition_provider = trait_definition_provider(&declaration);
    let trait_marker_path = reflected_trait_path.map(|path| {
        let marker_path = trait_provider_path(path, "");
        quote!(#marker_path)
    });
    let trait_definition_marker_path = reflected_trait_path
        .map(|path| {
            let marker_path = trait_provider_path(path, "");
            quote!(#marker_path)
        })
        .or_else(|| {
            declaration.external_trait.as_ref().map(|path| {
                let marker_path = path.tokens.clone();
                quote!(#marker_path)
            })
        });
    let trait_default_methods_callsite = trait_marker_path.as_ref().map(|marker_path| {
        let mut callsite_path: syn::Path =
            syn::parse2(marker_path.clone()).expect("validated trait marker path parses");
        let last = callsite_path
            .segments
            .last_mut()
            .expect("validated trait path has a final segment");
        last.ident = format_ident!("{}_DEFAULT_METHODS_CALLSITE", last.ident);
        last.arguments = syn::PathArguments::None;
        quote!(#callsite_path)
    });
    let trait_default_methods_module = trait_marker_path.as_ref().map(|marker_path| {
        let mut module_path: syn::Path = syn::parse2(marker_path.clone()).expect("validated trait path parses");
        module_path.segments.pop();
        module_path.segments.pop_punct();
        module_path.leading_colon = None;
        if module_path.segments.is_empty() {
            syn::parse_quote!(super)
        } else {
            module_path
        }
    });
    let (trait_default_method_lifetimes, trait_default_method_types) = declaration
        .trait_path
        .as_ref()
        .and_then(|path| syn::parse2::<syn::Path>(path.tokens.clone()).ok())
        .and_then(|path| path.segments.last().map(|segment| segment.arguments.clone()))
        .and_then(|arguments| match arguments {
            syn::PathArguments::AngleBracketed(arguments) => {
                let mut lifetimes = Vec::new();
                let mut types = Vec::new();
                for argument in &arguments.args {
                    match argument {
                        syn::GenericArgument::Lifetime(lifetime) => lifetimes.push(quote!(#lifetime)),
                        syn::GenericArgument::Type(ty) => types.push(quote!(#ty)),
                        syn::GenericArgument::Const(value) => types.push(quote!(#value)),
                        _ => {}
                    }
                }
                Some((lifetimes, types))
            }
            _ => None,
        })
        .unwrap_or_default();
    let overridden_method_names = declaration
        .methods
        .iter()
        .chain(declaration.skipped_methods.iter())
        .map(|method| syn::LitStr::new(&method.name.to_string(), method.span));
    let default_methods = trait_marker_path.as_ref().map_or_else(
        || quote!(::std::vec::Vec::<#codegen::invoke::EffectiveMethod>::new().into_boxed_slice()),
        |_| {
            let callsite = trait_default_methods_callsite
                .as_ref()
                .expect("reflected trait marker has a default-method callsite");
            let module = trait_default_methods_module
                .as_ref()
                .expect("reflected trait marker has a module path");
            quote!(#callsite!(
                (#module),
                (#facade_identifier),
                #target,
                [#(#trait_default_method_lifetimes),*],
                [#(#trait_default_method_types),*],
                &[#(#overridden_method_names),*]
            ))
        },
    );
    let (trait_application_arguments, trait_argument_identity) = declaration
        .trait_path
        .as_ref()
        .and_then(|path| path.segments.last())
        .map(|segment| {
            let provider = trait_definition_provider.as_ref();
            // An external typed handle proves identity only. Its definition
            // intentionally has no generic declaration metadata, so retain
            // source argument identities without indexing that empty list.
            let generic_parameter_provider = declaration
                .reflected_trait
                .as_ref()
                .and(provider);
            let mut arguments = Vec::new();
            let mut identity = Vec::new();
            if let crate::ir::PathArgumentsIr::AngleBracketed(source_arguments) = &segment.arguments {
                for (index, argument) in source_arguments.iter().enumerate() {
                    match argument {
                        crate::ir::PathArgumentIr::Type(ty) => {
                            let concrete_type = ty.tokens.clone();
                            let expression = super::traits::type_expression(ty, &environment, &facade);
                            if let Some(provider) = generic_parameter_provider {
                                let parameter = quote!(#provider().generic_parameter_at(#index));
                                arguments.push(quote!(#codegen::expression::GenericArgumentMetadata::type_argument::<#concrete_type>(#parameter, #expression)));
                            } else {
                                arguments.push(quote!(#codegen::expression::GenericArgumentMetadata::positional_type_argument::<#concrete_type>(#expression)));
                            }
                            identity.push(quote!(#codegen::traits::ArgumentIdentity::type_argument::<#concrete_type>()));
                        }
                        crate::ir::PathArgumentIr::Const(value) => {
                            let value = super::expression_codegen::const_expression(value, &environment, &facade);
                            if let Some(provider) = generic_parameter_provider {
                                let parameter = quote!(#provider().generic_parameter_at(#index));
                                arguments.push(quote!(#codegen::expression::GenericArgumentMetadata::const_argument(
                                    #parameter,
                                    #value.expect("supported const trait arguments are available"),
                                )));
                            } else {
                                arguments.push(quote!(#codegen::expression::GenericArgumentMetadata::positional_const_argument(
                                    #value.expect("supported const trait arguments are available"),
                                )));
                            }
                            identity.push(quote!(#codegen::traits::ArgumentIdentity::const_argument(
                                #value.expect("supported const trait arguments are available")
                            )));
                        }
                        crate::ir::PathArgumentIr::Lifetime(_) => {}
                        _ => {}
                    }
                }
            }
            (arguments, identity)
        })
        .unwrap_or_default();
    let trait_application_provider = match (
        declaration.trait_path.as_ref(),
        trait_definition_provider.as_ref(),
        trait_definition_marker_path.as_ref(),
    ) {
        (Some(path), Some(provider), Some(trait_marker)) => {
            let trait_path = syn::LitStr::new(&trait_source_path(path), path.span);
            let source_trait_path: syn::Path =
                syn::parse2(path.tokens.clone()).expect("validated impl trait path parses");
            let application_hook = format_ident!("__qbr_trait_application_metadata");
            let application_metadata = if declaration.reflected_trait.is_some() {
                quote! {
                    let (arguments, identity, direct_supertraits, all_supertraits) =
                        <#target as #source_trait_path>::#application_hook();
                    let arguments = ::std::boxed::Box::leak(arguments);
                    let identity = ::std::boxed::Box::leak(identity);
                }
            } else {
                quote! {
                    let arguments = ::std::boxed::Box::leak(
                        ::std::vec![#(#trait_application_arguments),*].into_boxed_slice()
                    );
                    let identity = ::std::boxed::Box::leak(
                        ::std::vec![#(#trait_argument_identity),*].into_boxed_slice()
                    );
                    let direct_supertraits = &[];
                    let all_supertraits = &[];
                }
            };
            let supertrait_completeness = if declaration.reflected_trait.is_some() {
                quote!(#codegen::traits::supertrait_applications_complete(direct_supertraits))
            } else {
                quote!(#codegen::traits::SupertraitStatus::Incomplete)
            };
            let expression_args = path.segments.iter().flat_map(|segment| {
                super::expression_codegen::path_arguments(&segment.arguments, &environment, &facade, path.span)
            });
            Some(quote! {
                fn trait_application_provider() -> &'static #codegen::traits::TraitApplicationSpec<#target> {
                    static VALUE: ::std::sync::OnceLock<#codegen::traits::TraitApplicationSpec<#target>> = ::std::sync::OnceLock::new();
                    VALUE.get_or_init(|| {
                        let expression = ::std::boxed::Box::leak(::std::boxed::Box::new(
                            #codegen::expression::TraitExpressionData::new(
                                #trait_path,
                                ::std::boxed::Box::leak(::std::vec![#(#expression_args),*].into_boxed_slice()),
                                &[],
                            ),
                        ));
                        #application_metadata
                        #codegen::traits::TraitApplicationSpec::<#target>::for_target_with_source::<#trait_marker>(
                            #provider(),
                            expression,
                            arguments,
                            identity,
                            direct_supertraits,
                            all_supertraits,
                            #supertrait_completeness,
                        )
                    })
                }
            })
        }
        (Some(path), _, _) => {
            let trait_path = syn::LitStr::new(&trait_source_path(path), path.span);
            let expression_args = path.segments.iter().flat_map(|segment| {
                super::expression_codegen::path_arguments(&segment.arguments, &environment, &facade, path.span)
            });
            Some(quote! {
                fn trait_application_provider() -> &'static #codegen::traits::TraitApplicationSpec<#target> {
                    static VALUE: ::std::sync::OnceLock<#codegen::traits::TraitApplicationSpec<#target>> = ::std::sync::OnceLock::new();
                    VALUE.get_or_init(|| {
                        let expression = ::std::boxed::Box::leak(::std::boxed::Box::new(
                            #codegen::expression::TraitExpressionData::new(
                                #trait_path,
                                ::std::boxed::Box::leak(::std::vec![#(#expression_args),*].into_boxed_slice()),
                                &[],
                            ),
                        ));
                        let arguments = ::std::boxed::Box::leak(::std::vec![#(#trait_application_arguments),*].into_boxed_slice());
                        let identity = ::std::boxed::Box::leak(::std::vec![#(#trait_argument_identity),*].into_boxed_slice());
                        #codegen::traits::TraitApplicationSpec::<#target>::unresolved_for_target(
                            expression,
                            arguments,
                            identity,
                        )
                    })
                }
            })
        }
        _ => None,
    };
    let implemented_trait = match (declaration.trait_path.as_ref(), trait_definition_provider.as_ref()) {
        (Some(path), Some(provider)) => {
            let name = syn::LitStr::new(&trait_source_path(path), path.span);
            let expression_args = path.segments.iter().flat_map(|segment| {
                super::expression_codegen::path_arguments(&segment.arguments, &environment, &facade, path.span)
            });
            let expression = quote!(::std::boxed::Box::leak(::std::boxed::Box::new(
                #codegen::expression::TraitExpressionData::new(
                    #name,
                    ::std::boxed::Box::leak(::std::vec![#(#expression_args),*].into_boxed_slice()),
                    &[],
                ),
            )));
            quote!(Some(::std::boxed::Box::leak(::std::boxed::Box::new(
                #codegen::traits::trait_expression_for(#expression, #provider()),
            ))))
        }
        (Some(path), _) => {
            let name = syn::LitStr::new(&trait_source_path(path), path.span);
            let expression_args = path.segments.iter().flat_map(|segment| {
                super::expression_codegen::path_arguments(&segment.arguments, &environment, &facade, path.span)
            });
            let expression = quote!(::std::boxed::Box::leak(::std::boxed::Box::new(
                #codegen::expression::TraitExpressionData::new(
                    #name,
                    ::std::boxed::Box::leak(::std::vec![#(#expression_args),*].into_boxed_slice()),
                    &[],
                ),
            )));
            quote!(Some(::std::boxed::Box::leak(::std::boxed::Box::new(
                #codegen::traits::unresolved_trait_expression(#expression),
            ))))
        }
        _ => quote!(None),
    };
    let associated_type_bindings: Vec<_> = declaration
        .associated_types
        .iter()
        .filter_map(|item| {
            let value = item.value.as_ref()?;
            let (Some(provider), Some(_)) = (trait_definition_provider.as_ref(), declaration.trait_path.as_ref())
            else {
                return None;
            };
            let name = syn::LitStr::new(&item.name.to_string(), item.span);
            let expression = super::traits::type_expression(value, &environment, &facade);
            let binding = if declaration.generics.params.is_empty()
                && item.generics.params.is_empty()
                && is_static_concrete_type(value, &environment)
            {
                let concrete_type = &value.tokens;
                quote!(#codegen::impls::associated_type_binding_for::<#concrete_type>(
                    #provider().associated_type_named(#name),
                    #expression,
                    #codegen::impls::ItemSource::Overridden,
                ))
            } else {
                quote!(#codegen::impls::associated_type_binding(
                    #provider().associated_type_named(#name),
                    #expression,
                    #codegen::impls::ItemSource::Overridden,
                ))
            };
            Some(binding)
        })
        .collect();
    let associated_type_binding_names = declaration
        .associated_types
        .iter()
        .map(|item| syn::LitStr::new(&item.name.to_string(), item.span))
        .collect::<Vec<_>>();
    let associated_type_bindings_expression = if let Some(provider) = &trait_definition_provider {
        quote!(#codegen::impls::associated_type_bindings_with_defaults(
            #provider(),
            trait_application_provider().metadata(),
            &[#(#associated_type_binding_names),*],
            ::std::boxed::Box::leak(::std::vec![#(#associated_type_bindings),*].into_boxed_slice()),
        ))
    } else {
        quote!(::std::boxed::Box::leak(
            ::std::vec![#(#associated_type_bindings),*].into_boxed_slice()
        ))
    };
    let associated_const_bindings: Vec<_> = declaration
        .associated_consts
        .iter()
        .filter_map(|item| {
            let value = item.value.as_ref()?;
            let (Some(provider), Some(_)) = (trait_definition_provider.as_ref(), declaration.trait_path.as_ref())
            else {
                return None;
            };
            let name = syn::LitStr::new(&item.name.to_string(), item.span);
            let expression = super::expression_codegen::const_expression(value, &environment, &facade);
            Some(quote!(#codegen::impls::associated_const_binding(
                #provider().associated_const_named(#name),
                #expression.expect("supported const expressions are available"),
                #codegen::impls::ItemSource::Overridden,
            )))
        })
        .collect();
    let associated_const_binding_names = declaration
        .associated_consts
        .iter()
        .map(|item| syn::LitStr::new(&item.name.to_string(), item.span))
        .collect::<Vec<_>>();
    let associated_const_bindings_expression = if let Some(provider) = &trait_definition_provider {
        quote!(#codegen::impls::associated_const_bindings_with_defaults(
            #provider(),
            trait_application_provider().metadata(),
            &[#(#associated_const_binding_names),*],
            ::std::boxed::Box::leak(::std::vec![#(#associated_const_bindings),*].into_boxed_slice()),
        ))
    } else {
        quote!(::std::boxed::Box::leak(
            ::std::vec![#(#associated_const_bindings),*].into_boxed_slice()
        ))
    };
    let mut adapter_definitions = Vec::new();
    let mut thread_safe_selectors = Vec::new();
    for (index, method) in declaration.methods.iter().enumerate() {
        let trait_call_path = declaration.trait_path.as_ref().map(|path| path.tokens.clone());
        if let Some(definition) = invocation_adapter::definition(
            method,
            index,
            &target,
            &trait_call_path,
            &declaration.target_type.source,
            &facade,
        ) {
            adapter_definitions.push(definition);
        }
        // A generic impl is registered as a symbolic definition only. It has
        // no concrete target at this expansion site, so a ThreadSafe selector
        // cannot prove `Send + Sync` or choose an adapter here. Concrete
        // applications must supply a `bind_method!` binding, where Rust has
        // monomorphized the target type.
        if declaration.generics.params.is_empty()
            && let Some(definition) =
                invocation_adapter::thread_safe_definition(method, index, &target, &trait_call_path, &facade)
        {
            adapter_definitions.push(definition.tokens);
            thread_safe_selectors.push((index, definition.selector));
        }
    }
    let adapter_slots: Vec<_> = declaration
        .methods
        .iter()
        .enumerate()
        .filter_map(|(index, method)| {
            let mut method_context = super::invocation::analysis::MethodContext::implementation(&target);
            method_context.extension_receiver = method
                .receiver
                .as_ref()
                .and_then(|receiver| typed_extension_receiver_type(receiver, &target));
            let plan = super::invocation::analysis::analyze_method(method, method_context)
                .expect("validated method analysis is infallible");
            if !plan.is_executable() {
                return None;
            }
            let name = format_ident!("__QUBIT_REFLECT_INVOCATION_ADAPTER_{index}");
            let pinned = plan.pinned_receiver_mutability().is_some();
            let builder = if pinned {
                quote!(#codegen::invoke::InvocationAdapters::builder().pinned_local::<#target>(&#name))
            } else if plan.modes.asynchronous || method.qualifiers.is_unsafe {
                quote!(#codegen::invoke::InvocationAdapters::builder().local(&#name))
            } else {
                quote!(#codegen::invoke::InvocationAdapters::builder().local(&#name).catching_local(&#name))
            };
            let selector = thread_safe_selectors
                .iter()
                .find(|(selector_index, _)| *selector_index == index)
                .map(|(_, selector)| selector);
            let slot = if let Some(selector) = selector {
                let catching = !plan.modes.asynchronous && !method.qualifiers.is_unsafe;
                let thread_safe_builder = if catching {
                    quote!(builder.thread_safe(adapter).catching_thread_safe(adapter))
                } else {
                    quote!(builder.thread_safe(adapter))
                };
                quote!({
                    let builder = #builder;
                    match #selector {
                        Some(adapter) => #thread_safe_builder.build(),
                        None => builder.build(),
                    }
                })
            } else {
                quote!(#builder.build())
            };
            Some((index, slot))
        })
        .collect();
    let effective_methods: Vec<_> = declaration
        .methods
        .iter()
        .enumerate()
        .map(|(index, method)| {
            let slots = adapter_slots
                .iter()
                .filter(|(slot_index, _)| *slot_index == index)
                .map(|(_, slot)| slot)
                .collect::<Vec<_>>();
            let adapters = if slots.is_empty() {
                quote!()
            } else {
                let slot = slots[0];
                quote!(.with_adapters(#slot))
            };
            let mut method_context = super::invocation::analysis::MethodContext::implementation(&target);
            method_context.extension_receiver = method
                .receiver
                .as_ref()
                .and_then(|receiver| typed_extension_receiver_type(receiver, &target));
            let plan = super::invocation::analysis::analyze_method(method, method_context)
                .expect("validated method analysis is infallible");
            let availability = if slots.is_empty() {
                let reasons = if plan.is_executable() {
                    quote!(::std::boxed::Box::leak(::std::boxed::Box::new([
                        #codegen::invoke::InvocationUnavailableReason::AdapterUnavailable,
                    ])))
                } else {
                    super::invocation::emit::emit_unavailable_reasons(&plan, context)
                };
                quote!(.with_unavailable(#reasons))
            } else {
                quote!()
            };
            let source = if declaration.trait_path.is_some() {
                quote!(#codegen::invoke::MethodImplementationSource::Overridden)
            } else {
                quote!(#codegen::invoke::MethodImplementationSource::Declared)
            };
            quote!(#codegen::invoke::EffectiveMethod::for_type::<#target>(&methods[#index])
            #adapters
            #availability
            .with_source(#source))
        })
        .collect();
    let retained = declaration.retained_tokens;
    let registration = if trait_application_provider.is_some() {
        quote! {
            #codegen::traits::submit_trait_application!(#target, trait_application_provider);
            #codegen::impls::submit_impl_definition!(#marker, impl_definition_provider);
            #codegen::impls::submit_impl_application!(#target, impl_application_provider);
        }
    } else {
        quote! {
            #codegen::impls::submit_impl_definition!(#marker, impl_definition_provider);
            #codegen::impls::submit_impl_application!(#target, impl_application_provider);
        }
    };
    let trait_application = if declaration.trait_path.is_some() {
        quote!(Some(trait_application_provider().metadata()))
    } else {
        quote!(None)
    };
    let tokens = quote! {
        #retained

        #[doc(hidden)]
        mod #module {
            use super::*;
            struct #marker;
            #(#adapter_definitions)*
            #trait_application_provider

            fn methods() -> &'static [#codegen::invoke::MethodMetadata] {
                static VALUE: ::std::sync::OnceLock<::std::boxed::Box<[#codegen::invoke::MethodMetadata]>> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| ::std::vec![#(#methods),*].into_boxed_slice())
            }
            fn associated_types() -> &'static [#codegen::traits::AssociatedType] {
                static VALUE: ::std::sync::OnceLock<::std::boxed::Box<[#codegen::traits::AssociatedType]>> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| ::std::vec![#(#associated_types),*].into_boxed_slice())
            }
            fn associated_consts() -> &'static [#codegen::traits::AssociatedConst] {
                static VALUE: ::std::sync::OnceLock<::std::boxed::Box<[#codegen::traits::AssociatedConst]>> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| ::std::vec![#(#associated_consts),*].into_boxed_slice())
            }
            fn generic_definition() -> &'static #codegen::expression::GenericDefinitionMetadata {
                static VALUE: ::std::sync::OnceLock<#codegen::expression::GenericDefinitionMetadata> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| #generic_definition)
            }
            fn impl_definition_provider() -> &'static #codegen::impls::ImplDefinitionSpec<#marker> {
                static VALUE: ::std::sync::OnceLock<#codegen::impls::ImplDefinitionSpec<#marker>> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| #codegen::impls::ImplDefinitionSpec::new(
                    #kind,
                    #target_expression,
                    #implemented_trait,
                    generic_definition(),
                    #codegen::impls::Qualifiers::new(false),
                    methods(),
                    associated_types(),
                    associated_consts(),
                ))
            }
            fn effective_methods() -> &'static [#codegen::invoke::EffectiveMethod] {
                static VALUE: ::std::sync::OnceLock<::std::boxed::Box<[#codegen::invoke::EffectiveMethod]>> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| {
                    let methods = methods();
                    let mut effective_methods: ::std::vec::Vec<#codegen::invoke::EffectiveMethod> =
                        ::std::vec![#(#effective_methods),*];
                    effective_methods.extend(#default_methods.into_vec());
                    effective_methods.into_boxed_slice()
                })
            }
            fn impl_application_provider() -> &'static #codegen::impls::ImplApplicationSpec<#target> {
                static VALUE: ::std::sync::OnceLock<#codegen::impls::ImplApplicationSpec<#target>> = ::std::sync::OnceLock::new();
                VALUE.get_or_init(|| #codegen::impls::ImplApplicationSpec::for_target::<#marker>(
                    impl_definition_provider(),
                    #trait_application,
                    &[],
                    &[],
                    effective_methods(),
                    #associated_type_bindings_expression,
                    #associated_const_bindings_expression,
                ))
            }
            #registration
        }
    };
    tokens
}

fn trait_source_path(path: &crate::ir::PathIr) -> String {
    let mut source = path
        .segments
        .iter()
        .map(|segment| segment.name.as_str())
        .collect::<Vec<_>>()
        .join("::");
    if path.leading_colon {
        source.insert_str(0, "::");
    }
    source
}

fn trait_provider_path(path: &crate::ir::PathIr, suffix: &str) -> syn::Path {
    let mut provider_path: syn::Path = syn::parse2(path.tokens.clone()).expect("validated trait path parses");
    let first = provider_path
        .segments
        .first_mut()
        .expect("trait path has a first segment");
    if first.ident == "self" {
        first.ident = format_ident!("super");
    } else if first.ident == "super" {
        provider_path
            .segments
            .insert(0, syn::PathSegment::from(format_ident!("super")));
    }
    let last = provider_path
        .segments
        .last_mut()
        .expect("trait path has a final segment");
    let trait_name = last.ident.clone();
    last.arguments = syn::PathArguments::None;
    last.ident = format_ident!("__QubitReflectTraitDef_{}{}", trait_name, suffix);
    provider_path
}

fn trait_definition_provider(declaration: &ImplDeclarationIr) -> Option<TokenStream> {
    declaration
        .reflected_trait
        .as_ref()
        .map(|path| {
            let provider_path = trait_provider_path(path, "_SPEC");
            quote!(#provider_path)
        })
        .or_else(|| {
            declaration.external_trait.as_ref().map(|path| {
                let marker_path = path.tokens.clone();
                quote!(#marker_path::__qubit_reflect_trait_definition)
            })
        })
}

// Generic specialization generation lives in `impls::specialization_codegen`.

#[cfg(test)]
mod parameter_concrete_type_tests {
    use super::parameter_concrete_type;
    use crate::expand::generic_environment::GenericEnvironment;
    use crate::parse::convert_type;

    fn parameter_type(source: &str) -> crate::ir::TypeIr {
        convert_type(&syn::parse_str(source).expect("type syntax parses"))
    }

    #[test]
    fn resolves_static_concrete_parameter_types() {
        let environment = GenericEnvironment::new();

        assert_eq!(
            parameter_concrete_type(&parameter_type("u8"), &environment)
                .expect("builtin argument has a concrete type")
                .source,
            "u8"
        );
        assert_eq!(
            parameter_concrete_type(&parameter_type("&str"), &environment)
                .expect("shared references use their referent type")
                .source,
            "str"
        );
        assert_eq!(
            parameter_concrete_type(&parameter_type("CustomValue"), &environment)
                .expect("concrete custom types do not require Reflect")
                .source,
            "CustomValue"
        );
    }

    #[test]
    fn leaves_generic_or_non_static_parameter_types_unresolved() {
        let environment = GenericEnvironment::new().with_type_parameter("T");

        assert!(parameter_concrete_type(&parameter_type("T"), &environment).is_none());
        assert!(parameter_concrete_type(&parameter_type("Option<T>"), &environment).is_none());
        assert!(parameter_concrete_type(&parameter_type("Cow<'a, str>"), &environment).is_none());
    }
}
