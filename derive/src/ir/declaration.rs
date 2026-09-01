// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Declaration-level semantic IR shared by validation and expansion.

use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;

use crate::ir::ExternalTraitIr;
use crate::ir::HelperAttributeIr;
use crate::ir::PathIr;
use crate::ir::SpecializationIr;
use crate::ir::TypeIr;

/// Selects one of the three reflection procedural macro entry points.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MacroKind {
    Derive,
    Trait,
    Impl,
}

/// A declaration after parsing but before semantic validation.
#[derive(Clone, Debug)]
pub(crate) struct ParsedDeclaration {
    /// The declaration produced by the parser boundary.
    pub(crate) declaration: DeclarationIr,
}

/// A declaration whose locally provable macro invariants have been checked.
#[derive(Clone, Debug)]
pub(crate) struct ValidatedDeclaration {
    /// The declaration whose validation invariants now hold.
    pub(crate) declaration: DeclarationIr,
}

/// One of the declarations supported by the reflection macros.
#[derive(Clone, Debug)]
pub(crate) enum DeclarationIr {
    Type(TypeDeclarationIr),
    Trait(TraitDeclarationIr),
    Impl(ImplDeclarationIr),
}

/// A struct, enum, or rejected union derive input.
#[derive(Clone, Debug)]
pub(crate) struct TypeDeclarationIr {
    /// The source type identifier.
    pub(crate) name: Ident,
    /// The Rust data-declaration form.
    pub(crate) kind: TypeDeclarationKindIr,
    /// The normalized source visibility.
    pub(crate) visibility: VisibilityIr,
    /// Generic parameters and where predicates.
    pub(crate) generics: GenericsIr,
    /// Type-level reflection helpers.
    pub(crate) attributes: Vec<HelperAttributeIr>,
    /// Direct fields for struct or union declarations.
    pub(crate) fields: Vec<FieldIr>,
    /// Direct variants for enum declarations.
    pub(crate) variants: Vec<VariantIr>,
    /// Original declaration tokens retained for later expansion.
    pub(crate) retained_tokens: TokenStream,
    /// The declaration-name span used for diagnostics.
    pub(crate) span: Span,
}

/// Distinguishes the three data declaration forms accepted by `syn`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TypeDeclarationKindIr {
    Struct,
    Enum,
    Union,
}

/// A field in a struct or enum variant.
#[derive(Clone, Debug)]
pub(crate) struct FieldIr {
    /// The source field name, or `None` for tuple fields.
    pub(crate) name: Option<Ident>,
    /// The zero-based source index within the direct field scope.
    pub(crate) index: usize,
    /// The normalized source visibility.
    pub(crate) visibility: VisibilityIr,
    /// The field type converted at the parser boundary.
    pub(crate) ty: TypeIr,
    /// Field-level reflection helpers.
    pub(crate) attributes: Vec<HelperAttributeIr>,
    /// The complete field source span.
    pub(crate) span: Span,
}

/// An enum variant and its direct fields.
#[derive(Clone, Debug)]
pub(crate) struct VariantIr {
    /// The source variant identifier.
    pub(crate) name: Ident,
    /// The zero-based source index within the enum.
    pub(crate) index: usize,
    /// The source payload shape.
    pub(crate) kind: VariantKindIr,
    /// Variant fields in source order.
    pub(crate) fields: Vec<FieldIr>,
    /// Variant-level reflection helpers.
    pub(crate) attributes: Vec<HelperAttributeIr>,
    /// An explicit discriminant expression, when present.
    pub(crate) discriminant: Option<TokenStream>,
    /// The variant-name span used for diagnostics.
    pub(crate) span: Span,
}

/// The source shape of an enum variant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VariantKindIr {
    Unit,
    Tuple,
    Struct,
}

/// A reflected trait declaration.
#[derive(Clone, Debug)]
pub(crate) struct TraitDeclarationIr {
    /// The source trait identifier.
    pub(crate) name: Ident,
    /// The normalized source visibility.
    pub(crate) visibility: VisibilityIr,
    /// Generic parameters and where predicates.
    pub(crate) generics: GenericsIr,
    /// Direct supertrait bounds in source order.
    pub(crate) supertraits: Vec<GenericBoundIr>,
    /// Helpers supplied to the outer trait macro.
    pub(crate) attributes: Vec<HelperAttributeIr>,
    /// Explicit mappings for external trait bounds.
    pub(crate) external_traits: Vec<ExternalTraitIr>,
    /// Explicitly declared reflected direct supertrait paths.
    pub(crate) reflected_supertraits: Vec<PathIr>,
    /// Trait methods in source order.
    pub(crate) methods: Vec<MethodIr>,
    /// Associated type declarations in source order.
    pub(crate) associated_types: Vec<AssociatedTypeIr>,
    /// Associated const declarations in source order.
    pub(crate) associated_consts: Vec<AssociatedConstIr>,
    /// Trait tokens with nested reflection helpers removed.
    pub(crate) retained_tokens: TokenStream,
    /// The trait-name span used for diagnostics.
    pub(crate) span: Span,
}

