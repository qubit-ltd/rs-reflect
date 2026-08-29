//! Declaration-level semantic IR shared by validation and expansion.

use proc_macro2::{Ident, Span, TokenStream};

use crate::ir::{ExternalTraitIr, HelperAttributeIr, PathIr, SpecializationIr, TypeIr};

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
    pub(crate) supertraits: Vec<TokenStream>,
    /// Helpers supplied to the outer trait macro.
    pub(crate) attributes: Vec<HelperAttributeIr>,
    /// Explicit mappings for external trait bounds.
    pub(crate) external_traits: Vec<ExternalTraitIr>,
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
    pub(crate) receiver: Option<TokenStream>,
    /// Non-receiver parameter name, type, and pattern facts.
    pub(crate) parameters: Vec<(Option<String>, TypeIr, TokenStream)>,
    /// The semantic method return category.
    pub(crate) return_type: ReturnTypeIr,
    /// Const, async, unsafe, and ABI qualifier tokens.
    pub(crate) qualifiers: TokenStream,
    /// Method-level reflection helpers.
    pub(crate) attributes: Vec<HelperAttributeIr>,
    /// Concrete method specializations in source order.
    pub(crate) specializations: Vec<SpecializationIr>,
    /// The method-name span used for diagnostics.
    pub(crate) span: Span,
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
    pub(crate) where_predicates: Vec<TokenStream>,
    /// The complete generic declaration tokens.
    pub(crate) declaration: TokenStream,
}

/// One lifetime, type, or const generic parameter.
#[derive(Clone, Debug)]
pub(crate) struct GenericParamIr {
    /// The parameter name without lifetime punctuation.
    pub(crate) name: String,
    /// The parameter's lifetime, type, or const category.
    pub(crate) kind: GenericKindIr,
    /// The complete parameter declaration tokens.
    pub(crate) declaration: TokenStream,
    /// The parameter source span.
    pub(crate) span: Span,
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
