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
    SpecializationBindingIr, SpecializationIr, SpecializationValueIr,
};
pub(crate) use declaration::{
    AssociatedConstIr, AssociatedTypeIr, DeclarationIr, FieldIr, GenericBoundIr, GenericDefaultIr,
    GenericKindIr, GenericParamIr, GenericsIr, ImplDeclarationIr, MacroKind, MethodIr,
    MethodQualifiersIr, ParameterIr, ParameterPatternIr, ParameterPatternKindIr, ParsedDeclaration,
    ReceiverIr, ReceiverKindIr, ReturnTypeIr, TraitBoundModifierIr, TraitDeclarationIr,
    TypeDeclarationIr, TypeDeclarationKindIr, ValidatedDeclaration, VariantIr, VariantKindIr,
    VisibilityIr, WherePredicateIr,
};
pub(crate) use type_ir::{
    PathArgumentIr, PathArgumentsIr, PathIr, PathSegmentIr, QualifiedSelfIr, TypeIr, TypeKindIr,
};
