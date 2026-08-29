//! Semantic intermediate representation for reflection declarations.

#![allow(
    dead_code,
    reason = "the expansion tasks consume the complete IR introduced by this parser task"
)]

mod attribute;
mod declaration;
mod type_ir;

pub(crate) use attribute::{
    ExternalTraitIr, HelperAttributeIr, HelperName, HelperTarget, HelperValueIr,
    SpecializationBindingIr, SpecializationIr,
};
pub(crate) use declaration::{
    AssociatedConstIr, AssociatedTypeIr, DeclarationIr, FieldIr, GenericKindIr, GenericParamIr,
    GenericsIr, ImplDeclarationIr, MacroKind, MethodIr, ParsedDeclaration, ReturnTypeIr,
    TraitDeclarationIr, TypeDeclarationIr, TypeDeclarationKindIr, ValidatedDeclaration, VariantIr,
    VariantKindIr, VisibilityIr,
};
pub(crate) use type_ir::{PathIr, TypeIr, TypeKindIr};
