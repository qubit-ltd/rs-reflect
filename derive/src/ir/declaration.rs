// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Declaration-level semantic IR shared by validation and expansion.

// qubit-style: allow multiple-public-types

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
    /// Derive a descriptor for a type.
    Derive,
    /// Expand a reflected trait declaration.
    Trait,
    /// Expand a reflected impl declaration.
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
    /// Struct, enum, or union declaration.
    Type(TypeDeclarationIr),
    /// Trait declaration.
    Trait(TraitDeclarationIr),
    /// Inherent or trait impl declaration.
    Impl(ImplDeclarationIr),
}

/// A struct, enum, or rejected union derive input.
#[derive(Clone, Debug)]
pub(crate) struct TypeDeclarationIr {
    /// The source type identifier.
    pub(crate) name: Ident,
    /// The Rust data-declaration form.
    pub(crate) kind: TypeDeclarationKindIr,
    /// The source field container; enum variants retain their own shapes.
    pub(crate) field_shape: FieldShapeIr,
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

/// Source syntax of a declaration's field container, including empty lists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FieldShapeIr {
    /// A unit declaration without braces or parentheses.
    Unit,
    /// Fields enclosed in braces.
    Named,
    /// Fields enclosed in parentheses.
    Unnamed,
}

/// Distinguishes the three data declaration forms accepted by `syn`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TypeDeclarationKindIr {
    /// Struct declaration.
    Struct,
    /// Enum declaration.
    Enum,
    /// Union declaration, retained for validation diagnostics.
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
    /// Unit variant.
    Unit,
    /// Tuple variant.
    Tuple,
    /// Struct-like variant.
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
    /// Normalized receiver category.
    pub(crate) kind: ReceiverKindIr,
    /// Converted receiver type.
    pub(crate) ty: TypeIr,
    /// Original receiver declaration tokens.
    pub(crate) declaration: TokenStream,
    /// Receiver source span.
    pub(crate) span: Span,
}

/// The core receiver forms relevant to safe adapter selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReceiverKindIr {
    /// `self` by value.
    Value,
    /// `&self`.
    SharedReference,
    /// `&mut self`.
    MutableReference,
    /// Explicit typed receiver syntax.
    Typed,
}

/// One non-receiver method parameter.
#[derive(Clone, Debug)]
pub(crate) struct ParameterIr {
    /// Identifier name when the pattern is a plain binding.
    pub(crate) name: Option<String>,
    /// Normalized parameter pattern facts.
    pub(crate) pattern: ParameterPatternIr,
    /// Converted parameter type.
    pub(crate) ty: TypeIr,
    /// Zero-based position in the method signature.
    pub(crate) index: usize,
    /// Parameter source span.
    pub(crate) span: Span,
}

/// A method parameter pattern represented independently of `syn::Pat`.
#[derive(Clone, Debug)]
pub(crate) struct ParameterPatternIr {
    /// Pattern category used by invocation logic.
    pub(crate) kind: ParameterPatternKindIr,
    /// Diagnostic rendering of the pattern.
    pub(crate) source: String,
    /// Original pattern tokens with source spans.
    pub(crate) tokens: TokenStream,
}

/// Parameter pattern categories that affect named invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ParameterPatternKindIr {
    /// A simple identifier binding.
    Identifier,
    /// A wildcard pattern.
    Wildcard,
    /// Any destructuring pattern.
    Destructure,
}

/// Method qualifiers that affect descriptor facts and adapter safety.
#[derive(Clone, Debug, Default)]
pub(crate) struct MethodQualifiersIr {
    /// Whether the method is declared `const`.
    pub(crate) is_const: bool,
    /// Whether the method is declared `async`.
    pub(crate) is_async: bool,
    /// Whether the method is declared `unsafe`.
    pub(crate) is_unsafe: bool,
    /// Explicit ABI name, when present.
    pub(crate) abi: Option<String>,
    /// Whether the method is variadic.
    pub(crate) is_variadic: bool,
}

/// The semantic return category of a method.
#[derive(Clone, Debug)]
pub(crate) enum ReturnTypeIr {
    /// No explicit return type, equivalent to `()`.
    Unit,
    /// Explicit non-unit return type.
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
    /// Lifetime outlives predicate such as `'a: 'b`.
    Lifetime {
        /// Lifetime on the left-hand side.
        lifetime: String,
        /// Lifetime bounds on the right-hand side.
        bounds: Vec<String>,
        /// Original predicate tokens.
        declaration: TokenStream,
    },
    /// Type bound predicate such as `T: Trait`.
    Type {
        /// Type bounded by the predicate.
        bounded_type: TypeIr,
        /// Higher-ranked lifetimes declared by the predicate.
        lifetimes: Vec<String>,
        /// Trait and lifetime bounds on the type.
        bounds: Vec<GenericBoundIr>,
        /// Original predicate tokens.
        declaration: TokenStream,
    },
    /// Predicate syntax outside the structured forms.
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
    /// Lifetime outlives bound.
    Lifetime(String),
    /// Trait bound with modifiers and higher-ranked lifetimes.
    Trait {
        /// Bound trait path.
        path: PathIr,
        /// Optional `?` modifier.
        modifier: TraitBoundModifierIr,
        /// Higher-ranked lifetime binders.
        lifetimes: Vec<String>,
    },
    /// Bound syntax not covered by the structured variants.
    Other(TokenStream),
}

/// The optionality modifier of a trait bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TraitBoundModifierIr {
    /// Required trait bound.
    None,
    /// Optional trait bound written `?Trait`.
    Maybe,
}

/// A type or const generic default.
#[derive(Clone, Debug)]
pub(crate) enum GenericDefaultIr {
    /// Default type argument.
    Type(TypeIr),
    /// Default const expression tokens.
    Const(TokenStream),
}

/// The kind of a source generic parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GenericKindIr {
    /// Lifetime parameter.
    Lifetime,
    /// Type parameter.
    Type,
    /// Const parameter.
    Const,
}

/// A normalized Rust source visibility.
#[derive(Clone, Debug)]
pub(crate) enum VisibilityIr {
    /// Public visibility.
    Public,
    /// Crate-wide visibility.
    Crate,
    /// Parent-module visibility.
    Super,
    /// Current-module visibility.
    SelfValue,
    /// Restricted path visibility.
    Restricted(PathIr),
    /// Private inherited visibility.
    Inherited,
}

impl TypeDeclarationIr {
    /// Counts occurrences of `name` on the type declaration.
    ///
    /// # Parameters
    ///
    /// - `name`: Helper key to count.
    ///
    /// # Returns
    ///
    /// Returns the number of matching type-level helper attributes.
    pub(crate) fn helper_count(&self, name: crate::ir::HelperName) -> usize {
        self.attributes
            .iter()
            .filter(|attribute| attribute.name == name)
            .count()
    }
}
