//! Expansion of reflected trait declarations and their registration fragments.

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};

use crate::ir::{
    GenericBoundIr, GenericKindIr, GenericsIr, ParameterPatternKindIr, PathArgumentIr,
    PathArgumentsIr, ReceiverKindIr, ReturnTypeIr, TraitDeclarationIr, TypeIr, TypeKindIr,
    WherePredicateIr,
};

/// Expands a validated reflected trait without changing its ordinary Rust semantics.
pub(crate) fn expand(declaration: TraitDeclarationIr) -> TokenStream {
    let facade = match facade_path() {
        Some(path) => path,
        None => return declaration.retained_tokens,
    };
    let fingerprint = fingerprint(&declaration.retained_tokens.to_string());
    let suffix = format!("{fingerprint:016x}");
    let marker = Ident::new(&format!("__QubitReflectTraitMarker_{suffix}"), declaration.span);
    let hook = Ident::new("__qubit_reflect_trait_payload", declaration.span);
    let generic_factory = Ident::new(
        &format!("__qubit_reflect_trait_generics_{suffix}"),
        declaration.span,
    );
    let definition_factory = Ident::new(
        &format!("__qubit_reflect_trait_definition_{suffix}"),
        declaration.span,
    );
    let identity_factory = Ident::new(
        &format!("__qubit_reflect_trait_identity_{suffix}"),
        declaration.span,
    );
    let payload_factory = Ident::new(
        &format!("__qubit_reflect_trait_fragment_payload_{suffix}"),
        declaration.span,
    );
    let support = Ident::new(
        &format!("__qubit_reflect_trait_support_{suffix}"),
        declaration.span,
    );
    let rust_path = Ident::new(
        &format!("__QUBIT_REFLECT_TRAIT_PATH_{}", suffix.to_ascii_uppercase()),
        declaration.span,
    );
    let trait_name = declaration.name.to_string();
    let trait_name_literal = syn::LitStr::new(&trait_name, declaration.span);
    let query_name = declaration
        .attributes
        .iter()
        .find_map(|attribute| attribute.rename())
        .unwrap_or(&trait_name)
        .to_owned();
    let query_name_literal = syn::LitStr::new(&query_name, declaration.span);
    let visibility = match &declaration.visibility {
        crate::ir::VisibilityIr::Public => quote!(#facade::identity::Visibility::Public),
        crate::ir::VisibilityIr::Crate => quote!(#facade::identity::Visibility::Crate),
        crate::ir::VisibilityIr::Super => quote!(#facade::identity::Visibility::Super),
        crate::ir::VisibilityIr::SelfValue | crate::ir::VisibilityIr::Inherited => quote!(#facade::identity::Visibility::Private),
        crate::ir::VisibilityIr::Restricted(path) => { let path = syn::LitStr::new(&path.source, declaration.span); quote!(#facade::identity::Visibility::Restricted(#path.into())) },
    };
    let direct_supertraits = declaration
        .supertraits
        .iter()
        .filter_map(|bound| match bound {
            GenericBoundIr::Trait { path, .. } => {
                if let Some(external) = declaration
                    .external_traits
                    .iter()
                    .find(|external| external.path.source == path.source)
                {
                    let id = syn::LitStr::new(&external.id, declaration.span);
                    let diagnostic_path = syn::LitStr::new(&path.source, declaration.span);
                    let arguments = path
                        .segments
                        .last()
                        .map(|segment| {
                            external_supertrait_arguments(
                                &segment.arguments,
                                &declaration,
                                &facade,
                            )
                        })
                        .unwrap_or_default();
                    Some(quote!(#facade::__private::external_supertrait::<Self>(
                        #id,
                        #diagnostic_path,
                        vec![#(#arguments),*],
                    )))
                } else {
                    let path = &path.tokens;
                    Some(quote!(<Self as #path>::__qubit_reflect_trait_payload().applied()))
                }
            }
            _ => None,
        });
    let applied_arguments = declaration.generics.params.iter().filter_map(|parameter| {
        if parameter.kind == GenericKindIr::Const {
            let identifier = Ident::new(&parameter.name, declaration.span);
            let type_source = parameter.const_type.as_ref()?.source.as_str();
            let expression = match type_source {
                "bool" => quote!(#facade::expression::ConstExpression::Boolean(#identifier)),
                "char" => quote!(#facade::expression::ConstExpression::Character(#identifier)),
                "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => quote!(#facade::expression::ConstExpression::SignedInteger(#identifier as i128)),
                "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => quote!(#facade::expression::ConstExpression::UnsignedInteger(#identifier as u128)),
                _ => return None,
            };
            let type_source = syn::LitStr::new(type_source, declaration.span);
            return Some(quote!(#facade::expression::GenericArgument::Const(
                #facade::expression::ConstGenericArgument {
                    declared_type: Box::new(#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression {
                        path: Box::new([#type_source.into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::from(#type_source),
                    })),
                    value: #expression,
                    normalized_diagnostic: stringify!(#identifier).into(),
                },
            )));
        }
        if parameter.kind != GenericKindIr::Type { return None; }
        let identifier = Ident::new(&parameter.name, declaration.span);
        Some(quote! {
            #facade::expression::GenericArgument::Type(
                #facade::expression::TypeExpression::Concrete(
                    #facade::expression::ConcreteTypeExpression {
                        path: Box::new([std::any::type_name::<#identifier>().into()]),
                        arguments: Box::new([]),
                        diagnostic: #facade::expression::DiagnosticText::from(
                            std::any::type_name::<#identifier>(),
                        ),
                    },
                ),
            )
        })
    });
    let hook_type_bounds = declaration.generics.params.iter().filter_map(|parameter| {
        if parameter.kind != GenericKindIr::Type {
            return None;
        }
        let identifier = Ident::new(&parameter.name, declaration.span);
        Some(quote!(#identifier: 'static))
    });
    let methods = declaration.methods.iter().enumerate().map(|(method_index, method)| {
        let name = syn::LitStr::new(&method.name.to_string(), method.span);
        let query = method
            .attributes
            .iter()
            .find_map(|attribute| attribute.rename())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| method.name.to_string());
        let query = syn::LitStr::new(&query, method.span);
        let index = method_index;
        let receiver = match &method.receiver {
            Some(receiver) => match receiver.kind {
                ReceiverKindIr::Value => quote!(Some(#facade::descriptor::ReceiverDescriptor::Owned)),
                ReceiverKindIr::SharedReference => quote!(Some(#facade::descriptor::ReceiverDescriptor::Shared)),
                ReceiverKindIr::MutableReference => quote!(Some(#facade::descriptor::ReceiverDescriptor::Mutable)),
                ReceiverKindIr::Typed => {
                    let declaration = syn::LitStr::new(&receiver.declaration.to_string(), receiver.span);
                    quote!(Some(#facade::descriptor::ReceiverDescriptor::Explicit(#declaration)))
                }
            },
            None => quote!(None),
        };
        let parameters = method.parameters.iter().map(|parameter| {
            let name = parameter.name.as_deref().map(|name| syn::LitStr::new(name, parameter.span));
            let name = match name { Some(name) => quote!(Some(#name)), None => quote!(None) };
            let pattern = match parameter.pattern.kind {
                ParameterPatternKindIr::Identifier => quote!(#facade::descriptor::ParameterPatternDescriptor::Identifier),
                ParameterPatternKindIr::Wildcard => quote!(#facade::descriptor::ParameterPatternDescriptor::Wildcard),
                ParameterPatternKindIr::Destructure => {
                    let source = syn::LitStr::new(&parameter.pattern.source, parameter.span);
                    quote!(#facade::descriptor::ParameterPatternDescriptor::Destructure(#source.into()))
                }
            };
            let passing = match &parameter.ty.kind {
                TypeKindIr::Reference { mutable: true, .. } => quote!(#facade::descriptor::ParameterPassingMode::MutableBorrow),
                TypeKindIr::Reference { .. } => quote!(#facade::descriptor::ParameterPassingMode::SharedBorrow),
                _ => quote!(#facade::descriptor::ParameterPassingMode::Owned),
            };
            let ty = type_expression(&parameter.ty, &facade);
            let index = parameter.index;
            quote!(#facade::descriptor::ParameterDescriptor::new(#index, #name, #pattern, #passing, #ty, None))
        });
        let return_value = match &method.return_type {
            ReturnTypeIr::Unit => quote!(#facade::descriptor::ReturnDescriptor::unit()),
            ReturnTypeIr::Type(ty) => {
                let expression = type_expression(ty, &facade);
                let kind = match ty.kind {
                    TypeKindIr::Never => quote!(#facade::descriptor::ReturnKind::Never),
                    TypeKindIr::Reference { .. } => quote!(#facade::descriptor::ReturnKind::Reference),
                    TypeKindIr::ImplTrait { .. } => quote!(#facade::descriptor::ReturnKind::Opaque),
                    _ => quote!(#facade::descriptor::ReturnKind::Concrete),
                };
                quote!(#facade::descriptor::ReturnDescriptor::new(#kind, Some(#expression), None))
            }
        };
        let qualifiers = &method.qualifiers;
        let method_generic_definition = generic_definition(&method.generics, method.span, &facade);
        let is_async = qualifiers.is_async;
        let is_unsafe = qualifiers.is_unsafe;
        let is_const = qualifiers.is_const;
        let is_variadic = qualifiers.is_variadic;
        let has_default = method.has_default;
        let abi = qualifiers.abi.as_deref().map(|abi| syn::LitStr::new(abi, method.span));
        let abi = match abi { Some(abi) => quote!(Some(#facade::expression::FunctionAbi::Other(#abi.into()))), None => quote!(None) };
        quote! {
            #facade::descriptor::MethodDescriptor::builder(
                #facade::identity::MemberId::new(
                    #trait_name_literal,
                    "method",
                    #index,
                    #facade::identity::FragmentIdentity::new(
                        env!("CARGO_PKG_NAME"), module_path!(), line!(), column!(), "method", #index as u64,
                    ),
                ),
                #name,
                #query,
                #facade::descriptor::MethodDeclarationOwner::Trait(definition),
            )
            .visibility(#facade::descriptor::MethodVisibility::InheritedFromTrait)
            .receiver(#receiver)
            .parameters(vec![#(#parameters),*])
            .return_value(#return_value)
            .qualifiers(#facade::descriptor::MethodQualifiers {
                is_async: #is_async,
                is_unsafe: #is_unsafe,
                is_const: #is_const,
                abi: #abi,
                is_variadic: #is_variadic,
            })
            .generic_definition(&#method_generic_definition)
            .has_default(#has_default)
            .build()
        }
    });
    let associated_types = declaration.associated_types.iter().enumerate().map(|(index, item)| {
        let name = syn::LitStr::new(&item.name.to_string(), item.span);
        let bounds = item.bounds.iter().filter_map(|bound| match bound {
            GenericBoundIr::Trait { path, lifetimes, modifier } => {
                let path = syn::LitStr::new(&path.source, item.span);
                let lifetimes = lifetimes
                    .iter()
                    .map(|lifetime| lifetime_expression(lifetime, item.span, &facade));
                let modifier = match modifier { crate::ir::TraitBoundModifierIr::None => quote!(#facade::expression::TraitBoundModifier::None), crate::ir::TraitBoundModifierIr::Maybe => quote!(#facade::expression::TraitBoundModifier::Maybe) };
                Some(quote!(#facade::expression::PredicateDescriptor::TypeBound {
                    subject: #facade::expression::TypeExpression::Parameter(#name.into()),
                    bounds: Box::new([#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression {
                        path: Box::new([#path.into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::from(#path),
                    })]), bound_modifiers: Box::new([#modifier]), higher_ranked_lifetimes: Box::new([#(#lifetimes),*]), diagnostic: #facade::expression::DiagnosticText::default(),
                }))
            }
            GenericBoundIr::Lifetime(lifetime) => {
                let lifetime = lifetime_expression(lifetime, item.span, &facade);
                Some(quote!(#facade::expression::PredicateDescriptor::TypeOutlives {
                    ty: #facade::expression::TypeExpression::Parameter(#name.into()),
                    lifetime: #lifetime,
                    diagnostic: #facade::expression::DiagnosticText::default(),
                }))
            }
            _ => None,
        });
        let default = item.value.as_ref().map(|value| type_expression(value, &facade));
        let default = match default { Some(value) => quote!(Some(#value)), None => quote!(None) };
        quote!(#facade::descriptor::AssociatedTypeDescriptor::new(#index, #name, #name, Box::new([#(#bounds),*]), #default))
    });
    let associated_consts = declaration.associated_consts.iter().enumerate().map(|(index, item)| {
        let name = syn::LitStr::new(&item.name.to_string(), item.span);
        let ty = type_expression(&item.ty, &facade);
        let has_default = item.value.is_some();
        quote!(#facade::descriptor::AssociatedConstDescriptor::new(#index, #name, #name, #ty, #has_default))
    });
    let parameters = declaration.generics.params.iter().map(|parameter| {
        let name = syn::LitStr::new(&parameter.name, declaration.span);
        match parameter.kind {
            GenericKindIr::Lifetime => {
                let bounds = parameter.bounds.iter().filter_map(|bound| match bound {
                    GenericBoundIr::Lifetime(lifetime) => Some(lifetime_expression(
                        lifetime,
                        declaration.span,
                        &facade,
                    )),
                    _ => None,
                });
                quote! {
                __qubit_reflect::expression::GenericParameterDescriptor::Lifetime {
                    name: #name.into(),
                    bounds: Box::new([#(#bounds),*]),
                    diagnostic: __qubit_reflect::expression::DiagnosticText::default(),
                }
            }}
            GenericKindIr::Type => {
                let subject = syn::LitStr::new(&parameter.name, declaration.span);
                let bounds = parameter.bounds.iter().filter_map(|bound| match bound {
                    GenericBoundIr::Trait { path, lifetimes, modifier } => {
                        let path = syn::LitStr::new(&path.source, declaration.span);
                        let lifetimes = lifetimes.iter().map(|lifetime| {
                            lifetime_expression(lifetime, declaration.span, &facade)
                        });
                        let modifier = match modifier {
                            crate::ir::TraitBoundModifierIr::None => quote!(#facade::expression::TraitBoundModifier::None),
                            crate::ir::TraitBoundModifierIr::Maybe => quote!(#facade::expression::TraitBoundModifier::Maybe),
                        };
                        Some(quote!(#facade::expression::PredicateDescriptor::TypeBound {
                            subject: #facade::expression::TypeExpression::Parameter(#subject.into()),
                            bounds: Box::new([#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression {
                                path: Box::new([#path.into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::from(#path),
                            })]),
                            bound_modifiers: Box::new([#modifier]),
                            higher_ranked_lifetimes: Box::new([#(#lifetimes),*]), diagnostic: #facade::expression::DiagnosticText::default(),
                        }))
                    }
                    GenericBoundIr::Lifetime(lifetime) => {
                        let lifetime = lifetime_expression(lifetime, declaration.span, &facade);
                        Some(quote!(#facade::expression::PredicateDescriptor::TypeOutlives {
                            ty: #facade::expression::TypeExpression::Parameter(#subject.into()),
                            lifetime: #lifetime,
                            diagnostic: #facade::expression::DiagnosticText::default(),
                        }))
                    }
                    _ => None,
                });
                let default = match parameter.default.as_ref() {
                    Some(crate::ir::GenericDefaultIr::Type(value)) => {
                        let value = type_expression(value, &facade);
                        quote!(Some(#value))
                    }
                    _ => quote!(None),
                };
                quote! {
                __qubit_reflect::expression::GenericParameterDescriptor::Type {
                    name: #name.into(),
                    bounds: Box::new([#(#bounds),*]),
                    default: #default,
                    diagnostic: __qubit_reflect::expression::DiagnosticText::default(),
                }
            }}
            GenericKindIr::Const => {
                let const_type = parameter
                    .const_type
                    .as_ref()
                    .map(|value| value.source.as_str())
                    .unwrap_or("_");
                let const_type = syn::LitStr::new(const_type, declaration.span);
                let default = match parameter.default.as_ref() {
                    Some(crate::ir::GenericDefaultIr::Const(value)) => const_expression(value, &facade),
                    _ => quote!(None),
                };
                quote! {
                    __qubit_reflect::expression::GenericParameterDescriptor::Const {
                        name: #name.into(),
                        ty: Box::new(__qubit_reflect::expression::TypeExpression::Concrete(
                            __qubit_reflect::expression::ConcreteTypeExpression {
                                path: Box::new([#const_type.into()]),
                                arguments: Box::new([]),
                                diagnostic: __qubit_reflect::expression::DiagnosticText::from(#const_type),
                            },
                        )),
                        default: #default,
                        diagnostic: __qubit_reflect::expression::DiagnosticText::default(),
                    }
                }
            }
        }
    });
    let where_predicates = declaration.generics.where_predicates.iter().flat_map(|predicate| match predicate {
        crate::ir::WherePredicateIr::Lifetime { lifetime, bounds, .. } => {
            let lifetime = lifetime_expression(lifetime, declaration.span, &facade);
            let bounds: Vec<_> = bounds.iter().map(|bound| {
                lifetime_expression(bound, declaration.span, &facade)
            }).collect();
            vec![quote!(#facade::expression::PredicateDescriptor::LifetimeOutlives {
                lifetime: #lifetime,
                bounds: Box::new([#(#bounds),*]), diagnostic: #facade::expression::DiagnosticText::default(),
            })]
        }
        crate::ir::WherePredicateIr::Type {
            bounded_type,
            lifetimes,
            bounds,
            ..
        } => {
            let subject = type_expression(bounded_type, &facade);
            let higher_ranked_lifetimes: Vec<_> = lifetimes.iter().map(|lifetime| {
                lifetime_expression(lifetime, declaration.span, &facade)
            }).collect();
            let trait_bounds: Vec<_> = bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Trait { path, .. } => {
                    let path = syn::LitStr::new(&path.source, declaration.span);
                    Some(quote!(#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression {
                        path: Box::new([#path.into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::from(#path),
                    })))
                }
                _ => None,
            }).collect();
            let bound_modifiers: Vec<_> = bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Trait { modifier, .. } => Some(match modifier {
                    crate::ir::TraitBoundModifierIr::None => {
                        quote!(#facade::expression::TraitBoundModifier::None)
                    }
                    crate::ir::TraitBoundModifierIr::Maybe => {
                        quote!(#facade::expression::TraitBoundModifier::Maybe)
                    }
                }),
                _ => None,
            }).collect();
            let type_bound = (!trait_bounds.is_empty()).then(|| quote!(#facade::expression::PredicateDescriptor::TypeBound {
                subject: #subject,
                bounds: Box::new([#(#trait_bounds),*]),
                bound_modifiers: Box::new([#(#bound_modifiers),*]),
                higher_ranked_lifetimes: Box::new([#(#higher_ranked_lifetimes),*]),
                diagnostic: #facade::expression::DiagnosticText::default(),
            }));
            let lifetime_bounds = bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Lifetime(lifetime) => {
                    let lifetime = lifetime_expression(lifetime, declaration.span, &facade);
                    let subject = type_expression(bounded_type, &facade);
                    Some(quote!(#facade::expression::PredicateDescriptor::TypeOutlives {
                        ty: #subject,
                        lifetime: #lifetime,
                        diagnostic: #facade::expression::DiagnosticText::default(),
                    }))
                }
                _ => None,
            });
            type_bound.into_iter().chain(lifetime_bounds).collect()
        }
        _ => Vec::new(),
    });
    let mut trait_item: syn::ItemTrait = match syn::parse2(declaration.retained_tokens.clone()) {
        Ok(item) => item,
        Err(error) => return error.into_compile_error(),
    };
    if trait_item.items.iter().any(|item| {
        matches!(item, syn::TraitItem::Fn(function) if function.sig.ident == hook)
    }) {
        return syn::Error::new(
            declaration.span,
            "`__qubit_reflect_trait_payload` is reserved by #[reflect]",
        )
        .into_compile_error();
    }
    let hook_item = match syn::parse2(quote! {
        #[doc(hidden)]
        fn #hook() -> #facade::__private::TraitImplPayload
        where
            Self: Sized + 'static,
            #(#hook_type_bounds,)*
        {
            let definition = #support::#definition_factory();
            let arguments = vec![#(#applied_arguments),*];
            #facade::__private::TraitImplPayload::cached_with_arguments::<Self>(definition, arguments, |arguments| {
                #facade::descriptor::TraitDescriptor::builder(definition)
                    .arguments(arguments)
                    .direct_supertraits([#(#direct_supertraits),*])
                    .methods(Box::leak(vec![#(#methods),*].into_boxed_slice()))
                    .associated_types(vec![#(#associated_types),*])
                    .associated_consts(vec![#(#associated_consts),*])
                    .build()
            })
        }
    }) {
        Ok(item) => item,
        Err(error) => return error.into_compile_error(),
    };
    trait_item.items.push(hook_item);
    quote! {
        #trait_item

        #[doc(hidden)]
        const #rust_path: &str = concat!(module_path!(), "::", #trait_name_literal);

        #[doc(hidden)]
        mod #support {
            use #facade as __qubit_reflect;

            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            struct #marker;

            #[doc(hidden)]
            fn #generic_factory() -> &'static __qubit_reflect::expression::GenericDefinitionDescriptor {
                static VALUE: std::sync::LazyLock<__qubit_reflect::expression::GenericDefinitionDescriptor> =
                    std::sync::LazyLock::new(|| __qubit_reflect::expression::GenericDefinitionDescriptor {
                        parameters: Box::new([#(#parameters),*]),
                        predicates: Box::new([#(#where_predicates),*]),
                        diagnostic: __qubit_reflect::expression::DiagnosticText::default(),
                    });
                &VALUE
            }

            #[doc(hidden)]
            pub(super) fn #definition_factory() -> &'static __qubit_reflect::descriptor::TraitDefinitionDescriptor {
                static VALUE: std::sync::LazyLock<__qubit_reflect::descriptor::TraitDefinitionDescriptor> =
                    std::sync::LazyLock::new(|| __qubit_reflect::descriptor::TraitDefinitionDescriptor::new_with_visibility(
                        __qubit_reflect::descriptor::TraitId::Reflected(std::any::TypeId::of::<#marker>()),
                        #trait_name_literal,
                        super::#rust_path,
                        #query_name_literal,
                        __qubit_reflect::descriptor::TraitCompleteness::Complete,
                        #generic_factory(),
                        #visibility,
                    ));
                &VALUE
            }

            #[doc(hidden)]
            fn #identity_factory() -> __qubit_reflect::__private::RuntimeIdentity {
                __qubit_reflect::__private::RuntimeIdentity::Trait(
                    __qubit_reflect::descriptor::TraitId::Reflected(std::any::TypeId::of::<#marker>()),
                )
            }

            #[doc(hidden)]
            fn #payload_factory() -> __qubit_reflect::__private::FragmentPayload {
                __qubit_reflect::__private::FragmentPayload::Trait(#definition_factory())
            }

            __qubit_reflect::__private::inventory::submit! {
                __qubit_reflect::__private::RegistrationFragment::new(
                    __qubit_reflect::__private::FragmentKind::Trait,
                    __qubit_reflect::__private::StaticFragmentIdentity::new(
                        env!("CARGO_PKG_NAME"),
                        module_path!(),
                        line!(),
                        column!(),
                        "trait",
                        #fingerprint,
                    ),
                    #identity_factory,
                    #payload_factory,
                )
            }
        }
    }
}

/// Returns the caller-visible path of the reflection facade.
fn facade_path() -> Option<TokenStream> {
    match proc_macro_crate::crate_name("qubit-reflect") {
        Ok(proc_macro_crate::FoundCrate::Itself) => Some(quote!(crate)),
        Ok(proc_macro_crate::FoundCrate::Name(name)) => {
            let identifier = Ident::new(&name, Span::call_site());
            Some(quote!(#identifier))
        }
        Err(_) => None,
    }
}

/// Computes a stable FNV-1a fingerprint for one normalized declaration.
fn fingerprint(input: &str) -> u64 {
    input.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

/// Converts generic declaration facts into the runtime generic descriptor model.
pub(crate) fn generic_definition(generics: &GenericsIr, span: Span, facade: &TokenStream) -> TokenStream {
    let parameters = generics.params.iter().map(|parameter| {
        let name = syn::LitStr::new(&parameter.name, parameter.span);
        match parameter.kind {
            GenericKindIr::Lifetime => {
                let bounds = parameter.bounds.iter().filter_map(|bound| match bound {
                    GenericBoundIr::Lifetime(value) => Some(lifetime_expression(value, parameter.span, facade)),
                    _ => None,
                });
                quote!(#facade::expression::GenericParameterDescriptor::Lifetime {
                    name: #name.into(), bounds: Box::new([#(#bounds),*]),
                    diagnostic: #facade::expression::DiagnosticText::default(),
                })
            }
            GenericKindIr::Type => {
                let subject = syn::LitStr::new(&parameter.name, parameter.span);
                let bounds = generic_bounds(&parameter.bounds, &subject, parameter.span, facade);
                let default = match parameter.default.as_ref() {
                    Some(crate::ir::GenericDefaultIr::Type(value)) => {
                        let value = type_expression(value, facade);
                        quote!(Some(#value))
                    }
                    _ => quote!(None),
                };
                quote!(#facade::expression::GenericParameterDescriptor::Type {
                    name: #name.into(), bounds: Box::new([#(#bounds),*]), default: #default,
                    diagnostic: #facade::expression::DiagnosticText::default(),
                })
            }
            GenericKindIr::Const => {
                let ty = parameter.const_type.as_ref().map(|ty| type_expression(ty, facade)).unwrap_or_else(|| quote!(#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression { path: Box::new(["_".into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::default() })));
                let default = match parameter.default.as_ref() {
                    Some(crate::ir::GenericDefaultIr::Const(value)) => const_expression(value, facade),
                    _ => quote!(None),
                };
                quote!(#facade::expression::GenericParameterDescriptor::Const {
                    name: #name.into(), ty: Box::new(#ty), default: #default,
                    diagnostic: #facade::expression::DiagnosticText::default(),
                })
            }
        }
    });
    let predicates = generics.where_predicates.iter().flat_map(|predicate| match predicate {
        WherePredicateIr::Lifetime { lifetime, bounds, .. } => {
            let lifetime = lifetime_expression(lifetime, span, facade);
            let bounds = bounds.iter().map(|bound| lifetime_expression(bound, span, facade));
            vec![quote!(#facade::expression::PredicateDescriptor::LifetimeOutlives {
                lifetime: #lifetime, bounds: Box::new([#(#bounds),*]),
                diagnostic: #facade::expression::DiagnosticText::default(),
            })]
        }
        WherePredicateIr::Type { bounded_type, lifetimes, bounds, .. } => {
            let subject = type_expression(bounded_type, facade);
            let lifetimes = lifetimes.iter().map(|lifetime| lifetime_expression(lifetime, span, facade));
            let trait_bounds: Vec<_> = bounds.iter().filter_map(|bound| match bound {
                GenericBoundIr::Trait { path, .. } => {
                    let path = syn::LitStr::new(&path.source, span);
                    Some(quote!(#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression { path: Box::new([#path.into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::from(#path) })))
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
            let type_bound = (!trait_bounds.is_empty()).then(|| quote!(#facade::expression::PredicateDescriptor::TypeBound {
                subject: #subject, bounds: Box::new([#(#trait_bounds),*]), bound_modifiers: Box::new([#(#modifiers),*]), higher_ranked_lifetimes: Box::new([#(#lifetimes),*]), diagnostic: #facade::expression::DiagnosticText::default(),
            }));
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
    quote!(#facade::expression::GenericDefinitionDescriptor {
        parameters: Box::new([#(#parameters),*]), predicates: Box::new([#(#predicates),*]),
        diagnostic: #facade::expression::DiagnosticText::default(),
    })
}

fn generic_bounds(
    bounds: &[GenericBoundIr],
    subject: &syn::LitStr,
    span: Span,
    facade: &TokenStream,
) -> Vec<TokenStream> {
    bounds.iter().filter_map(move |bound| match bound {
        GenericBoundIr::Trait { path, lifetimes, modifier } => {
            let path = syn::LitStr::new(&path.source, span);
            let lifetimes = lifetimes.iter().map(|lifetime| lifetime_expression(lifetime, span, facade));
            let modifier = match modifier {
                crate::ir::TraitBoundModifierIr::None => quote!(#facade::expression::TraitBoundModifier::None),
                crate::ir::TraitBoundModifierIr::Maybe => quote!(#facade::expression::TraitBoundModifier::Maybe),
            };
            Some(quote!(#facade::expression::PredicateDescriptor::TypeBound {
                subject: #facade::expression::TypeExpression::Parameter(#subject.into()),
                bounds: Box::new([#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression { path: Box::new([#path.into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::from(#path) })]),
                bound_modifiers: Box::new([#modifier]), higher_ranked_lifetimes: Box::new([#(#lifetimes),*]), diagnostic: #facade::expression::DiagnosticText::default(),
            }))
        }
        GenericBoundIr::Lifetime(lifetime) => {
            let lifetime = lifetime_expression(lifetime, span, facade);
            Some(quote!(#facade::expression::PredicateDescriptor::TypeOutlives {
                ty: #facade::expression::TypeExpression::Parameter(#subject.into()), lifetime: #lifetime,
                diagnostic: #facade::expression::DiagnosticText::default(),
            }))
        }
        GenericBoundIr::Other(_) => None,
    }).collect()
}

/// Converts source lifetime syntax into the runtime lifetime expression model.
fn lifetime_expression(lifetime: &str, span: Span, facade: &TokenStream) -> TokenStream {
    if lifetime == "'static" {
        return quote!(#facade::expression::LifetimeExpression::Static);
    }
    let lifetime = syn::LitStr::new(lifetime.trim_start_matches('\''), span);
    quote!(#facade::expression::LifetimeExpression::Named(#lifetime.into()))
}

/// Converts the type forms required by trait item descriptors into runtime expressions.
pub(crate) fn type_expression(ty: &TypeIr, facade: &TokenStream) -> TokenStream {
    match &ty.kind {
        TypeKindIr::Never => quote!(#facade::expression::TypeExpression::Never),
        TypeKindIr::Path(path) => path_expression(path, ty, facade),
        TypeKindIr::Reference { lifetime, mutable, element } => {
            let target = type_expression(element, facade);
            let lifetime = match lifetime.as_deref() {
                Some("'static") => quote!(#facade::expression::LifetimeExpression::Static),
                Some(value) => { let value = syn::LitStr::new(value.trim_start_matches('\''), ty.span); quote!(#facade::expression::LifetimeExpression::Named(#value.into())) },
                None => quote!(#facade::expression::LifetimeExpression::Elided),
            };
            quote!(#facade::expression::TypeExpression::Reference(#facade::expression::ReferenceTypeExpression {
                lifetime: #lifetime, mutable: #mutable, target: Box::new(#target), diagnostic: #facade::expression::DiagnosticText::default(),
            }))
        }
        TypeKindIr::Tuple(elements) => {
            let elements = elements.iter().map(|element| type_expression(element, facade));
            quote!(#facade::expression::TypeExpression::Tuple(Box::new([#(#elements),*])))
        }
        TypeKindIr::Slice(element) => {
            let element = type_expression(element, facade);
            quote!(#facade::expression::TypeExpression::Slice(Box::new(#element)))
        }
        TypeKindIr::Array { element, length } => {
            let element = type_expression(element, facade);
            let length = const_expression_value(length, facade);
            quote!(#facade::expression::TypeExpression::Array(#facade::expression::ArrayTypeExpression {
                element: Box::new(#element), length: #length, diagnostic: #facade::expression::DiagnosticText::default(),
            }))
        }
        TypeKindIr::Pointer { mutable, element } => {
            let target = type_expression(element, facade);
            quote!(#facade::expression::TypeExpression::RawPointer(#facade::expression::RawPointerTypeExpression {
                mutable: #mutable, target: Box::new(#target), diagnostic: #facade::expression::DiagnosticText::default(),
            }))
        }
        TypeKindIr::BareFunction { lifetimes, inputs, output, is_unsafe, abi, is_variadic } => {
            let higher_ranked_lifetimes = lifetimes.iter().map(|value| lifetime_expression(value, ty.span, facade));
            let parameters = inputs.iter().map(|value| type_expression(value, facade));
            let return_type = output.as_deref().map(|value| type_expression(value, facade)).unwrap_or_else(|| quote!(#facade::expression::TypeExpression::Tuple(Box::new([]))));
            let safety = if *is_unsafe { quote!(#facade::expression::FunctionSafety::Unsafe) } else { quote!(#facade::expression::FunctionSafety::Safe) };
            let abi = match abi.as_deref() { Some("C") => quote!(#facade::expression::FunctionAbi::C), Some("system") => quote!(#facade::expression::FunctionAbi::System), Some(value) => { let value = syn::LitStr::new(value, ty.span); quote!(#facade::expression::FunctionAbi::Other(#value.into())) }, None => quote!(#facade::expression::FunctionAbi::Rust) };
            quote!(#facade::expression::TypeExpression::FunctionPointer(#facade::expression::FunctionPointerExpression {
                abi: #abi, safety: #safety, variadic: #is_variadic, higher_ranked_lifetimes: Box::new([#(#higher_ranked_lifetimes),*]), parameters: Box::new([#(#parameters),*]), return_type: Box::new(#return_type), diagnostic: #facade::expression::DiagnosticText::default(),
            }))
        }
        TypeKindIr::TraitObject { bounds, .. } => {
            let bounds = bound_predicates(bounds, facade, ty.span);
            quote!(#facade::expression::TypeExpression::TraitObject(#facade::expression::TraitObjectExpression { bounds: Box::new([#(#bounds),*]), diagnostic: #facade::expression::DiagnosticText::default() }))
        }
        TypeKindIr::ImplTrait { bounds } => {
            let bounds = bound_predicates(bounds, facade, ty.span);
            quote!(#facade::expression::TypeExpression::Opaque(#facade::expression::OpaqueTypeExpression { bounds: Box::new([#(#bounds),*]), diagnostic: #facade::expression::DiagnosticText::default() }))
        }
        _ => {
            let source = syn::LitStr::new(&ty.source, ty.span);
            quote!(#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression {
                path: Box::new([#source.into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::from(#source),
            }))
        }
    }
}

fn path_expression(path: &crate::ir::PathIr, ty: &TypeIr, facade: &TokenStream) -> TokenStream {
    let diagnostic = syn::LitStr::new(&ty.source, ty.span);
    if path.qualified_self.is_none() && path.segments.len() == 1 && path.segments[0].name == "Self" {
        return quote!(#facade::expression::TypeExpression::SelfType);
    }
    if path.qualified_self.is_none()
        && path.segments.len() == 1
        && matches!(path.segments[0].arguments, PathArgumentsIr::None)
        && path.segments[0].name.chars().all(|value| value.is_ascii_uppercase())
    {
        let name = syn::LitStr::new(&path.segments[0].name, ty.span);
        return quote!(#facade::expression::TypeExpression::Parameter(#name.into()));
    }
    if let Some(qualified) = &path.qualified_self {
        let self_type = type_expression(&qualified.ty, facade);
        let item = path.segments.last().map(|segment| syn::LitStr::new(&segment.name, ty.span)).expect("qualified path has an item");
        let arguments = path.segments.last().map(|segment| path_arguments(&segment.arguments, facade, ty.span)).unwrap_or_default();
        let trait_segments = path.segments.iter().take(qualified.position).map(|segment| syn::LitStr::new(&segment.name, ty.span));
        let trait_path = if qualified.has_as { quote!(Some(Box::new(#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression { path: Box::new([#(#trait_segments.into()),*]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::default() })))) } else { quote!(None) };
        return quote!(#facade::expression::TypeExpression::Associated(#facade::expression::AssociatedTypeExpression { self_type: Box::new(#self_type), trait_path: #trait_path, item: #item.into(), arguments: Box::new([#(#arguments),*]), diagnostic: #facade::expression::DiagnosticText::from(#diagnostic) }));
    }
    let segments = path.segments.iter().map(|segment| syn::LitStr::new(&segment.name, ty.span));
    let arguments = path.segments.last().map(|segment| path_arguments(&segment.arguments, facade, ty.span)).unwrap_or_default();
    quote!(#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression { path: Box::new([#(#segments.into()),*]), arguments: Box::new([#(#arguments),*]), diagnostic: #facade::expression::DiagnosticText::from(#diagnostic) }))
}

fn path_arguments(arguments: &PathArgumentsIr, facade: &TokenStream, span: Span) -> Vec<TokenStream> {
    match arguments {
        PathArgumentsIr::None => Vec::new(),
        PathArgumentsIr::Parenthesized { inputs, output } => {
            let inputs = inputs.iter().map(|value| type_expression(value, facade));
            let output = output.as_deref().map(|value| type_expression(value, facade)).unwrap_or_else(|| quote!(#facade::expression::TypeExpression::Tuple(Box::new([]))));
            vec![quote!(#facade::expression::GenericArgument::Type(#facade::expression::TypeExpression::FunctionPointer(#facade::expression::FunctionPointerExpression { abi: #facade::expression::FunctionAbi::Rust, safety: #facade::expression::FunctionSafety::Safe, variadic: false, higher_ranked_lifetimes: Box::new([]), parameters: Box::new([#(#inputs),*]), return_type: Box::new(#output), diagnostic: #facade::expression::DiagnosticText::default() })))]
        }
        PathArgumentsIr::AngleBracketed(values) => values.iter().filter_map(|value| match value {
            PathArgumentIr::Lifetime(value) => { let value = lifetime_expression(value, span, facade); Some(quote!(#facade::expression::GenericArgument::Lifetime(#value))) }
            PathArgumentIr::Type(value) => { let value = type_expression(value, facade); Some(quote!(#facade::expression::GenericArgument::Type(#value))) }
            PathArgumentIr::Const(value) => { let value = const_expression_value(value, facade); let source = syn::LitStr::new(&value.to_string(), span); Some(quote!(#facade::expression::GenericArgument::Const(#facade::expression::ConstGenericArgument { declared_type: Box::new(#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression { path: Box::new(["_".into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::default() })), value: #value, normalized_diagnostic: #source.into() }))) }
            PathArgumentIr::AssociatedType { name, ty } => { let name = syn::LitStr::new(name, span); let value = type_expression(ty, facade); Some(quote!(#facade::expression::GenericArgument::AssociatedType { name: #name.into(), value: Box::new(#value) })) }
            _ => None,
        }).collect(),
    }
}

/// Materializes direct external-supertrait arguments at the trait hook's concrete instance.
fn external_supertrait_arguments(
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
            PathArgumentIr::Type(TypeIr {
                kind: TypeKindIr::Path(path),
                ..
            }) if path.segments.len() == 1 => {
                let name = &path.segments[0].name;
                let parameter = declaration.generics.params.iter().find(|parameter| {
                    parameter.kind == GenericKindIr::Type && parameter.name == *name
                })?;
                let identifier = Ident::new(&parameter.name, parameter.span);
                Some(quote!(#facade::expression::GenericArgument::Type(
                    #facade::expression::TypeExpression::Concrete(
                        #facade::expression::ConcreteTypeExpression {
                            path: Box::new([std::any::type_name::<#identifier>().into()]),
                            arguments: Box::new([]),
                            diagnostic: #facade::expression::DiagnosticText::from(
                                std::any::type_name::<#identifier>(),
                            ),
                        },
                    ),
                )))
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

fn bound_predicates(bounds: &[GenericBoundIr], facade: &TokenStream, span: Span) -> Vec<TokenStream> {
    bounds.iter().filter_map(|bound| match bound { GenericBoundIr::Trait { path, modifier, lifetimes } => { let source = syn::LitStr::new(&path.source, span); let modifier = match modifier { crate::ir::TraitBoundModifierIr::None => quote!(#facade::expression::TraitBoundModifier::None), crate::ir::TraitBoundModifierIr::Maybe => quote!(#facade::expression::TraitBoundModifier::Maybe) }; let lifetimes = lifetimes.iter().map(|value| lifetime_expression(value, span, facade)); Some(quote!(#facade::expression::PredicateDescriptor::TypeBound { subject: #facade::expression::TypeExpression::SelfType, bounds: Box::new([#facade::expression::TypeExpression::Concrete(#facade::expression::ConcreteTypeExpression { path: Box::new([#source.into()]), arguments: Box::new([]), diagnostic: #facade::expression::DiagnosticText::from(#source) })]), bound_modifiers: Box::new([#modifier]), higher_ranked_lifetimes: Box::new([#(#lifetimes),*]), diagnostic: #facade::expression::DiagnosticText::default() })) }, GenericBoundIr::Lifetime(value) => { let value = lifetime_expression(value, span, facade); Some(quote!(#facade::expression::PredicateDescriptor::TypeOutlives { ty: #facade::expression::TypeExpression::SelfType, lifetime: #value, diagnostic: #facade::expression::DiagnosticText::default() })) }, _ => None }).collect()
}

fn const_expression_value(value: &TokenStream, facade: &TokenStream) -> TokenStream {
    let source = value.to_string();
    if let Ok(value) = syn::parse2::<syn::Lit>(value.clone()) { match value { syn::Lit::Bool(value) => { let value = value.value; return quote!(#facade::expression::ConstExpression::Boolean(#value)); }, syn::Lit::Char(value) => { let value = value.value(); return quote!(#facade::expression::ConstExpression::Character(#value)); }, syn::Lit::Int(value) => { if let Ok(value) = value.base10_parse::<u128>() { return quote!(#facade::expression::ConstExpression::UnsignedInteger(#value)); } }, _ => {} } }
    if let Ok(syn::Expr::Unary(value)) = syn::parse2::<syn::Expr>(value.clone()) && matches!(value.op, syn::UnOp::Neg(_)) && let syn::Expr::Lit(literal) = *value.expr && let syn::Lit::Int(value) = literal.lit && let Ok(value) = value.base10_parse::<i128>() { let value = -value; return quote!(#facade::expression::ConstExpression::SignedInteger(#value)); }
    let source = syn::LitStr::new(&source, Span::call_site());
    quote!(compile_error!(concat!("unsupported const expression in #[reflect] trait: ", #source)))
}

/// Converts the literal const-default subset that has a structural runtime representation.
fn const_expression(value: &TokenStream, facade: &TokenStream) -> TokenStream {
    let expression = syn::parse2::<syn::Expr>(value.clone());
    let expression = match expression {
        Ok(expression) => expression,
        Err(_) => return unsupported_const_default(value),
    };
    match expression {
        syn::Expr::Lit(expression) => match expression.lit {
            syn::Lit::Bool(value) => {
                let value = value.value;
                quote!(Some(#facade::expression::ConstExpression::Boolean(#value)))
            }
            syn::Lit::Char(value) => {
                let value = value.value();
                quote!(Some(#facade::expression::ConstExpression::Character(#value)))
            }
            syn::Lit::Int(value) => integer_const_expression(&value, false, facade)
                .unwrap_or_else(|| unsupported_const_default(value.to_token_stream())),
            _ => unsupported_const_default(value),
        },
        syn::Expr::Unary(expression) if matches!(expression.op, syn::UnOp::Neg(_)) => {
            match expression.expr.as_ref() {
                syn::Expr::Lit(expression) => match &expression.lit {
                    syn::Lit::Int(value) => integer_const_expression(value, true, facade)
                        .unwrap_or_else(|| unsupported_const_default(value.to_token_stream())),
                    _ => unsupported_const_default(value),
                },
                _ => unsupported_const_default(value),
            }
        }
        _ => unsupported_const_default(value),
    }
}

/// Converts an integer literal without relying on its whitespace-normalized token rendering.
fn integer_const_expression(
    value: &syn::LitInt,
    negative: bool,
    facade: &TokenStream,
) -> Option<TokenStream> {
    let suffix = value.suffix();
    let signed = negative || matches!(suffix, "i8" | "i16" | "i32" | "i64" | "i128" | "isize");
    if signed {
        let magnitude = value.base10_parse::<i128>().ok()?;
        let value = if negative { magnitude.checked_neg()? } else { magnitude };
        Some(quote!(Some(#facade::expression::ConstExpression::SignedInteger(#value))))
    } else {
        let value = value.base10_parse::<u128>().ok()?;
        Some(quote!(Some(#facade::expression::ConstExpression::UnsignedInteger(#value))))
    }
}

/// Emits a deterministic compile error for const defaults without a runtime structural value.
fn unsupported_const_default(value: impl quote::ToTokens) -> TokenStream {
    let source = syn::LitStr::new(&value.into_token_stream().to_string(), Span::call_site());
    quote!(compile_error!(concat!("unsupported non-literal const default in #[reflect] trait: ", #source)))
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;

    use super::const_expression;

    /// Verifies that literals retain their structural value rather than their token spelling.
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

    /// Verifies that symbolic defaults fail explicitly instead of becoming symbolic values.
    #[test]
    fn test_const_default_rejects_non_literal_expression() {
        let value: TokenStream = quote!(DEFAULT_LIMIT);
        let rendered = const_expression(&value, &quote!(qubit_reflect)).to_string();

        assert!(rendered.contains("unsupported non-literal const default"));
        assert!(rendered.contains("DEFAULT_LIMIT"));
    }
}
