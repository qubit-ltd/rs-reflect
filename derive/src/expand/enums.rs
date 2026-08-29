//! Expansion of non-generic reflected enum declarations.

use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;

use crate::ir::GenericKindIr;
use crate::ir::HelperName;
use crate::ir::TypeDeclarationIr;
use crate::ir::TypeDeclarationKindIr;
use crate::ir::VariantKindIr;

/// Expands an enum root, its variants, and safe active-variant field adapters.
pub(crate) fn expand(declaration: TypeDeclarationIr) -> TokenStream {
    if declaration.kind != TypeDeclarationKindIr::Enum {
        return TokenStream::new();
    }
    let Some(facade) = super::facade_path_for(&declaration.attributes) else {
        return TokenStream::new();
    };
    let name = declaration.name;
    let reflected_parameters: Vec<_> = declaration.generics.params.iter()
        .filter(|parameter| parameter.kind == GenericKindIr::Type)
        .filter(|parameter| declaration.variants.iter().any(|variant| variant.fields.iter().any(|field|
            !field.attributes.iter().any(|attribute| attribute.name == HelperName::Opaque)
                && super::structs::type_uses_identifier(&field.ty.tokens, &parameter.name))))
        .map(|parameter| syn::Ident::new(&parameter.name, parameter.span))
        .collect();
    let type_parameters: Vec<_> = declaration.generics.params.iter()
        .filter(|parameter| parameter.kind == GenericKindIr::Type)
        .map(|parameter| syn::Ident::new(&parameter.name, parameter.span))
        .collect();
    let mut generics: syn::Generics = match syn::parse2(declaration.generics.declaration.clone()) {
        Ok(generics) => generics,
        Err(_) => return TokenStream::new(),
    };
    {
        let where_clause = generics.make_where_clause();
        for parameter in &type_parameters {
            where_clause.predicates.push(syn::parse_quote!(#parameter: 'static));
        }
        for parameter in &reflected_parameters {
            where_clause.predicates.push(syn::parse_quote!(#parameter: #facade::Reflect));
        }
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let fingerprint = fingerprint(&declaration.retained_tokens.to_string());
    let registration_module = format_ident!("__qubit_reflect_enum_registration_{fingerprint:016x}");
    let query_name = declaration
        .attributes
        .iter()
        .find_map(|attribute| attribute.rename())
        .unwrap_or(&name.to_string())
        .to_owned();
    let integer_repr = declaration
        .variants
        .iter()
        .all(|variant| variant.kind == VariantKindIr::Unit)
        .then(|| declaration.generics.params.is_empty().then(|| integer_repr(&declaration.retained_tokens)))
        .flatten()
        .flatten();
    if declaration
        .attributes
        .iter()
        .any(|attribute| attribute.name == HelperName::Opaque)
    {
        let registration = registration(&facade, &name, &registration_module, fingerprint, !declaration.generics.params.is_empty());
        return quote! {
            impl #impl_generics #facade::Reflect for #name #type_generics #where_clause {
                fn type_descriptor() -> &'static #facade::TypeDescriptor {
                    #facade::__private::intern_type::<Self>(|| {
                        #facade::__private::descriptor::opaque_root::<Self>(#query_name)
                    })
                }
            }
            #registration
        };
    }
    let adapters = declaration
        .variants
        .iter()
        .flat_map(|variant| adapters(&name, variant, &facade));
    let variants = declaration
        .variants
        .iter()
        .map(|variant| variant_descriptor(&name, &quote!(#name #type_generics), variant, &facade, integer_repr.as_deref()));
    let registration = registration(&facade, &name, &registration_module, fingerprint, !declaration.generics.params.is_empty());
    quote! {
        impl #impl_generics #name #type_generics #where_clause { #(#adapters)* }

        impl #impl_generics #facade::Reflect for #name #type_generics #where_clause {
            fn type_descriptor() -> &'static #facade::TypeDescriptor {
                #facade::__private::intern_type::<Self>(|| {
                    let variants = ::std::boxed::Box::leak(::std::vec![#(#variants),*].into_boxed_slice());
                    #facade::__private::descriptor::enum_type::<Self>(#query_name, variants)
                })
            }
        }

        #registration
    }
}

/// Emits the static registry fragment for one concrete derived enum root.
fn registration(facade: &TokenStream, name: &syn::Ident, module: &syn::Ident, fingerprint: u64, has_generics: bool) -> TokenStream {
    if has_generics {
        return TokenStream::new();
    }
    quote! {
        #[doc(hidden)]
        mod #module {
            use super::*;
            use #facade as __qubit_reflect;

            fn runtime_identity() -> __qubit_reflect::__private::RuntimeIdentity {
                __qubit_reflect::__private::RuntimeIdentity::Type(::std::any::TypeId::of::<#name>())
            }

            fn payload() -> __qubit_reflect::__private::FragmentPayload {
                __qubit_reflect::__private::FragmentPayload::Type(
                    <#name as __qubit_reflect::Reflect>::type_descriptor(),
                )
            }

            __qubit_reflect::__private::inventory::submit! {
                __qubit_reflect::__private::RegistrationFragment::new(
                    __qubit_reflect::__private::FragmentKind::Type,
                    __qubit_reflect::__private::StaticFragmentIdentity::new(
                        env!("CARGO_PKG_NAME"), module_path!(), line!(), column!(), "type", #fingerprint,
                    ),
                    runtime_identity,
                    payload,
                )
            }
        }
    }
}

/// Computes a stable FNV-1a content fingerprint for one declaration.
fn fingerprint(input: &str) -> u64 {
    input.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

fn adapters(_name: &syn::Ident, variant: &crate::ir::VariantIr, facade: &TokenStream) -> Vec<TokenStream> {
    let variant_name = &variant.name;
    let variant_index = variant.index;
    let variant_name_text = variant_name.to_string();
    let active = format_ident!("__qubit_reflect_is_variant_{variant_index}");
    let active_pattern = match variant.kind {
        VariantKindIr::Unit => quote!(Self::#variant_name),
        VariantKindIr::Tuple => quote!(Self::#variant_name(..)),
        VariantKindIr::Struct => quote!(Self::#variant_name { .. }),
    };
    let mut definitions = vec![quote! {
        fn #active(value: #facade::value::ReflectedRef<'_>)
            -> ::core::result::Result<bool, #facade::error::TypeMismatch>
        {
            let value = value.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
            Ok(matches!(value, #active_pattern))
        }
    }];
    definitions.extend(variant
        .fields
        .iter()
        .flat_map(|field| {
            let index = field.index;
            let get = format_ident!("__qubit_reflect_get_variant_{variant_index}_field_{index}");
            let get_mut = format_ident!("__qubit_reflect_get_mut_variant_{variant_index}_field_{index}");
            let set = format_ident!("__qubit_reflect_set_variant_{variant_index}_field_{index}");
            let ty = &field.ty.tokens;
            let binding = format_ident!("__qubit_reflect_value_{index}");
            let rust_name = field.name.as_ref().map(|value| value.to_string());
            let rust_name = match rust_name {
                Some(value) => quote!(Some(#value)),
                None => quote!(None),
            };
            let pattern = match variant.kind {
                VariantKindIr::Struct => {
                    let field_name = field.name.as_ref().expect("validated struct variant field");
                    quote!(Self::#variant_name { #field_name: #binding, .. })
                }
                VariantKindIr::Tuple => {
                    let bindings = (0..variant.fields.len())
                        .map(|position| if position == index { quote!(#binding) } else { quote!(_) });
                    quote!(Self::#variant_name(#(#bindings),*))
                }
                VariantKindIr::Unit => return Vec::new(),
            };
            let inactive = quote!(#facade::access::FieldAccessError::inactive_variant(
                #facade::access::FieldIdentity::new_variant(
                    ::std::any::TypeId::of::<Self>(), ::std::any::type_name::<Self>(), #index, #rust_name,
                    #variant_index, #variant_name_text,
                ), #variant_index, #variant_name_text,
            ));
            vec![quote! {
                fn #get<'a>(target: #facade::value::ReflectedRef<'a>)
                    -> ::core::result::Result<#facade::value::ReflectedRef<'a>, #facade::access::FieldAccessError>
                {
                    let value = target.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
                    match value { #pattern => Ok(#facade::value::ReflectedRef::new(#binding)), _ => Err(#inactive) }
                }
                fn #get_mut<'a>(target: #facade::value::ReflectedMut<'a>)
                    -> ::core::result::Result<#facade::value::ReflectedMut<'a>, #facade::access::FieldAccessError>
                {
                    let value = target.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
                    match value { #pattern => Ok(#facade::value::ReflectedMut::new(#binding)), _ => Err(#inactive) }
                }
                fn #set(target: #facade::value::ReflectedMut<'_>, replacement: #facade::value::ReflectedOwned)
                    -> ::core::result::Result<(), #facade::access::FieldAccessError>
                {
                    let value = target.downcast::<Self>().unwrap_or_else(|_| unreachable!("validated enum target"));
                    let replacement = #facade::value::ReflectedOwned::downcast::<#ty>(replacement)
                        .unwrap_or_else(|_| unreachable!("validated enum field value"));
                    match value { #pattern => { *#binding = replacement; Ok(()) }, _ => Err(#inactive) }
                }
            }]
        })
        .collect::<Vec<_>>());
    definitions
}

fn variant_descriptor(
    name: &syn::Ident,
    self_type: &TokenStream,
    variant: &crate::ir::VariantIr,
    facade: &TokenStream,
    integer_repr: Option<&str>,
) -> TokenStream {
    let variant_name = &variant.name;
    let variant_index = variant.index;
    let variant_rust_name = variant_name.to_string();
    let query_name = variant
        .attributes
        .iter()
        .find_map(|attribute| attribute.rename())
        .unwrap_or(&variant_rust_name)
        .to_owned();
    let kind = match variant.kind {
        VariantKindIr::Unit => quote!(#facade::descriptor::VariantKind::Unit),
        VariantKindIr::Tuple => quote!(#facade::descriptor::VariantKind::Tuple),
        VariantKindIr::Struct => quote!(#facade::descriptor::VariantKind::Struct),
    };
    let origin = if variant.discriminant.is_some() {
        quote!(#facade::descriptor::DiscriminantOrigin::Explicit)
    } else {
        quote!(#facade::descriptor::DiscriminantOrigin::Implicit)
    };
    let numeric_discriminant = if variant.kind == VariantKindIr::Unit {
        numeric_discriminant(name, variant_name, integer_repr, facade)
    } else {
        quote!(None)
    };
    let active = format_ident!("__qubit_reflect_is_variant_{variant_index}");
    let fields = variant.fields.iter().map(|field| {
        let index = field.index;
        let get = format_ident!("__qubit_reflect_get_variant_{variant_index}_field_{index}");
        let get_mut = format_ident!("__qubit_reflect_get_mut_variant_{variant_index}_field_{index}");
        let set = format_ident!("__qubit_reflect_set_variant_{variant_index}_field_{index}");
        let field_rust_name = field.name.as_ref().map(|value| value.to_string());
        let field_rust_name = match field_rust_name { Some(value) => quote!(Some(#value)), None => quote!(None) };
        let query_name = field.name.as_ref().map(|field_name| field.attributes.iter().find_map(|attribute| attribute.rename()).unwrap_or(&field_name.to_string()).to_owned());
        let query_name = match query_name { Some(value) => quote!(Some(#value)), None => quote!(None) };
        let ty = &field.ty.tokens;
        let field_type = if field.attributes.iter().any(|attribute| attribute.name == HelperName::Opaque) {
            quote!(#facade::descriptor::TypeRef::Opaque(::std::boxed::Box::leak(::std::boxed::Box::new(#facade::__private::descriptor::opaque_member::<#ty>()))))
        } else {
            quote!(#facade::descriptor::TypeRef::Resolved(<#ty as #facade::Reflect>::type_descriptor()))
        };
        let policy = if field.attributes.iter().any(|attribute| attribute.name == HelperName::Skip) {
            quote!(#facade::access::FieldAccessPolicy::Skipped, None, None, None)
        } else if field.attributes.iter().any(|attribute| attribute.name == HelperName::ReadOnly) {
            quote!(#facade::access::FieldAccessPolicy::ReadOnly, Some(<#self_type>::#get), None, None)
        } else {
            quote!(#facade::access::FieldAccessPolicy::ReadWrite, Some(<#self_type>::#get), Some(<#self_type>::#get_mut), Some(<#self_type>::#set))
        };
        quote!(#facade::__private::descriptor::field(<#self_type as #facade::Reflect>::type_descriptor, #index, #field_rust_name, #query_name,
            ::std::boxed::Box::leak(::std::boxed::Box::new(#field_type)), #facade::identity::Visibility::Private)
            .with_access(#policy).with_variant(#variant_index, #variant_rust_name))
    });
    quote! {{
        let fields = ::std::boxed::Box::leak(::std::vec![#(#fields),*].into_boxed_slice());
        #facade::__private::descriptor::variant(<#self_type as #facade::Reflect>::type_descriptor, #variant_index, #variant_rust_name, #query_name, #kind, fields, <#self_type>::#active)
            .with_discriminant(#origin, #numeric_discriminant)
    }}
}

/// Extracts the sole integer representation relevant to discriminant queries.
fn integer_repr(tokens: &TokenStream) -> Option<String> {
    let input: syn::DeriveInput = syn::parse2(tokens.clone()).ok()?;
    input.attrs.iter().find_map(|attribute| {
        if !attribute.path().is_ident("repr") {
            return None;
        }
        let syn::Meta::List(list) = &attribute.meta else {
            return None;
        };
        let values = list
            .parse_args_with(syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated)
            .ok()?;
        values.into_iter().map(|value| value.to_string()).find(|value| {
            matches!(
                value.as_str(),
                "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128" | "usize"
            )
        })
    })
}

/// Emits an exact compiler-checked cast for a fieldless integer-repr variant.
fn numeric_discriminant(
    enum_name: &syn::Ident,
    variant_name: &syn::Ident,
    repr: Option<&str>,
    facade: &TokenStream,
) -> TokenStream {
    let Some(repr) = repr else {
        return quote!(None);
    };
    let variant = match repr {
        "i8" => quote!(I8),
        "i16" => quote!(I16),
        "i32" => quote!(I32),
        "i64" => quote!(I64),
        "i128" => quote!(I128),
        "isize" => quote!(Isize),
        "u8" => quote!(U8),
        "u16" => quote!(U16),
        "u32" => quote!(U32),
        "u64" => quote!(U64),
        "u128" => quote!(U128),
        "usize" => quote!(Usize),
        _ => return quote!(None),
    };
    let repr = syn::Ident::new(repr, variant_name.span());
    quote!(Some(#facade::descriptor::NumericDiscriminant::#variant(#enum_name::#variant_name as #repr)))
}
