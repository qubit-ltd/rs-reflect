// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Expansion of reflected struct declarations.

use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;

use crate::expand::ExpansionContext;
use crate::expand::generic_environment::GenericEnvironment;
use crate::ir::FieldShapeIr;
use crate::ir::GenericKindIr;
use crate::ir::HelperName;
use crate::ir::PathArgumentsIr;
use crate::ir::TypeDeclarationIr;
use crate::ir::TypeDeclarationKindIr;
use crate::ir::VisibilityIr;

/// Expands a concrete struct into a static root descriptor and safe field
/// adapters.
///
/// # Parameters
///
/// - `declaration`: Validated struct declaration and its retained source facts.
/// - `context`: Expansion configuration and runtime facade resolver.
///
/// # Returns
///
/// Returns generated items or a diagnostic if retained generic syntax is
/// malformed.
///
/// # Errors
///
/// Returns a syntax diagnostic when the declaration is not a struct or retained
/// generic syntax cannot be parsed.
pub(crate) fn expand(
    declaration: TypeDeclarationIr,
    context: &ExpansionContext,
) -> syn::Result<TokenStream> {
    return expand_v4(declaration, context);
}

fn expand_v4(
    declaration: TypeDeclarationIr,
    context: &ExpansionContext,
) -> syn::Result<TokenStream> {
    if declaration.kind != TypeDeclarationKindIr::Struct {
        return Err(syn::Error::new(
            declaration.span,
            "cannot expand Reflect for non-struct declaration",
        ));
    }
    let facade = context.facade().clone();
    let name = declaration.name.clone();
    let fingerprint = context.fingerprint(&declaration.retained_tokens.to_string());
    let module = format_ident!("__qubit_reflect_v4_struct_{fingerprint:016x}");
    let marker = format_ident!("TypeDefinitionMarker");
    let mut generics = super::generics::parse_type_generics(&declaration)?;
    let visible_fields: Vec<_> = declaration
        .fields
        .iter()
        .filter(|field| {
            !field
                .attributes
                .iter()
                .any(|attribute| attribute.name == HelperName::Skip)
        })
        .collect();
    let environment = GenericEnvironment::from_generics(&declaration.generics);
    let constructible = visible_fields
        .iter()
        .all(|field| supports_construction(&field.ty, &environment))
        && declaration.fields.iter().all(|field| {
            super::generics::field_cfg_attributes(&declaration, None, field.index).is_empty()
        })
        && visible_fields.len() == declaration.fields.len();
    {
        let where_clause = generics.make_where_clause();
        for parameter in declaration
            .generics
            .params
            .iter()
            .filter(|parameter| parameter.kind == GenericKindIr::Lifetime)
        {
            let lifetime = syn::Lifetime::new(&format!("'{}", parameter.name), parameter.span);
            where_clause
                .predicates
                .push(syn::parse_quote!(#lifetime: 'static));
        }
        for parameter in declaration
            .generics
            .params
            .iter()
            .filter(|parameter| parameter.kind == GenericKindIr::Type)
        {
            let parameter = syn::Ident::new(&parameter.name, parameter.span);
            where_clause
                .predicates
                .push(syn::parse_quote!(#parameter: 'static));
        }
        for field in visible_fields.iter().filter(|field| {
            supports_construction(&field.ty, &environment)
                && super::generics::field_cfg_attributes(&declaration, None, field.index).is_empty()
        }) {
            let ty = &field.ty.tokens;
            where_clause
                .predicates
                .push(syn::parse_quote!(#ty: #facade::__private::codegen_v4::descriptor::Reflect));
        }
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let mut field_functions = Vec::new();
    let mut field_metadata = Vec::new();
    let mut construct_fields = Vec::new();
    for field in &visible_fields {
        let cfg = super::generics::field_cfg_attributes(&declaration, None, field.index);
        let index = field.index;
        let get = format_ident!("get_field_{index}");
        let get_mut = format_ident!("get_mut_field_{index}");
        let ty = &field.ty.tokens;
        let member = field
            .name
            .as_ref()
            .map(|field_name| quote!(#field_name))
            .unwrap_or_else(|| {
                let index = syn::Index::from(index);
                quote!(#index)
            });
        let source_name = field
            .name
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| index.to_string());
        let visibility = v4_visibility(&field.visibility, &facade, field.span);
        let expression =
            super::expression_codegen::type_expression(&field.ty, &environment, &facade);
        field_functions.push(quote! {
            #(#cfg)*
            fn #get<'a>(value: &'a Self) -> &'a #ty { &value.#member }
            #(#cfg)*
            fn #get_mut<'a>(value: &'a mut Self) -> &'a mut #ty { &mut value.#member }
        });
        let read_only = field
            .attributes
            .iter()
            .any(|attribute| attribute.name == HelperName::ReadOnly);
        let mutable = !read_only;
        let access = quote!(#facade::__private::codegen_v4::access::FieldAccess::new(
            true, #mutable, #mutable, true,
        ));
        let adapter = quote!(::std::boxed::Box::leak(::std::boxed::Box::new(
            #facade::__private::codegen_v4::access::TypedFieldAdapter::<Self, #ty>::new(Self::#get, Self::#get_mut)
        )));
        let local_dynamic = if supports_construction(&field.ty, &environment) && cfg.is_empty() {
            quote!(.with_local_dynamic(#adapter))
        } else {
            TokenStream::new()
        };
        let thread_safe_dynamic = if constructible && declaration.generics.params.is_empty() {
            quote!(.with_thread_safe_dynamic(#adapter))
        } else {
            TokenStream::new()
        };
        field_metadata.push(quote! {
            #(#cfg)*
            #facade::__private::codegen_v4::access::FieldMetadata::typed::<Self, #ty>(
                #index, #source_name, #visibility, #expression,
                #adapter, #access,
            ) #local_dynamic #thread_safe_dynamic
        });
        let field_policy = if constructible {
            quote!(#facade::__private::codegen_v4::construct::ConstructionFieldPolicy::Required)
        } else {
            quote!(#facade::__private::codegen_v4::construct::ConstructionFieldPolicy::Unavailable(
                #facade::__private::codegen_v4::construct::ConstructionUnavailableReason::FieldNotReflectable,
            ))
        };
        let update_policy = if constructible {
            quote!(#facade::__private::codegen_v4::construct::UpdateFieldPolicy::Replaceable)
        } else {
            quote!(#facade::__private::codegen_v4::construct::UpdateFieldPolicy::Unavailable(
                #facade::__private::codegen_v4::construct::UpdateUnavailableReason::FieldNotReplaceable,
            ))
        };
        construct_fields.push(quote! {
            #(#cfg)*
            #facade::__private::codegen_v4::construct::ConstructField::typed::<#ty>(
                #source_name, #visibility, #field_policy, #update_policy,
            )
        });
    }
    let (provider_functions, providers, availability) =
        struct_provider(&declaration, &facade, constructible);
    field_functions.push(provider_functions);
    let struct_kind = match declaration.field_shape {
        crate::ir::FieldShapeIr::Unit => {
            quote!(#facade::__private::codegen_v4::expression::StructKind::Unit)
        }
        crate::ir::FieldShapeIr::Named => {
            quote!(#facade::__private::codegen_v4::expression::StructKind::Named)
        }
        crate::ir::FieldShapeIr::Unnamed => {
            quote!(#facade::__private::codegen_v4::expression::StructKind::Tuple)
        }
    };
    let kind = quote!(#facade::__private::codegen_v4::expression::TypeDefinitionKind::Struct(#struct_kind));
    let type_definition = if declaration.generics.params.is_empty() {
        quote!(None)
    } else {
        let generic_definition = super::expression_codegen::generic_definition(
            &declaration.generics,
            declaration.span,
            &facade,
        );
        let generic_definition =
            quote!(::std::boxed::Box::leak(::std::boxed::Box::new(#generic_definition)));
        let rust_name = name.to_string();
        quote!(Some(::std::boxed::Box::leak(::std::boxed::Box::new(
            #facade::__private::codegen_v4::expression::TypeDefinitionMetadata::new::<#module::#marker>(
                #rust_name, concat!(module_path!(), "::", stringify!(#name)), #kind, #generic_definition,
            )
        ))))
    };
    let (shape, construction_shape) = match declaration.field_shape {
        crate::ir::FieldShapeIr::Unit => (
            quote!(#facade::__private::codegen_v4::access::TypeKindMetadata::Struct),
            quote!(#facade::__private::codegen_v4::construct::ConstructionShape::Unit),
        ),
        crate::ir::FieldShapeIr::Named => (
            quote!(#facade::__private::codegen_v4::access::TypeKindMetadata::Struct),
            quote!(#facade::__private::codegen_v4::construct::ConstructionShape::Named),
        ),
        crate::ir::FieldShapeIr::Unnamed => (
            quote!(#facade::__private::codegen_v4::access::TypeKindMetadata::Tuple),
            quote!(#facade::__private::codegen_v4::construct::ConstructionShape::Positional),
        ),
    };
    let plans = quote! {
        let construct_fields = ::std::boxed::Box::leak(::std::vec![#(#construct_fields),*].into_boxed_slice());
        let plan = #facade::__private::codegen_v4::construct::ConstructionPlan::for_type::<Self>(
            #construction_shape,
            construct_fields,
            #providers,
            #availability,
        );
        ::std::boxed::Box::leak(::std::vec![plan].into_boxed_slice())
    };
    let shape_expr = quote!({
        let fields = ::std::boxed::Box::leak(::std::vec![#(#field_metadata),*].into_boxed_slice());
        ::std::boxed::Box::leak(::std::boxed::Box::new(#shape(fields)))
    });
    Ok(quote! {
        #[allow(non_camel_case_types)]
        mod #module { pub struct #marker; }
        impl #impl_generics #facade::__private::codegen_v4::descriptor::Reflect for #name #type_generics #where_clause {
            fn __reflect_registration() -> &'static #facade::__private::codegen_v4::descriptor::TypeRegistration<Self> {
                #(#field_functions)*
                #facade::__private::codegen_v4::descriptor::TypeRegistration::<Self>::get_or_init_with(|| {
                    #facade::__private::codegen_v4::descriptor::TypeRegistration::<Self>::with_shape(
                        #shape_expr, #type_definition, #plans,
                    )
                })
            }
        }
    })
}

pub(super) fn supports_construction(
    ty: &crate::ir::TypeIr,
    environment: &GenericEnvironment,
) -> bool {
    use crate::ir::TypeKindIr;
    match &ty.kind {
        TypeKindIr::Path(path) if path.qualified_self.is_none() && path.segments.len() == 1 => {
            let name = path.segments[0].name.as_str();
            environment.is_type_parameter(name)
                || matches!(
                    name,
                    "bool"
                        | "char"
                        | "str"
                        | "String"
                        | "i8"
                        | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "isize"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "usize"
                        | "f32"
                        | "f64"
                )
        }
        TypeKindIr::Tuple(values) => values
            .iter()
            .all(|value| supports_construction(value, environment)),
        _ => false,
    }
}

fn struct_provider(
    declaration: &TypeDeclarationIr,
    facade: &TokenStream,
    constructible: bool,
) -> (TokenStream, TokenStream, TokenStream) {
    if !constructible {
        return (
            TokenStream::new(),
            quote!(#facade::__private::codegen_v4::construct::ConstructionProviders::new::<Self>(None)),
            quote!(#facade::__private::codegen_v4::construct::ConstructionAvailability::Unavailable {
                reasons: ::std::vec![#facade::__private::codegen_v4::construct::ConstructionUnavailableReason::FieldNotReflectable].into_boxed_slice(),
            }),
        );
    }
    let fields: Vec<_> = declaration
        .fields
        .iter()
        .filter(|field| {
            !field
                .attributes
                .iter()
                .any(|attribute| attribute.name == HelperName::Skip)
        })
        .collect();
    let (local_functions, local_provider) =
        struct_provider_mode(declaration, facade, &fields, "local", quote!(Local));
    let (thread_safe_functions, thread_safe_provider) = if declaration.generics.params.is_empty() {
        struct_provider_mode(
            declaration,
            facade,
            &fields,
            "thread_safe",
            quote!(ThreadSafe),
        )
    } else {
        (TokenStream::new(), TokenStream::new())
    };
    let make_functions = quote!(#local_functions #thread_safe_functions);
    let local_providers = quote!(#facade::__private::codegen_v4::construct::ConstructionProviders::new::<Self>(Some(#local_provider)));
    let provider = if declaration.generics.params.is_empty() {
        quote!({
            let providers = #local_providers;
            providers.with_thread_safe::<Self>(#thread_safe_provider)
        })
    } else {
        local_providers
    };
    (
        make_functions,
        provider,
        quote!(#facade::__private::codegen_v4::construct::ConstructionAvailability::Available),
    )
}

fn struct_provider_mode(
    declaration: &TypeDeclarationIr,
    facade: &TokenStream,
    fields: &[&crate::ir::FieldIr],
    suffix: &str,
    mode: TokenStream,
) -> (TokenStream, TokenStream) {
    let construct_name = format_ident!("construct_{suffix}");
    let update_name = format_ident!("update_{suffix}");
    let mut construct_bindings = Vec::new();
    let mut initializers = Vec::new();
    let mut updates = Vec::new();
    for (position, field) in fields.iter().enumerate() {
        let variable = format_ident!("field_{position}");
        let ty = &field.ty.tokens;
        let index = syn::Index::from(position);
        construct_bindings.push(quote!(let #variable = fields.get_mut(#index).and_then(Option::take).expect("validated construction field must be present")
            .downcast::<#ty>().unwrap_or_else(|_| unreachable!("validated construction field type"));));
        let member = field
            .name
            .as_ref()
            .map(|name| quote!(#name))
            .unwrap_or_else(|| quote!(#index));
        if field.name.is_some() {
            initializers.push(quote!(#member: #variable));
        } else {
            initializers.push(quote!(#variable));
        }
        updates.push(quote! {
            if let Some(replacement) = fields.get_mut(#index).and_then(Option::take) {
                value.#member = replacement.downcast::<#ty>()
                    .unwrap_or_else(|_| unreachable!("validated update field type"));
            }
        });
    }
    let construct_value = match declaration.field_shape {
        crate::ir::FieldShapeIr::Unit => quote!(Self),
        crate::ir::FieldShapeIr::Named => quote!(Self { #(#initializers),* }),
        crate::ir::FieldShapeIr::Unnamed => quote!(Self(#(#initializers),*)),
    };
    let functions = quote! {
        fn #construct_name(mut fields: ::std::vec::Vec<Option<#facade::__private::codegen_v4::value::DynamicOwned<#facade::__private::codegen_v4::value::#mode>>>) -> Self {
            #(#construct_bindings)*
            #construct_value
        }
        fn #update_name(mut value: Self, mut fields: ::std::vec::Vec<Option<#facade::__private::codegen_v4::value::DynamicOwned<#facade::__private::codegen_v4::value::#mode>>>) -> Self {
            #(#updates)*
            value
        }
    };
    let provider = quote! {
        ::std::boxed::Box::leak(::std::boxed::Box::new(
            #facade::__private::codegen_v4::construct::ConstructProvider::<Self, #facade::__private::codegen_v4::value::#mode>::new(
                Self::#construct_name, Self::#update_name,
            )
        ))
    };
    (functions, provider)
}

pub(super) fn v4_visibility(
    visibility: &VisibilityIr,
    facade: &TokenStream,
    span: Span,
) -> TokenStream {
    match visibility {
        VisibilityIr::Public => quote!(#facade::__private::codegen_v4::access::Visibility::Public),
        VisibilityIr::Crate => quote!(#facade::__private::codegen_v4::access::Visibility::Crate),
        VisibilityIr::Super => {
            quote!(#facade::__private::codegen_v4::access::Visibility::Restricted("super"))
        }
        VisibilityIr::SelfValue | VisibilityIr::Inherited => {
            quote!(#facade::__private::codegen_v4::access::Visibility::Private)
        }
        VisibilityIr::Restricted(path) => {
            let path = syn::LitStr::new(&path.source, span);
            quote!(#facade::__private::codegen_v4::access::Visibility::Restricted(#path))
        }
    }
}

#[allow(dead_code)]
fn expand_legacy(
    declaration: TypeDeclarationIr,
    context: &ExpansionContext,
) -> syn::Result<TokenStream> {
    if declaration.kind != TypeDeclarationKindIr::Struct {
        return Err(syn::Error::new(
            declaration.span,
            "cannot expand Reflect for non-struct declaration",
        ));
    }
    let facade = context.facade().clone();
    let name = declaration.name.clone();
    let fingerprint = context.fingerprint(&declaration.retained_tokens.to_string());
    let registration_module = format_ident!("__qubit_reflect_type_registration_{fingerprint:016x}");
    let opaque_root = declaration
        .attributes
        .iter()
        .any(|attribute| attribute.name == HelperName::Opaque);
    let thread_safe = declaration
        .attributes
        .iter()
        .any(|attribute| attribute.name == HelperName::ThreadSafe);
    let reflected_field_types = super::generics::reflected_field_types(&declaration);
    let transparently_reflected_parameters =
        super::generics::transparently_reflected_type_parameters(&declaration);
    let type_parameter_names: Vec<_> = declaration
        .generics
        .params
        .iter()
        .filter(|parameter| parameter.kind == GenericKindIr::Type)
        .map(|parameter| syn::Ident::new(&parameter.name, parameter.span))
        .collect();
    let mut generics = super::generics::parse_type_generics(&declaration)?;
    {
        let where_clause = generics.make_where_clause();
        for parameter in declaration
            .generics
            .params
            .iter()
            .filter(|parameter| parameter.kind == GenericKindIr::Lifetime)
        {
            let lifetime = syn::Lifetime::new(&format!("'{}", parameter.name), parameter.span);
            where_clause
                .predicates
                .push(syn::parse_quote!(#lifetime: 'static));
        }
        for parameter in &type_parameter_names {
            where_clause
                .predicates
                .push(syn::parse_quote!(#parameter: 'static));
        }
        for field_type in &reflected_field_types {
            let field_type = &field_type.tokens;
            where_clause
                .predicates
                .push(syn::parse_quote!(#field_type: #facade::__private::codegen_v3::Reflect));
        }
        for parameter in &transparently_reflected_parameters {
            where_clause
                .predicates
                .push(syn::parse_quote!(#parameter: #facade::__private::codegen_v3::Reflect));
        }
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let self_type = quote!(#name #type_generics);
    let query_name = declaration
        .attributes
        .iter()
        .find_map(|attribute| attribute.rename())
        .unwrap_or(&name.to_string())
        .to_owned();
    let capability_function = format_ident!("__qubit_reflect_capabilities_{fingerprint:016x}");
    let capability_resolver = quote!(<#self_type>::#capability_function);
    let capability_definition = capabilities(&declaration, &facade, &capability_function);
    let adapter_definitions: Vec<_> = if opaque_root {
        Vec::new()
    } else {
        declaration.fields.iter().map(|field| {
        let cfg_attributes = super::generics::field_cfg_attributes(&declaration, None, field.index);
        let index = field.index;
        let get = format_ident!("__qubit_reflect_get_field_{index}");
        let get_mut = format_ident!("__qubit_reflect_get_mut_field_{index}");
        let set = format_ident!("__qubit_reflect_set_field_{index}");
        let get_thread_safe = format_ident!("__qubit_reflect_get_field_{index}_thread_safe");
        let get_mut_thread_safe = format_ident!("__qubit_reflect_get_mut_field_{index}_thread_safe");
        let set_thread_safe = format_ident!("__qubit_reflect_set_field_{index}_thread_safe");
        let ty = &field.ty.tokens;
        let access = match &field.name {
            Some(name) => quote!(#name),
            None => {
                let index = syn::Index::from(index);
                quote!(#index)
            }
        };
        let thread_safe_adapters = thread_safe.then(|| quote! {
            fn #get_thread_safe<'__qubit_reflect>(target: #facade::__private::codegen_v3::value::DynamicRef<'__qubit_reflect, #facade::__private::codegen_v3::value::ThreadSafe>)
                -> ::core::result::Result<#facade::__private::codegen_v3::value::DynamicRef<'__qubit_reflect, #facade::__private::codegen_v3::value::ThreadSafe>, #facade::__private::codegen_v3::access::FieldAccessError>
            {
                let value = target.downcast::<#self_type>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field target"));
                Ok(#facade::__private::codegen_v3::value::DynamicRef::<#facade::__private::codegen_v3::value::ThreadSafe>::new(&value.#access))
            }

            fn #get_mut_thread_safe<'__qubit_reflect>(target: #facade::__private::codegen_v3::value::DynamicMut<'__qubit_reflect, #facade::__private::codegen_v3::value::ThreadSafe>)
                -> ::core::result::Result<#facade::__private::codegen_v3::value::DynamicMut<'__qubit_reflect, #facade::__private::codegen_v3::value::ThreadSafe>, #facade::__private::codegen_v3::access::FieldAccessError>
            {
                let value = target.downcast::<#self_type>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field target"));
                Ok(#facade::__private::codegen_v3::value::DynamicMut::<#facade::__private::codegen_v3::value::ThreadSafe>::new(&mut value.#access))
            }

            fn #set_thread_safe(
                target: #facade::__private::codegen_v3::value::DynamicMut<'_, #facade::__private::codegen_v3::value::ThreadSafe>,
                replacement: #facade::__private::codegen_v3::value::DynamicOwned<#facade::__private::codegen_v3::value::ThreadSafe>,
            ) -> ::core::result::Result<(), #facade::__private::codegen_v3::access::FieldAccessError> {
                let value = target.downcast::<#self_type>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field target"));
                let replacement = replacement.downcast::<#ty>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field value"));
                value.#access = replacement;
                Ok(())
            }
        });
        quote! {
            #(#cfg_attributes)*
            fn #get<'__qubit_reflect>(target: #facade::__private::codegen_v3::value::DynamicRef<'__qubit_reflect, #facade::__private::codegen_v3::value::Local>)
                -> ::core::result::Result<#facade::__private::codegen_v3::value::DynamicRef<'__qubit_reflect, #facade::__private::codegen_v3::value::Local>, #facade::__private::codegen_v3::access::FieldAccessError>
            {
                let value = target.downcast::<#self_type>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field target"));
                Ok(#facade::__private::codegen_v3::value::DynamicRef::<#facade::__private::codegen_v3::value::Local>::new(&value.#access))
            }

            #(#cfg_attributes)*
            fn #get_mut<'__qubit_reflect>(target: #facade::__private::codegen_v3::value::DynamicMut<'__qubit_reflect, #facade::__private::codegen_v3::value::Local>)
                -> ::core::result::Result<#facade::__private::codegen_v3::value::DynamicMut<'__qubit_reflect, #facade::__private::codegen_v3::value::Local>, #facade::__private::codegen_v3::access::FieldAccessError>
            {
                let value = target.downcast::<#self_type>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field target"));
                Ok(#facade::__private::codegen_v3::value::DynamicMut::<#facade::__private::codegen_v3::value::Local>::new(&mut value.#access))
            }

            #(#cfg_attributes)*
            fn #set(target: #facade::__private::codegen_v3::value::DynamicMut<'_, #facade::__private::codegen_v3::value::Local>, replacement: #facade::__private::codegen_v3::value::DynamicOwned<#facade::__private::codegen_v3::value::Local>)
                -> ::core::result::Result<(), #facade::__private::codegen_v3::access::FieldAccessError>
            {
                let value = target.downcast::<#self_type>()
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field target"));
                let replacement = #facade::__private::codegen_v3::value::DynamicOwned::<#facade::__private::codegen_v3::value::Local>::downcast::<#ty>(replacement)
                    .unwrap_or_else(|_| unreachable!("descriptor validated derived field value"));
                value.#access = replacement;
                Ok(())
            }

            #thread_safe_adapters
        }
        }).collect()
    };
    let construction_adapters =
        if opaque_root || super::generics::has_skipped_field(&declaration, None) {
            TokenStream::new()
        } else {
            super::construction::struct_adapters(&declaration, &facade)
        };
    let construction_descriptor = super::construction::struct_descriptor(&facade);
    let fields: Vec<_> = if opaque_root {
        Vec::new()
    } else {
        declaration.fields.iter().map(|field| {
        let cfg_attributes = super::generics::field_cfg_attributes(&declaration, None, field.index);
        let index = field.index;
        let get = format_ident!("__qubit_reflect_get_field_{index}");
        let get_mut = format_ident!("__qubit_reflect_get_mut_field_{index}");
        let set = format_ident!("__qubit_reflect_set_field_{index}");
        let get_thread_safe = format_ident!("__qubit_reflect_get_field_{index}_thread_safe");
        let get_mut_thread_safe = format_ident!("__qubit_reflect_get_mut_field_{index}_thread_safe");
        let set_thread_safe = format_ident!("__qubit_reflect_set_field_{index}_thread_safe");
        let rust_name = field.name.as_ref().map(|name| name.to_string());
        let rust_name = match rust_name { Some(name) => quote!(Some(#name)), None => quote!(None) };
        let query_name = field.name.as_ref().map(|name| {
            field.attributes.iter().find_map(|attribute| attribute.rename()).unwrap_or(&name.to_string()).to_owned()
        });
        let query_name = match query_name { Some(name) => quote!(Some(#name)), None => quote!(None) };
        let ty = &field.ty.tokens;
        let opaque_field = field.attributes.iter().any(|attribute| attribute.name == HelperName::Opaque)
            || super::generics::is_boxed_dyn_any(&field.ty);
        let policy = if field.attributes.iter().any(|attribute| attribute.name == HelperName::Skip) {
            quote!(#facade::__private::codegen_v3::access::FieldAccessPolicy::Skipped, None, None, None)
        } else if field.attributes.iter().any(|attribute| attribute.name == HelperName::ReadOnly) {
            quote!(#facade::__private::codegen_v3::access::FieldAccessPolicy::ReadOnly, Some(<#self_type>::#get), None, None)
        } else {
            quote!(#facade::__private::codegen_v3::access::FieldAccessPolicy::ReadWrite, Some(<#self_type>::#get), Some(<#self_type>::#get_mut), Some(<#self_type>::#set))
        };
        let thread_safe_access = if !thread_safe {
            TokenStream::new()
        } else if field.attributes.iter().any(|attribute| attribute.name == HelperName::Skip) {
            quote!(.with_thread_safe_access(None, None, None))
        } else if field.attributes.iter().any(|attribute| attribute.name == HelperName::ReadOnly) {
            quote!(.with_thread_safe_access(Some(<#self_type>::#get_thread_safe), None, None))
        } else {
            quote!(.with_thread_safe_access(
                Some(<#self_type>::#get_thread_safe),
                Some(<#self_type>::#get_mut_thread_safe),
                Some(<#self_type>::#set_thread_safe),
            ))
        };
        let visibility = visibility(&field.visibility, &facade, field.span);
        let descriptor = if opaque_field {
            quote! {
                #facade::__private::codegen_v3::descriptor::field(
                    <#self_type as #facade::__private::codegen_v3::Reflect>::type_descriptor,
                    #index, #rust_name, #query_name,
                    ::std::boxed::Box::leak(::std::boxed::Box::new(
                        #facade::__private::codegen_v3::descriptor::TypeRef::Opaque(::std::boxed::Box::leak(
                            ::std::boxed::Box::new(#facade::__private::codegen_v3::descriptor::opaque_member::<#ty>()),
                        )),
                    )),
                    #visibility,
                )
            }
        } else {
            quote! {
                #facade::__private::codegen_v3::descriptor::lazy_field(
                    <#self_type as #facade::__private::codegen_v3::Reflect>::type_descriptor,
                    #index, #rust_name, #query_name,
                    #facade::__private::codegen_v3::descriptor::lazy_type_ref::<#ty>(),
                    #visibility,
                )
            }
        };
        quote! {
            #(#cfg_attributes)*
            #descriptor.with_access(#policy) #thread_safe_access
        }
        }).collect()
    };
    let struct_kind = kind_tokens(&declaration, &facade);
    let root_descriptor = if opaque_root {
        quote!(#facade::__private::codegen_v3::descriptor::with_capabilities(
            #facade::__private::codegen_v3::descriptor::opaque_root::<Self>(#query_name),
            #capability_resolver,
        ))
    } else if super::generics::has_skipped_field(&declaration, None) {
        quote! {
            let fields = ::std::boxed::Box::leak(::std::vec![#(#fields),*].into_boxed_slice());
            #facade::__private::codegen_v3::descriptor::with_capabilities(
                #facade::__private::codegen_v3::descriptor::struct_type::<Self>(
                    #query_name, #struct_kind, fields,
                ),
                #capability_resolver,
            )
        }
    } else {
        quote! {
            let fields = ::std::boxed::Box::leak(::std::vec![#(#fields),*].into_boxed_slice());
            #facade::__private::codegen_v3::descriptor::with_capabilities(
                #facade::__private::codegen_v3::descriptor::struct_type_with_construction::<Self>(
                    #query_name, #struct_kind, fields, #construction_descriptor,
                ),
                #capability_resolver,
            )
        }
    };
    let descriptor = if declaration.generics.params.is_empty() {
        root_descriptor
    } else {
        let generic = super::generics::concrete_descriptor(&declaration, &facade);
        let definition = super::generics::type_definition_provider_name(&declaration);
        quote!(#facade::__private::codegen_v3::descriptor::with_type_definition(
            #facade::__private::codegen_v3::descriptor::with_concrete_generic(
                { #root_descriptor },
                ::std::boxed::Box::leak(::std::boxed::Box::new(#generic)),
            ),
            #definition,
        ))
    };
    let registration = registration(
        &facade,
        &name,
        &registration_module,
        fingerprint,
        !declaration.generics.params.is_empty(),
    );
    let generic_definition_provider = super::generics::definition_provider(&declaration, &facade);
    let type_definition_provider =
        super::generics::type_definition_provider(&declaration, &facade, fingerprint);
    Ok(quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #capability_definition
            #(#adapter_definitions)*
            #construction_adapters
        }

        impl #impl_generics #facade::__private::codegen_v3::Reflect for #name #type_generics #where_clause {
            fn type_descriptor() -> &'static #facade::__private::codegen_v3::TypeDescriptor {
                #facade::__private::codegen_v3::descriptor::intern_type::<Self>(|| { #descriptor })
            }
        }

        #registration
        #generic_definition_provider
        #type_definition_provider
    })
}

/// Expands the static capability set requested on one derived type.
///
/// # Parameters
///
/// - `declaration`: Validated declaration containing capability attributes.
/// - `facade`: Runtime facade path used by generated references.
/// - `function`: Generated capability resolver identifier.
///
/// # Returns
///
/// Returns the generated capability resolver item.
#[must_use]
pub(crate) fn capabilities(
    declaration: &TypeDeclarationIr,
    facade: &TokenStream,
    function: &syn::Ident,
) -> TokenStream {
    let descriptors = declaration.attributes.iter().flat_map(|attribute| {
        (attribute.name == HelperName::Capabilities)
            .then_some(&attribute.value)
            .and_then(|value| match value {
                crate::ir::HelperValueIr::Paths(paths) => Some(paths),
                _ => None,
            })
            .into_iter()
            .flatten()
            .map(|path| match builtin_capability_name(path) {
                Some("Clone") => {
                    quote!(#facade::__private::codegen_v3::capability::clone_descriptor::<Self>())
                }
                Some("Default") => {
                    quote!(#facade::__private::codegen_v3::capability::default_descriptor::<Self>())
                }
                Some("Send") => {
                    quote!(#facade::__private::codegen_v3::capability::send_descriptor::<Self>())
                }
                Some("Sync") => {
                    quote!(#facade::__private::codegen_v3::capability::sync_descriptor::<Self>())
                }
                Some(_) | None => {
                    let tokens = &path.tokens;
                    quote!(#tokens::<Self>())
                }
            })
    });
    quote! {
        fn #function() -> #facade::__private::codegen_v3::capability::TypeCapabilitiesResult {
            #facade::__private::codegen_v3::capability::intern_capabilities::<Self>(|| {
                #facade::__private::codegen_v3::capability::TypeCapabilities::try_new(
                    ::std::vec![#(#descriptors),*],
                )
            })
        }
    }
}

/// Returns the built-in capability name for a bare, unqualified path.
///
/// # Parameters
///
/// - `path`: Normalized path from a capability attribute.
///
/// # Returns
///
/// Returns the built-in name for a supported bare path, or `None` for custom
/// and qualified provider paths.
#[inline]
fn builtin_capability_name(path: &crate::ir::PathIr) -> Option<&'static str> {
    if path.leading_colon || path.qualified_self.is_some() || path.segments.len() != 1 {
        return None;
    }
    let segment = &path.segments[0];
    if !matches!(&segment.arguments, PathArgumentsIr::None) {
        return None;
    }
    match segment.name.as_str() {
        "Clone" => Some("Clone"),
        "Default" => Some("Default"),
        "Send" => Some("Send"),
        "Sync" => Some("Sync"),
        _ => None,
    }
}

/// Generates a static fragment for concrete derived roots.
///
/// # Parameters
///
/// - `facade`: Runtime facade path used by generated references.
/// - `name`: Reflected type identifier.
/// - `module`: Unique generated module identifier.
/// - `fingerprint`: Stable source fingerprint for the fragment identity.
/// - `has_generics`: Whether the declaration is generic.
///
/// # Returns
///
/// Returns an empty stream for generic roots, or the registration module.
#[must_use]
fn registration(
    facade: &TokenStream,
    name: &syn::Ident,
    module: &syn::Ident,
    fingerprint: u64,
    has_generics: bool,
) -> TokenStream {
    if has_generics {
        return TokenStream::new();
    }
    quote! {
        #[doc(hidden)]
        mod #module {
            use super::*;

            fn runtime_identity() -> #facade::__private::codegen_v3::registration::RuntimeIdentity {
                #facade::__private::codegen_v3::registration::RuntimeIdentity::Type(::std::any::TypeId::of::<#name>())
            }

            fn payload() -> #facade::__private::codegen_v3::registration::FragmentPayload {
                #facade::__private::codegen_v3::registration::FragmentPayload::Type(
                    <#name as #facade::__private::codegen_v3::Reflect>::type_descriptor(),
                )
            }

            #facade::__private::codegen_v3::inventory::submit! {
                #facade::__private::codegen_v3::registration::RegistrationFragment::new(
                    #facade::__private::codegen_v3::registration::FragmentKind::Type,
                    #facade::__private::codegen_v3::registration::StaticFragmentIdentity::new(
                        env!("CARGO_PKG_NAME"), module_path!(), line!(), column!(), "type", #fingerprint,
                    ),
                    runtime_identity,
                    payload,
                )
            }
        }
    }
}

/// Expands a normalized source visibility into its public runtime form.
///
/// # Parameters
///
/// - `visibility`: Normalized source visibility.
/// - `facade`: Runtime facade path used by generated references.
/// - `span`: Source span for generated string literals.
///
/// # Returns
///
/// Returns the corresponding runtime visibility expression.
#[must_use]
#[inline]
pub(crate) fn visibility(
    visibility: &VisibilityIr,
    facade: &TokenStream,
    span: Span,
) -> TokenStream {
    match visibility {
        VisibilityIr::Public => {
            quote!(#facade::__private::codegen_v3::identity::Visibility::Public)
        }
        VisibilityIr::Crate => {
            quote!(#facade::__private::codegen_v3::identity::Visibility::Crate)
        }
        VisibilityIr::Super => {
            quote!(#facade::__private::codegen_v3::identity::Visibility::Super)
        }
        VisibilityIr::SelfValue | VisibilityIr::Inherited => {
            quote!(#facade::__private::codegen_v3::identity::Visibility::Private)
        }
        VisibilityIr::Restricted(path) => {
            let path = syn::LitStr::new(&path.source, span);
            quote!(#facade::__private::codegen_v3::identity::Visibility::Restricted(#path.into()))
        }
    }
}

/// Emits the runtime struct category shared by concrete and generic
/// descriptors.
///
/// # Parameters
///
/// - `declaration`: Struct declaration containing its normalized field shape.
/// - `facade`: Runtime facade path used by generated references.
///
/// # Returns
///
/// Returns tokens naming the matching runtime struct category.
#[must_use]
#[inline]
pub(crate) fn kind_tokens(declaration: &TypeDeclarationIr, facade: &TokenStream) -> TokenStream {
    match declaration.field_shape {
        FieldShapeIr::Unit => {
            quote!(#facade::__private::codegen_v3::descriptor::StructKind::Unit)
        }
        FieldShapeIr::Named => {
            quote!(#facade::__private::codegen_v3::descriptor::StructKind::Named)
        }
        FieldShapeIr::Unnamed if declaration.fields.len() == 1 => {
            quote!(#facade::__private::codegen_v3::descriptor::StructKind::Newtype)
        }
        FieldShapeIr::Unnamed => {
            quote!(#facade::__private::codegen_v3::descriptor::StructKind::Tuple)
        }
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;

    use super::expand;
    use crate::expand::ExpansionContext;
    use crate::ir::DeclarationIr;
    use crate::ir::MacroKind;
    use crate::parse::parse_and_validate_declaration;

    #[test]
    fn test_struct_emission_uses_codegen_v4_only() {
        let parsed = parse_and_validate_declaration(
            MacroKind::Derive,
            TokenStream::new(),
            quote! {
                struct Fixture<T> {
                    value: T,
                    unreflected: Unreflected,
                    #[reflect(skip)]
                    skipped: Hidden,
                }
            },
        )
        .expect("the structural fixture should parse");
        let DeclarationIr::Type(declaration) = parsed.declaration else {
            panic!("the structural fixture should produce a type declaration");
        };
        let generated = expand(declaration, &ExpansionContext::for_tests())
            .expect("the structural fixture should expand")
            .to_string();

        assert!(
            generated.contains("codegen_v4"),
            "struct tokens must use the v4 protocol"
        );
        assert!(
            !generated.contains("codegen_v3"),
            "struct tokens must not use the v3 protocol"
        );
        assert!(
            generated.contains("Unreflected"),
            "unreflected fields retain their type expression"
        );
        assert!(
            generated.contains("ExpressionData :: Named"),
            "unreflected fields retain TypeExpression metadata"
        );
        assert!(generated.contains("with_local_dynamic"));
        assert!(!generated.contains("with_thread_safe_dynamic"));
        assert!(
            !generated.contains("\"skipped\""),
            "skipped fields produce no metadata"
        );
        for forbidden in [
            "TypeId",
            "TypeIdentity",
            "FragmentIdentity",
            "RuntimeIdentity",
            "inventory",
        ] {
            assert!(
                !generated.contains(forbidden),
                "generated tokens must not mention {forbidden}"
            );
        }
    }

    #[test]
    fn test_struct_construction_emits_typed_providers_for_named_tuple_and_unit() {
        for (source, has_fields) in [
            (
                quote!(
                    struct Named {
                        value: u8,
                    }
                ),
                true,
            ),
            (
                quote!(
                    struct Tuple(u8);
                ),
                true,
            ),
            (
                quote!(
                    struct Unit;
                ),
                false,
            ),
        ] {
            let parsed =
                parse_and_validate_declaration(MacroKind::Derive, TokenStream::new(), source)
                    .expect("the struct fixture should parse");
            let DeclarationIr::Type(declaration) = parsed.declaration else {
                panic!("the fixture should be a struct");
            };
            let generated = expand(declaration, &ExpansionContext::for_tests())
                .expect("the fixture should expand")
                .to_string();
            assert!(generated.contains("ConstructProvider"));
            assert!(generated.contains("ConstructionAvailability :: Available"));
            assert_eq!(generated.contains("with_thread_safe_dynamic"), has_fields);
            assert!(!generated.contains("codegen_v3"));
        }
    }

    #[test]
    fn test_cfg_gated_fields_keep_metadata_aligned_without_construction_provider() {
        let parsed = parse_and_validate_declaration(
            MacroKind::Derive,
            TokenStream::new(),
            quote! {
                struct CfgFixture {
                    #[cfg(any())]
                    conditional: u8,
                    stable: u16,
                }
            },
        )
        .expect("the cfg fixture should parse");
        let DeclarationIr::Type(declaration) = parsed.declaration else {
            panic!("the cfg fixture should be a struct");
        };
        let generated = expand(declaration, &ExpansionContext::for_tests())
            .expect("the cfg fixture should expand")
            .to_string();

        assert!(generated.contains("cfg (any ())"));
        assert!(generated.contains("ConstructionAvailability :: Unavailable"));
        assert!(!generated.contains("ConstructProvider"));
        assert!(!generated.contains("codegen_v3"));
    }
}
