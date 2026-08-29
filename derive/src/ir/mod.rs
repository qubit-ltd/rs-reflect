//! Semantic intermediate representation for reflection declarations.

#![allow(
    dead_code,
    reason = "the expansion tasks consume the complete IR introduced by this parser task"
)]

mod attribute;
mod declaration;
mod type_ir;

pub(crate) use attribute::ExternalTraitIr;
pub(crate) use attribute::HelperAttributeIr;
pub(crate) use attribute::HelperName;
pub(crate) use attribute::HelperTarget;
pub(crate) use attribute::HelperValueIr;
pub(crate) use attribute::SpecializationBindingIr;
pub(crate) use attribute::SpecializationIr;
pub(crate) use attribute::SpecializationValueIr;
pub(crate) use declaration::AssociatedConstIr;
pub(crate) use declaration::AssociatedTypeIr;
pub(crate) use declaration::DeclarationIr;
pub(crate) use declaration::FieldIr;
pub(crate) use declaration::GenericBoundIr;
pub(crate) use declaration::GenericDefaultIr;
pub(crate) use declaration::GenericKindIr;
pub(crate) use declaration::GenericParamIr;
pub(crate) use declaration::GenericsIr;
pub(crate) use declaration::ImplDeclarationIr;
pub(crate) use declaration::MacroKind;
pub(crate) use declaration::MethodIr;
pub(crate) use declaration::MethodQualifiersIr;
pub(crate) use declaration::ParameterIr;
pub(crate) use declaration::ParameterPatternIr;
pub(crate) use declaration::ParameterPatternKindIr;
pub(crate) use declaration::ParsedDeclaration;
pub(crate) use declaration::ReceiverIr;
pub(crate) use declaration::ReceiverKindIr;
pub(crate) use declaration::ReturnTypeIr;
pub(crate) use declaration::TraitBoundModifierIr;
pub(crate) use declaration::TraitDeclarationIr;
pub(crate) use declaration::TypeDeclarationIr;
pub(crate) use declaration::TypeDeclarationKindIr;
pub(crate) use declaration::ValidatedDeclaration;
pub(crate) use declaration::VariantIr;
pub(crate) use declaration::VariantKindIr;
pub(crate) use declaration::VisibilityIr;
pub(crate) use declaration::WherePredicateIr;
pub(crate) use type_ir::PathArgumentIr;
pub(crate) use type_ir::PathArgumentsIr;
pub(crate) use type_ir::PathIr;
pub(crate) use type_ir::PathSegmentIr;
pub(crate) use type_ir::QualifiedSelfIr;
pub(crate) use type_ir::TypeIr;
pub(crate) use type_ir::TypeKindIr;
