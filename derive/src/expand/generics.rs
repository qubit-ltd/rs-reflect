// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Expansion helpers for generic root-instance metadata.

use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;

use crate::ir::GenericBoundIr;
use crate::ir::GenericKindIr;
use crate::ir::HelperName;
use crate::ir::PathArgumentIr;
use crate::ir::PathArgumentsIr;
use crate::ir::TypeDeclarationIr;
use crate::ir::TypeDeclarationKindIr;
use crate::ir::TypeIr;
use crate::ir::TypeKindIr;

/// Emits the concrete generic view for the current monomorphized root.
pub(crate) fn concrete_descriptor(declaration: &TypeDeclarationIr, facade: &TokenStream) -> TokenStream {
    if declaration.generics.params.is_empty() {
        return TokenStream::new();
    }
    let definition = super::traits::generic_definition(&declaration.generics, declaration.span, facade);
    let mut arguments = Vec::new();
    let mut definition_indices = Vec::new();
    let mut type_arguments = Vec::new();
    let mut const_argument_values = Vec::new();
    for (definition_index, parameter) in declaration.generics.params.iter().enumerate() {
        match parameter.kind {
            GenericKindIr::Lifetime => {}
            GenericKindIr::Type => {
                let name = syn::Ident::new(&parameter.name, parameter.span);
                arguments.push(quote!(#facade::expression::GenericArgument::Type(
                    #facade::expression::TypeExpression::Parameter(
                        stringify!(#name).into(),
                    ),
                )));
                definition_indices.push(definition_index);
                type_arguments.push(quote!({
                    use #facade::__private::codegen_v1::descriptor::ResolveReflectArgument as _;
                    let probe = #facade::__private::codegen_v1::descriptor::ReflectArgumentProbe::<#name>::new();
                    (&probe).resolve_reflect_argument()
                }));
                const_argument_values.push(quote!(None));
            }
            GenericKindIr::Const => {
                let const_type = parameter
                    .const_type
                    .as_ref()
                    .expect("const generic parameters retain their declared type");
                let const_type_tokens = &const_type.tokens;
                let name = syn::Ident::new(&parameter.name, parameter.span);
                let declared_type = super::traits::type_expression(const_type, facade);
                arguments.push(quote!(#facade::expression::GenericArgument::Const(
                    #facade::expression::ConstGenericArgument::new(
                        #declared_type,
                        #facade::__private::codegen_v1::descriptor::const_argument_expression::<#const_type_tokens>(#name),
                        #facade::__private::codegen_v1::descriptor::const_argument_diagnostic::<#const_type_tokens>(#name),
                    ),
                )));
                definition_indices.push(definition_index);
                type_arguments.push(quote!(None));
                const_argument_values.push(quote!(Some(
                    (|| #facade::__private::codegen_v1::descriptor::const_argument_owned::<#const_type_tokens>(#name))
                        as fn() -> #facade::value::ReflectedOwned
                )));
            }
        }
    }
    quote!({
        static DEFINITION: ::std::sync::OnceLock<
            #facade::expression::GenericDefinitionDescriptor,
        > = ::std::sync::OnceLock::new();
        #facade::descriptor::ConcreteGenericDescriptor::new_with_runtime_arguments(
            DEFINITION.get_or_init(|| #definition),
            ::std::boxed::Box::leak(::std::vec![#(#arguments),*].into_boxed_slice()),
            ::std::boxed::Box::leak(::std::vec![#(#definition_indices),*].into_boxed_slice()),
            ::std::boxed::Box::leak(::std::vec![#(#type_arguments),*].into_boxed_slice()),
            ::std::boxed::Box::leak(::std::vec![#(#const_argument_values),*].into_boxed_slice()),
        )
    })
}

/// Returns the stable hidden provider name shared with facade macros that
/// augment one reflected generic declaration.
pub(crate) fn definition_provider_name(name: &syn::Ident) -> syn::Ident {
    let source = name.to_string();
    let mut snake = String::new();
    for (index, character) in source.chars().enumerate() {
        if character.is_ascii_uppercase() {
            if index != 0 {
                snake.push('_');
            }
            snake.push(character.to_ascii_lowercase());
        } else {
            snake.push(character);
        }
    }
    format_ident!("__qubit_reflect_generic_definition_{}", snake,)
}

/// Emits a non-generic provider for the declaration-level generic metadata.
///
/// Unlike `Reflect::type_descriptor`, this provider can be called without
/// choosing a concrete monomorph. Domain derive crates use it to register one
/// template while still reusing reflection's canonical generic IR.
pub(crate) fn definition_provider(declaration: &TypeDeclarationIr, facade: &TokenStream) -> TokenStream {
    if declaration.generics.params.is_empty() {
        return TokenStream::new();
    }
    let function = definition_provider_name(&declaration.name);
    let definition = super::traits::generic_definition(&declaration.generics, declaration.span, facade);
    quote! {
        #[doc(hidden)]
        fn #function() -> &'static #facade::expression::GenericDefinitionDescriptor {
            static DEFINITION: ::std::sync::OnceLock<
                #facade::expression::GenericDefinitionDescriptor,
            > = ::std::sync::OnceLock::new();
            DEFINITION.get_or_init(|| #definition)
        }
    }
}

/// Returns every visible field type that generated descriptor navigation must
/// be able to reflect as a complete Rust type.
pub(crate) fn reflected_field_types(declaration: &TypeDeclarationIr) -> Vec<&TypeIr> {
    if declaration
        .attributes
        .iter()
        .any(|attribute| attribute.name == HelperName::Opaque)
    {
        return Vec::new();
    }
    let mut fields: Vec<_> = match declaration.kind {
        TypeDeclarationKindIr::Struct | TypeDeclarationKindIr::Union => {
            declaration.fields.iter().filter_map(reflected_field_type).collect()
        }
        TypeDeclarationKindIr::Enum => declaration
            .variants
            .iter()
            .flat_map(|variant| variant.fields.iter().filter_map(reflected_field_type))
            .collect(),
    };
    fields.retain(|field_type| {
        declaration
            .generics
            .params
            .iter()
            .any(|parameter| match parameter.kind {
                GenericKindIr::Type => type_uses_parameter(field_type, &parameter.name),
                GenericKindIr::Lifetime => type_uses_lifetime(field_type, &parameter.name),
                GenericKindIr::Const => type_uses_const(field_type, &parameter.name),
            })
    });
    fields
}

/// Returns one field's complete type when its descriptor is source-visible.
fn reflected_field_type(field: &crate::ir::FieldIr) -> Option<&TypeIr> {
    (!field
        .attributes
        .iter()
        .any(|attribute| attribute.name == HelperName::Opaque))
    .then_some(&field.ty)
}

/// Returns parameters nested only through reflection-transparent builtin type
/// constructors whose public reflection implementations require the nested
/// argument itself to implement `Reflect`.
pub(crate) fn transparently_reflected_type_parameters(declaration: &TypeDeclarationIr) -> Vec<syn::Ident> {
    declaration
        .generics
        .params
        .iter()
        .filter(|parameter| parameter.kind == GenericKindIr::Type)
        .filter(|parameter| {
            reflected_field_types(declaration)
                .into_iter()
                .any(|field_type| transparent_type_uses_parameter(field_type, &parameter.name))
        })
        .map(|parameter| syn::Ident::new(&parameter.name, parameter.span))
        .collect()
}

/// Returns whether `parameter` occurs through constructors whose reflected
/// metadata exposes all relevant nested type arguments.
fn transparent_type_uses_parameter(ty: &TypeIr, parameter: &str) -> bool {
    if type_is_parameter(ty, parameter) {
        return true;
    }
    match &ty.kind {
        TypeKindIr::Path(path) if path.qualified_self.is_none() => {
            let Some(segment) = path.segments.last() else {
                return false;
            };
            let PathArgumentsIr::AngleBracketed(arguments) = &segment.arguments else {
                return false;
            };
            let relevant_types = arguments.iter().filter_map(|argument| match argument {
                PathArgumentIr::Type(ty) => Some(ty),
                _ => None,
            });
            match transparent_constructor_arity(path) {
                Some(1) => relevant_types
                    .take(1)
                    .any(|ty| transparent_type_uses_parameter(ty, parameter)),
                Some(2) => relevant_types
                    .take(2)
                    .any(|ty| transparent_type_uses_parameter(ty, parameter)),
                Some(_) | None => false,
            }
        }
        TypeKindIr::Reference { element, .. }
        | TypeKindIr::Slice(element)
        | TypeKindIr::Pointer { element, .. }
        | TypeKindIr::Array { element, .. } => transparent_type_uses_parameter(element, parameter),
        TypeKindIr::Tuple(elements) => elements
            .iter()
            .any(|element| transparent_type_uses_parameter(element, parameter)),
        TypeKindIr::BareFunction { inputs, output, .. } => {
            inputs
                .iter()
                .any(|input| transparent_type_uses_parameter(input, parameter))
                || output
                    .as_deref()
                    .is_some_and(|output| transparent_type_uses_parameter(output, parameter))
        }
        TypeKindIr::Path(_)
        | TypeKindIr::TraitObject { .. }
        | TypeKindIr::ImplTrait { .. }
        | TypeKindIr::Never
        | TypeKindIr::Infer
        | TypeKindIr::Macro
        | TypeKindIr::Other => false,
    }
}

/// Returns the reflected type-argument arity for syntactically unambiguous
/// standard-library container paths.
fn transparent_constructor_arity(path: &crate::ir::PathIr) -> Option<usize> {
    let segments: Vec<_> = path.segments.iter().map(|segment| segment.name.as_str()).collect();
    match segments.as_slice() {
        ["std" | "alloc", "vec", "Vec"]
        | ["std" | "alloc", "boxed", "Box"]
        | ["std" | "alloc", "rc", "Rc"]
        | ["std" | "alloc", "sync", "Arc"]
        | ["std" | "core", "option", "Option"]
        | ["std" | "alloc", "collections", "BTreeSet"]
        | ["std", "collections", "HashSet"] => Some(1),
        ["std" | "alloc", "collections", "BTreeMap"] | ["std", "collections", "HashMap"] => Some(2),
        _ => None,
    }
}

/// Returns whether one structured type node refers to `parameter`.
fn type_uses_parameter(ty: &TypeIr, parameter: &str) -> bool {
    match &ty.kind {
        TypeKindIr::Path(path) => {
            path.qualified_self
                .as_ref()
                .is_some_and(|qualified| type_uses_parameter(&qualified.ty, parameter))
                || path.segments.first().is_some_and(|segment| segment.name == parameter)
                || path
                    .segments
                    .iter()
                    .any(|segment| path_arguments_use_parameter(&segment.arguments, parameter))
        }
        TypeKindIr::Reference { element, .. } | TypeKindIr::Slice(element) | TypeKindIr::Pointer { element, .. } => {
            type_uses_parameter(element, parameter)
        }
        TypeKindIr::Tuple(elements) => elements.iter().any(|element| type_uses_parameter(element, parameter)),
        TypeKindIr::Array { element, .. } => type_uses_parameter(element, parameter),
        TypeKindIr::BareFunction { inputs, output, .. } => {
            inputs.iter().any(|input| type_uses_parameter(input, parameter))
                || output
                    .as_deref()
                    .is_some_and(|output| type_uses_parameter(output, parameter))
        }
        TypeKindIr::TraitObject { bounds, .. } | TypeKindIr::ImplTrait { bounds } => {
            bounds.iter().any(|bound| bound_uses_parameter(bound, parameter))
        }
        TypeKindIr::Never | TypeKindIr::Infer | TypeKindIr::Macro | TypeKindIr::Other => false,
    }
}

/// Returns whether path arguments contain `parameter` in a type position.
fn path_arguments_use_parameter(arguments: &PathArgumentsIr, parameter: &str) -> bool {
    match arguments {
        PathArgumentsIr::None => false,
        PathArgumentsIr::AngleBracketed(arguments) => arguments.iter().any(|argument| match argument {
            PathArgumentIr::Type(ty) | PathArgumentIr::AssociatedType { ty, .. } => type_uses_parameter(ty, parameter),
            PathArgumentIr::Constraint { bounds, .. } => {
                bounds.iter().any(|bound| bound_uses_parameter(bound, parameter))
            }
            PathArgumentIr::Lifetime(_)
            | PathArgumentIr::Const(_)
            | PathArgumentIr::AssociatedConst { .. }
            | PathArgumentIr::Other(_) => false,
        }),
        PathArgumentsIr::Parenthesized { inputs, output } => {
            inputs.iter().any(|input| type_uses_parameter(input, parameter))
                || output
                    .as_deref()
                    .is_some_and(|output| type_uses_parameter(output, parameter))
        }
    }
}

/// Returns whether one trait bound path contains `parameter`.
fn bound_uses_parameter(bound: &GenericBoundIr, parameter: &str) -> bool {
    let GenericBoundIr::Trait { path, .. } = bound else {
        return false;
    };
    path.qualified_self
        .as_ref()
        .is_some_and(|qualified| type_uses_parameter(&qualified.ty, parameter))
        || path
            .segments
            .iter()
            .any(|segment| path_arguments_use_parameter(&segment.arguments, parameter))
}

/// Returns whether one structured type node refers to an outer lifetime.
fn type_uses_lifetime(ty: &TypeIr, lifetime: &str) -> bool {
    match &ty.kind {
        TypeKindIr::Path(path) => {
            path.qualified_self
                .as_ref()
                .is_some_and(|qualified| type_uses_lifetime(&qualified.ty, lifetime))
                || path
                    .segments
                    .iter()
                    .any(|segment| path_arguments_use_lifetime(&segment.arguments, lifetime))
        }
        TypeKindIr::Reference {
            lifetime: reference_lifetime,
            element,
            ..
        } => {
            reference_lifetime
                .as_deref()
                .is_some_and(|candidate| candidate.trim_start_matches('\'') == lifetime)
                || type_uses_lifetime(element, lifetime)
        }
        TypeKindIr::Slice(element) | TypeKindIr::Pointer { element, .. } => type_uses_lifetime(element, lifetime),
        TypeKindIr::Tuple(elements) => elements.iter().any(|element| type_uses_lifetime(element, lifetime)),
        TypeKindIr::Array { element, .. } => type_uses_lifetime(element, lifetime),
        TypeKindIr::BareFunction {
            lifetimes,
            inputs,
            output,
            ..
        } => {
            !lifetimes.iter().any(|bound| bound.trim_start_matches('\'') == lifetime)
                && (inputs.iter().any(|input| type_uses_lifetime(input, lifetime))
                    || output
                        .as_deref()
                        .is_some_and(|output| type_uses_lifetime(output, lifetime)))
        }
        TypeKindIr::TraitObject { .. }
        | TypeKindIr::ImplTrait { .. }
        | TypeKindIr::Never
        | TypeKindIr::Infer
        | TypeKindIr::Macro
        | TypeKindIr::Other => false,
    }
}

/// Returns whether path arguments contain one outer lifetime.
fn path_arguments_use_lifetime(arguments: &PathArgumentsIr, lifetime: &str) -> bool {
    match arguments {
        PathArgumentsIr::None => false,
        PathArgumentsIr::AngleBracketed(arguments) => arguments.iter().any(|argument| match argument {
            PathArgumentIr::Lifetime(candidate) => candidate.trim_start_matches('\'') == lifetime,
            PathArgumentIr::Type(ty) | PathArgumentIr::AssociatedType { ty, .. } => type_uses_lifetime(ty, lifetime),
            PathArgumentIr::Const(_)
            | PathArgumentIr::AssociatedConst { .. }
            | PathArgumentIr::Constraint { .. }
            | PathArgumentIr::Other(_) => false,
        }),
        PathArgumentsIr::Parenthesized { inputs, output } => {
            inputs.iter().any(|input| type_uses_lifetime(input, lifetime))
                || output
                    .as_deref()
                    .is_some_and(|output| type_uses_lifetime(output, lifetime))
        }
    }
}

/// Returns whether one structured type node refers to an outer const generic.
fn type_uses_const(ty: &TypeIr, parameter: &str) -> bool {
    if type_is_parameter(ty, parameter) {
        return true;
    }
    match &ty.kind {
        TypeKindIr::Path(path) => {
            path.qualified_self
                .as_ref()
                .is_some_and(|qualified| type_uses_const(&qualified.ty, parameter))
                || path
                    .segments
                    .iter()
                    .any(|segment| path_arguments_use_const(&segment.arguments, parameter))
        }
        TypeKindIr::Reference { element, .. } | TypeKindIr::Slice(element) | TypeKindIr::Pointer { element, .. } => {
            type_uses_const(element, parameter)
        }
        TypeKindIr::Tuple(elements) => elements.iter().any(|element| type_uses_const(element, parameter)),
        TypeKindIr::Array { element, length } => {
            token_is_parameter(length, parameter) || type_uses_const(element, parameter)
        }
        TypeKindIr::BareFunction { inputs, output, .. } => {
            inputs.iter().any(|input| type_uses_const(input, parameter))
                || output
                    .as_deref()
                    .is_some_and(|output| type_uses_const(output, parameter))
        }
        TypeKindIr::TraitObject { .. }
        | TypeKindIr::ImplTrait { .. }
        | TypeKindIr::Never
        | TypeKindIr::Infer
        | TypeKindIr::Macro
        | TypeKindIr::Other => false,
    }
}

/// Returns whether path arguments contain one outer const generic.
fn path_arguments_use_const(arguments: &PathArgumentsIr, parameter: &str) -> bool {
    match arguments {
        PathArgumentsIr::None => false,
        PathArgumentsIr::AngleBracketed(arguments) => arguments.iter().any(|argument| match argument {
            PathArgumentIr::Const(value) | PathArgumentIr::AssociatedConst { value, .. } => {
                token_is_parameter(value, parameter)
            }
            PathArgumentIr::Type(ty) | PathArgumentIr::AssociatedType { ty, .. } => type_uses_const(ty, parameter),
            PathArgumentIr::Lifetime(_) | PathArgumentIr::Constraint { .. } | PathArgumentIr::Other(_) => false,
        }),
        PathArgumentsIr::Parenthesized { inputs, output } => {
            inputs.iter().any(|input| type_uses_const(input, parameter))
                || output
                    .as_deref()
                    .is_some_and(|output| type_uses_const(output, parameter))
        }
    }
}

/// Returns whether a const-expression token is the direct parameter path.
fn token_is_parameter(tokens: &TokenStream, parameter: &str) -> bool {
    syn::parse2::<syn::ExprPath>(tokens.clone()).is_ok_and(|path| path.qself.is_none() && path.path.is_ident(parameter))
}

/// Returns whether `ty` is the direct generic parameter named `parameter`.
fn type_is_parameter(ty: &TypeIr, parameter: &str) -> bool {
    let TypeKindIr::Path(path) = &ty.kind else {
        return false;
    };
    path.qualified_self.is_none()
        && path.segments.len() == 1
        && path.segments[0].name == parameter
        && matches!(path.segments[0].arguments, PathArgumentsIr::None)
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::reflected_field_types;
    use crate::ir::DeclarationIr;
    use crate::ir::MacroKind;
    use crate::parse::parse_declaration;

    #[test]
    fn test_const_and_lifetime_only_fields_receive_complete_reflect_bounds() {
        let parsed = parse_declaration(
            MacroKind::Derive,
            quote!(),
            quote! {
                struct ConditionalFields<'a, const N: usize> {
                    borrowed: &'a str,
                    bytes: [u8; N],
                    conditional: Conditional<N>,
                }
            },
        )
        .expect("the conditional generic fields must parse");
        let DeclarationIr::Type(declaration) = parsed.declaration else {
            panic!("the fixture must parse as a type declaration");
        };
        let field_types: Vec<_> = reflected_field_types(&declaration)
            .into_iter()
            .map(|field_type| field_type.source.as_str())
            .collect();

        assert_eq!(field_types, ["& 'a str", "[u8 ; N]", "Conditional < N >"]);
    }
}