/// A reflected inherent or trait impl declaration.
#[derive(Clone, Debug)]
pub(crate) struct ImplDeclarationIr {
    /// Generic parameters and where predicates.
    pub(crate) generics: GenericsIr,
    /// The impl self type converted at the parser boundary.
    pub(crate) target_type: TypeIr,
    /// The implemented trait path, or `None` for an inherent impl.
    pub(crate) trait_path: Option<PathIr>,
    /// Helpers supplied to the outer impl macro.
    pub(crate) attributes: Vec<HelperAttributeIr>,
    /// Concrete impl specializations in source order.
    pub(crate) specializations: Vec<SpecializationIr>,
    /// Impl methods in source order.
    pub(crate) methods: Vec<MethodIr>,
    /// Associated type bindings in source order.
    pub(crate) associated_types: Vec<AssociatedTypeIr>,
    /// Associated const bindings in source order.
    pub(crate) associated_consts: Vec<AssociatedConstIr>,
    /// Impl tokens with nested reflection helpers removed.
    pub(crate) retained_tokens: TokenStream,
    /// The `impl` keyword span used for diagnostics.
    pub(crate) span: Span,
}

/// A method signature and its reflection policies.
#[derive(Clone, Debug)]
pub(crate) struct MethodIr {
    /// The source method identifier.
    pub(crate) name: Ident,
    /// The normalized source visibility.
    pub(crate) visibility: VisibilityIr,
    /// Method generic parameters and where predicates.
    pub(crate) generics: GenericsIr,
    /// Receiver syntax when the method has a receiver.
    pub(crate) receiver: Option<ReceiverIr>,
    /// Non-receiver parameter name, type, and pattern facts.
    pub(crate) parameters: Vec<ParameterIr>,
    /// The semantic method return category.
    pub(crate) return_type: ReturnTypeIr,
    /// Const, async, unsafe, and ABI qualifier tokens.
    pub(crate) qualifiers: MethodQualifiersIr,
    /// Whether a trait method supplies a default body.
    pub(crate) has_default: bool,
    /// Method-level reflection helpers.
    pub(crate) attributes: Vec<HelperAttributeIr>,
    /// Concrete method specializations in source order.
    pub(crate) specializations: Vec<SpecializationIr>,
    /// The method-name span used for diagnostics.
    pub(crate) span: Span,
}

/// A method receiver classified without retaining a `syn::Receiver`.
#[derive(Clone, Debug)]
pub(crate) struct ReceiverIr {
    pub(crate) kind: ReceiverKindIr,
    pub(crate) ty: TypeIr,
    pub(crate) declaration: TokenStream,
    pub(crate) span: Span,
}

/// The core receiver forms relevant to safe adapter selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReceiverKindIr {
    Value,
    SharedReference,
    MutableReference,
    Typed,
}

/// One non-receiver method parameter.
#[derive(Clone, Debug)]
pub(crate) struct ParameterIr {
    pub(crate) name: Option<String>,
    pub(crate) pattern: ParameterPatternIr,
    pub(crate) ty: TypeIr,
    pub(crate) index: usize,
    pub(crate) span: Span,
}

/// A method parameter pattern represented independently of `syn::Pat`.
#[derive(Clone, Debug)]
pub(crate) struct ParameterPatternIr {
    pub(crate) kind: ParameterPatternKindIr,
    pub(crate) source: String,
    pub(crate) tokens: TokenStream,
}

/// Parameter pattern categories that affect named invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ParameterPatternKindIr {
    Identifier,
    Wildcard,
    Destructure,
}

/// Method qualifiers that affect descriptor facts and adapter safety.
#[derive(Clone, Debug, Default)]
pub(crate) struct MethodQualifiersIr {
    pub(crate) is_const: bool,
    pub(crate) is_async: bool,
    pub(crate) is_unsafe: bool,
    pub(crate) abi: Option<String>,
    pub(crate) is_variadic: bool,
}

/// The semantic return category of a method.
#[derive(Clone, Debug)]
pub(crate) enum ReturnTypeIr {
    Unit,
    Type(TypeIr),
}

