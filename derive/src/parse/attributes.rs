//! Parsing for the shared `reflect` helper grammar.

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Attribute;
use syn::Expr;
use syn::ExprLit;
use syn::Lit;
use syn::LitStr;
use syn::Meta;
use syn::Path;
use syn::Token;
use syn::Type;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::ir::ExternalTraitIr;
use crate::ir::HelperAttributeIr;
use crate::ir::HelperName;
use crate::ir::HelperTarget;
use crate::ir::PathIr;
use crate::ir::SpecializationBindingIr;
use crate::ir::SpecializationIr;
use crate::ir::SpecializationValueIr;
use crate::parse::type_ir::convert_path;
use crate::parse::type_ir::convert_type;

/// Accumulates independent `syn` diagnostics without losing their spans.
#[derive(Default)]
pub(super) struct ErrorCollector {
    error: Option<syn::Error>,
}

impl ErrorCollector {
    /// Adds one diagnostic to the aggregate.
    pub(super) fn push(&mut self, error: syn::Error) {
        if let Some(combined) = &mut self.error {
            combined.combine(error);
        } else {
            self.error = Some(error);
        }
    }

    /// Returns the accumulated error for a later validation phase.
    pub(super) fn into_error(self) -> Option<syn::Error> {
        self.error
    }
}

/// Parses helper attributes from a source attribute slice.
pub(super) fn parse_attributes(
    attributes: &[Attribute],
    target: HelperTarget,
    errors: &mut ErrorCollector,
) -> Vec<HelperAttributeIr> {
    let mut helpers = Vec::new();
    for attribute in attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("reflect"))
    {
        match &attribute.meta {
            Meta::Path(_) => errors.push(syn::Error::new(
                attribute.span(),
                "bare `#[reflect]` is not a valid helper attribute",
            )),
            Meta::List(list) => {
                helpers.extend(parse_helper_tokens(list.tokens.clone(), target, errors));
            }
            Meta::NameValue(_) => errors.push(syn::Error::new(
                attribute.span(),
                "`reflect` helpers must use `#[reflect(...)]` syntax",
            )),
        }
    }
    helpers
}

/// Parses the comma-separated arguments of a macro or helper attribute.
pub(super) fn parse_helper_tokens(
    tokens: TokenStream,
    target: HelperTarget,
    errors: &mut ErrorCollector,
) -> Vec<HelperAttributeIr> {
    if tokens.is_empty() {
        return Vec::new();
    }
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let metas = match parser.parse2(tokens) {
        Ok(metas) => metas,
        Err(error) => {
            errors.push(error);
            return Vec::new();
        }
    };
    metas
        .into_iter()
        .filter_map(|meta| convert_meta(meta, target, errors))
        .collect()
}

/// Converts one `syn::Meta` node into helper IR.
fn convert_meta(meta: Meta, target: HelperTarget, errors: &mut ErrorCollector) -> Option<HelperAttributeIr> {
    let span = meta.span();
    let path_text = meta.path().to_token_stream().to_string();
    let Some(source_name) = meta.path().get_ident().map(ToString::to_string) else {
        errors.push(syn::Error::new(
            span,
            format!("unknown reflection helper `{path_text}`"),
        ));
        return None;
    };
    let Some(name) = HelperName::from_str(&source_name) else {
        errors.push(syn::Error::new(
            span,
            format!("unknown reflection helper `{source_name}`"),
        ));
        return None;
    };
    let value = match name {
        HelperName::Rename => parse_string_value(&meta, "rename").map(crate::ir::HelperValueIr::Rename),
        HelperName::Opaque
        | HelperName::Skip
        | HelperName::ReadOnly
        | HelperName::NoConstruct
        | HelperName::NoInvoke
        | HelperName::CatchUnwind
        | HelperName::ThreadSafe => parse_flag(&meta, name.as_str()).map(|()| crate::ir::HelperValueIr::Flag),
        HelperName::Capabilities => parse_path_list(&meta, "capabilities").map(crate::ir::HelperValueIr::Paths),
        HelperName::Supertrait => parse_path_list(&meta, "supertrait").map(crate::ir::HelperValueIr::Paths),
        HelperName::Default => parse_default(&meta).map(crate::ir::HelperValueIr::DefaultPath),
        HelperName::Specialize => parse_specialization(&meta).map(crate::ir::HelperValueIr::Specialization),
        HelperName::ExternalTraitId => {
            parse_string_value(&meta, "external_trait_id").map(crate::ir::HelperValueIr::ExternalTraitId)
        }
        HelperName::ExternalTrait => parse_external_trait(&meta).map(crate::ir::HelperValueIr::ExternalTrait),
        HelperName::RuntimeCrate => parse_runtime_crate(&meta).map(crate::ir::HelperValueIr::RuntimeCrate),
    };
    match value {
        Ok(value) => {
            let value_span = match &meta {
                Meta::NameValue(name_value) => name_value.value.span(),
                Meta::List(list) => list.span(),
                Meta::Path(path) => path.span(),
            };
            Some(HelperAttributeIr {
                name,
                value,
                target,
                span,
                value_span,
            })
        }
        Err(error) => {
            errors.push(error);
            None
        }
    }
}

/// Parses the explicit facade path used by a downstream macro re-export.
fn parse_runtime_crate(meta: &Meta) -> syn::Result<PathIr> {
    let Meta::NameValue(name_value) = meta else {
        return Err(syn::Error::new(meta.span(), "`crate` requires a Rust facade path"));
    };
    let Expr::Path(path) = &name_value.value else {
        return Err(syn::Error::new(
            name_value.value.span(),
            "`crate` requires a Rust facade path",
        ));
    };
    Ok(convert_path(&path.path))
}