/// A trait or impl associated type declaration/binding.
#[derive(Clone, Debug)]
pub(crate) struct AssociatedTypeIr {
    /// The associated type identifier.
    pub(crate) name: Ident,
    /// Generic parameters and where predicates declared by the associated
    /// type.
    pub(crate) generics: GenericsIr,
    /// Direct bounds declared after the associated type name.
    pub(crate) bounds: Vec<GenericBoundIr>,
    /// A default or impl binding converted to type IR.
    pub(crate) value: Option<TypeIr>,
    /// The complete associated type declaration tokens.
    pub(crate) declaration: TokenStream,
    /// Reflection helpers found on the associated item.
    pub(crate) attributes: Vec<HelperAttributeIr>,
    /// The associated type name span.
    pub(crate) span: Span,
}

/// A trait or impl associated const declaration/binding.
#[derive(Clone, Debug)]
pub(crate) struct AssociatedConstIr {
    /// The associated const identifier.
    pub(crate) name: Ident,
    /// The declared associated const type.
    pub(crate) ty: TypeIr,
    /// A default or impl value expression when present.
    pub(crate) value: Option<TokenStream>,
    /// The complete associated const declaration tokens.
    pub(crate) declaration: TokenStream,
    /// Reflection helpers found on the associated item.
    pub(crate) attributes: Vec<HelperAttributeIr>,
    /// The associated const name span.
    pub(crate) span: Span,
}

/// A source generic parameter list and its predicates.
#[derive(Clone, Debug, Default)]
pub(crate) struct GenericsIr {
    /// Generic parameters in declaration order.
    pub(crate) params: Vec<GenericParamIr>,
    /// Where predicates in declaration order.
    pub(crate) where_predicates: Vec<WherePredicateIr>,
    /// The complete generic declaration tokens.
    pub(crate) declaration: TokenStream,
    /// Generic parameter tokens suitable for an impl header, with defaults
    /// removed.
    pub(crate) impl_declaration: TokenStream,
    /// Generic arguments in declaration order.
    pub(crate) arguments: TokenStream,
    /// The complete where clause tokens.
    pub(crate) where_clause: TokenStream,
}

/// A structured predicate from a declaration's where clause.
#[derive(Clone, Debug)]
pub(crate) enum WherePredicateIr {
    Lifetime {
        lifetime: String,
        bounds: Vec<String>,
        declaration: TokenStream,
    },
    Type {
        bounded_type: TypeIr,
        lifetimes: Vec<String>,
        bounds: Vec<GenericBoundIr>,
        declaration: TokenStream,
    },
    Other(TokenStream),
}

/// One lifetime, type, or const generic parameter.
#[derive(Clone, Debug)]
pub(crate) struct GenericParamIr {
    /// The parameter name without lifetime punctuation.
    pub(crate) name: String,
    /// The parameter's lifetime, type, or const category.
    pub(crate) kind: GenericKindIr,
    /// Bounds attached directly to this parameter.
    pub(crate) bounds: Vec<GenericBoundIr>,
    /// The declared default type or const expression.
    pub(crate) default: Option<GenericDefaultIr>,
    /// The declared type of a const parameter.
    pub(crate) const_type: Option<TypeIr>,
    /// The complete parameter declaration tokens.
    pub(crate) declaration: TokenStream,
    /// The parameter source span.
    pub(crate) span: Span,
}

/// A generic lifetime or trait bound.
#[derive(Clone, Debug)]
pub(crate) enum GenericBoundIr {
    Lifetime(String),
    Trait {
        path: PathIr,
        modifier: TraitBoundModifierIr,
        lifetimes: Vec<String>,
    },
    Other(TokenStream),
}

/// The optionality modifier of a trait bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TraitBoundModifierIr {
    None,
    Maybe,
}

/// A type or const generic default.
#[derive(Clone, Debug)]
pub(crate) enum GenericDefaultIr {
    Type(TypeIr),
    Const(TokenStream),
}

/// The kind of a source generic parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GenericKindIr {
    Lifetime,
    Type,
    Const,
}

/// A normalized Rust source visibility.
#[derive(Clone, Debug)]
pub(crate) enum VisibilityIr {
    Public,
    Crate,
    Super,
    SelfValue,
    Restricted(PathIr),
    Inherited,
}

impl TypeDeclarationIr {
    /// Counts occurrences of `name` on the type declaration.
    pub(crate) fn helper_count(&self, name: crate::ir::HelperName) -> usize {
        self.attributes
            .iter()
            .filter(|attribute| attribute.name == name)
            .count()
    }
}