/// Requires a bare flag helper without arguments or a value.
fn parse_flag(meta: &Meta, name: &str) -> syn::Result<()> {
    if matches!(meta, Meta::Path(_)) {
        Ok(())
    } else {
        Err(syn::Error::new(
            meta.span(),
            format!("`{name}` does not accept a value"),
        ))
    }
}

/// Parses a helper whose value must be a string literal.
fn parse_string_value(meta: &Meta, name: &str) -> syn::Result<String> {
    let Meta::NameValue(name_value) = meta else {
        return Err(syn::Error::new(
            meta.span(),
            format!("`{name}` requires a string literal value"),
        ));
    };
    let Expr::Lit(ExprLit {
        lit: Lit::Str(value), ..
    }) = &name_value.value
    else {
        return Err(syn::Error::new(
            name_value.value.span(),
            format!("`{name}` requires a string literal value"),
        ));
    };
    Ok(value.value())
}

/// Parses a parenthesized list of Rust paths.
fn parse_path_list(meta: &Meta, name: &str) -> syn::Result<Vec<PathIr>> {
    let Meta::List(list) = meta else {
        return Err(syn::Error::new(
            meta.span(),
            format!("`{name}` requires a parenthesized path list"),
        ));
    };
    let paths = list.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)?;
    if paths.is_empty() {
        return Err(syn::Error::new(
            list.span(),
            format!("`{name}` requires at least one path"),
        ));
    }
    Ok(paths.iter().map(convert_path).collect())
}

/// Parses `default` or `default = provider_path`.
fn parse_default(meta: &Meta) -> syn::Result<Option<PathIr>> {
    match meta {
        Meta::Path(_) => Ok(None),
        Meta::NameValue(name_value) => {
            let Expr::Path(path) = &name_value.value else {
                return Err(syn::Error::new(
                    name_value.value.span(),
                    "`default` provider must be a Rust path",
                ));
            };
            Ok(Some(convert_path(&path.path)))
        }
        Meta::List(_) => Err(syn::Error::new(
            meta.span(),
            "`default` accepts either no value or `= provider_path`",
        )),
    }
}

/// Parses a named concrete specialization.
fn parse_specialization(meta: &Meta) -> syn::Result<SpecializationIr> {
    let Meta::List(list) = meta else {
        return Err(syn::Error::new(meta.span(), "`specialize` requires named arguments"));
    };
    let bindings = syn::parse2::<SpecializationBindings>(list.tokens.clone())?;
    if bindings.0.is_empty() {
        return Err(syn::Error::new(
            list.span(),
            "`specialize` requires at least one named argument",
        ));
    }
    Ok(SpecializationIr {
        bindings: bindings.0,
        span: list.span(),
    })
}

/// Parses an external trait path and its stable ID.
fn parse_external_trait(meta: &Meta) -> syn::Result<ExternalTraitIr> {
    let Meta::List(list) = meta else {
        return Err(syn::Error::new(
            meta.span(),
            "`external_trait` requires `(path, id = \"...\")`",
        ));
    };
    let parsed = syn::parse2::<ExternalTraitSyntax>(list.tokens.clone())?;
    Ok(ExternalTraitIr {
        path: convert_path(&parsed.path),
        id: parsed.id.value(),
        id_span: parsed.id.span(),
        span: list.span(),
    })
}

/// Parser for the named specialization grammar.
struct SpecializationBindings(Vec<SpecializationBindingIr>);

impl Parse for SpecializationBindings {
    /// Parses comma-separated `Name = tokens` bindings while retaining RHS
    /// syntax.
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut bindings = Vec::new();
        while !input.is_empty() {
            let name: syn::Ident = input.parse()?;
            let span = name.span();
            input.parse::<Token![=]>()?;
            let type_fork = input.fork();
            let (value, value_span) =
                if type_fork.parse::<Type>().is_ok() && (type_fork.is_empty() || type_fork.peek(Token![,])) {
                    let ty = input.parse::<Type>()?;
                    let span = ty.span();
                    let value = if matches!(
                        &ty,
                        Type::Path(path)
                            if path.qself.is_none()
                                && path.path.segments.iter().all(|segment| {
                                    matches!(segment.arguments, syn::PathArguments::None)
                                })
                    ) {
                        SpecializationValueIr::AmbiguousPath(ty.to_token_stream())
                    } else {
                        SpecializationValueIr::Type(convert_type(&ty))
                    };
                    (value, span)
                } else {
                    let expression = input.parse::<Expr>()?;
                    let span = expression.span();
                    (SpecializationValueIr::Const(expression.to_token_stream()), span)
                };
            bindings.push(SpecializationBindingIr {
                name: name.to_string(),
                value,
                span,
                value_span,
            });
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(Self(bindings))
    }
}

/// Parser for `external_trait(path, id = "...")`.
struct ExternalTraitSyntax {
    path: Path,
    id: LitStr,
}

impl Parse for ExternalTraitSyntax {
    /// Parses the external path followed by the required `id` literal.
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let path = input.parse()?;
        input.parse::<Token![,]>()?;
        let key: syn::Ident = input.parse()?;
        if key != "id" {
            return Err(syn::Error::new(key.span(), "expected `id`"));
        }
        input.parse::<Token![=]>()?;
        let id = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after external trait ID"));
        }
        Ok(Self { path, id })
    }
}

/// Removes reflection helper attributes before an attribute macro returns its
/// item.
pub(super) fn remove_reflect_attributes(attributes: &mut Vec<Attribute>) {
    attributes.retain(|attribute| !attribute.path().is_ident("reflect"));
}
