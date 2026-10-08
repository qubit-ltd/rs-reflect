// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Substitutions from one concrete trait application into declaration facts.

use std::collections::HashMap;
use std::collections::HashSet;

use crate::descriptor::TraitDefinitionDescriptor;
use crate::expression::ConcretePathSegment;
use crate::expression::ConstExpression;
use crate::expression::GenericArgument;
use crate::expression::GenericDefinitionDescriptor;
use crate::expression::GenericParameterDescriptor;
use crate::expression::PredicateDescriptor;
use crate::expression::TypeExpression;

/// Concrete substitutions carried by one applied trait descriptor.
#[derive(Clone)]
pub(crate) struct TraitApplicationSubstitutions {
    /// Concrete type arguments indexed by declaration parameter name.
    types: HashMap<crate::expression::ExpressionName, TypeExpression>,
    /// Concrete const arguments indexed by declaration parameter name.
    consts: HashMap<crate::expression::ExpressionName, ConstExpression>,
    /// Lifetime parameters represented by the static trait-object root.
    lifetimes: HashSet<crate::expression::ExpressionName>,
    /// Concrete associated-type equalities indexed by associated item name.
    associated_types: HashMap<crate::expression::ExpressionName, TypeExpression>,
}

impl TraitApplicationSubstitutions {
    /// Builds substitutions from declaration-order runtime identity arguments.
    ///
    /// # Parameters
    ///
    /// - `definition`: Trait declaration whose generic parameters define the
    ///   argument order.
    /// - `arguments`: Concrete type and const arguments in declaration order.
    /// - `associated_type_arguments`: Concrete associated-type equalities.
    ///
    /// # Returns
    ///
    /// Returns substitutions for the supplied trait application.
    pub(in crate::descriptor::trait_descriptor) fn new(
        definition: &TraitDefinitionDescriptor,
        arguments: &[GenericArgument],
        associated_type_arguments: &[GenericArgument],
    ) -> Self {
        let mut types = HashMap::new();
        let mut consts = HashMap::new();
        let mut lifetimes = HashSet::new();
        let mut arguments = arguments.iter();
        for parameter in definition.generic_definition().parameters.iter() {
            match parameter {
                GenericParameterDescriptor::Lifetime { name, .. } => {
                    lifetimes.insert(name.clone());
                }
                GenericParameterDescriptor::Type { name, .. } => {
                    if let Some(GenericArgument::Type(value)) = arguments.next() {
                        types.insert(name.clone(), value.clone());
                    }
                }
                GenericParameterDescriptor::Const { name, .. } => {
                    if let Some(GenericArgument::Const(value)) = arguments.next() {
                        consts.insert(name.clone(), value.value.clone());
                    }
                }
            }
        }
        let associated_types = associated_type_arguments
            .iter()
            .filter_map(|argument| match argument {
                GenericArgument::AssociatedType { name, value } => Some((name.clone(), value.as_ref().clone())),
                _ => None,
            })
            .collect();
        Self {
            types,
            consts,
            lifetimes,
            associated_types,
        }
    }

    /// Returns whether this application carries no substitutions.
    ///
    /// # Returns
    ///
    /// Returns `true` when no type, const, lifetime, or associated-type
    /// substitution is present.
    pub(in crate::descriptor::trait_descriptor) fn is_empty(&self) -> bool {
        self.types.is_empty() && self.consts.is_empty() && self.lifetimes.is_empty() && self.associated_types.is_empty()
    }

    /// Applies outer trait arguments inside a nested item generic definition
    /// while preserving names shadowed by the item's own parameters.
    ///
    /// # Parameters
    ///
    /// - `definition`: Nested generic definition to substitute.
    ///
    /// # Returns
    ///
    /// Returns the generic definition with applicable outer substitutions.
    pub(in crate::descriptor::trait_descriptor) fn generic_definition(
        &self,
        definition: &GenericDefinitionDescriptor,
    ) -> GenericDefinitionDescriptor {
        let mut scoped = self.clone();
        for parameter in &definition.parameters {
            match parameter {
                GenericParameterDescriptor::Lifetime { name, .. } => {
                    scoped.lifetimes.remove(name.as_str());
                }
                GenericParameterDescriptor::Type { name, .. } => {
                    scoped.types.remove(name.as_str());
                }
                GenericParameterDescriptor::Const { name, .. } => {
                    scoped.consts.remove(name.as_str());
                }
            }
        }
        let parameters = definition
            .parameters
            .iter()
            .map(|parameter| match parameter {
                GenericParameterDescriptor::Lifetime {
                    name,
                    bounds,
                    diagnostic,
                } => GenericParameterDescriptor::Lifetime {
                    name: name.clone(),
                    bounds: bounds.iter().map(|bound| scoped.lifetime(bound)).collect(),
                    diagnostic: diagnostic.clone(),
                },
                GenericParameterDescriptor::Type {
                    name,
                    bounds,
                    default,
                    diagnostic,
                } => GenericParameterDescriptor::Type {
                    name: name.clone(),
                    bounds: bounds.iter().map(|bound| scoped.predicate(bound)).collect(),
                    default: default.as_ref().map(|value| scoped.type_expression(value)),
                    diagnostic: diagnostic.clone(),
                },
                GenericParameterDescriptor::Const {
                    name,
                    ty,
                    default,
                    diagnostic,
                } => GenericParameterDescriptor::Const {
                    name: name.clone(),
                    ty: Box::new(scoped.type_expression(ty)),
                    default: default.as_ref().map(|value| scoped.const_expression(value)),
                    diagnostic: diagnostic.clone(),
                },
            })
            .collect();
        let predicates = definition
            .predicates
            .iter()
            .map(|predicate| scoped.predicate(predicate))
            .collect();
        GenericDefinitionDescriptor {
            parameters,
            predicates,
            diagnostic: definition.diagnostic.clone(),
        }
    }

    /// Substitutes one structural type expression recursively.
    ///
    /// # Parameters
    ///
    /// - `expression`: Type expression to transform.
    ///
    /// # Returns
    ///
    /// Returns the expression with this application's substitutions applied.
    pub(crate) fn type_expression(&self, expression: &TypeExpression) -> TypeExpression {
        match expression {
            TypeExpression::Parameter(name) => self
                .types
                .get(name.as_str())
                .cloned()
                .unwrap_or_else(|| expression.clone()),
            TypeExpression::Concrete(concrete)
                if concrete.path.len() == 1 && self.types.contains_key(concrete.path[0].as_ref()) =>
            {
                self.types
                    .get(concrete.path[0].as_ref())
                    .expect("the guarded type substitution exists")
                    .clone()
            }
            TypeExpression::Concrete(concrete)
                if concrete.path.len() == 2
                    && concrete.path[0].as_ref() == "Self"
                    && self.associated_types.contains_key(concrete.path[1].as_ref()) =>
            {
                self.associated_types
                    .get(concrete.path[1].as_ref())
                    .expect("the guarded associated-type substitution exists")
                    .clone()
            }
            TypeExpression::Associated(associated)
                if matches!(associated.self_type.as_ref(), TypeExpression::SelfType)
                    && self.associated_types.contains_key(associated.item.as_str()) =>
            {
                self.associated_types[associated.item.as_str()].clone()
            }
            _ => {
                let mut result = expression.clone();
                match &mut result {
                    TypeExpression::Concrete(concrete) => {
                        concrete.segments = concrete
                            .segments
                            .iter()
                            .map(|segment| {
                                ConcretePathSegment::new(
                                    segment.name(),
                                    segment
                                        .arguments()
                                        .iter()
                                        .map(|argument| self.generic_argument(argument))
                                        .collect::<Box<[_]>>(),
                                )
                            })
                            .collect();
                        concrete.arguments = concrete
                            .segments
                            .last()
                            .map_or_else(Box::default, |segment| segment.arguments().to_vec().into_boxed_slice());
                    }
                    TypeExpression::Associated(associated) => {
                        *associated.self_type = self.type_expression(&associated.self_type);
                        associated.trait_path = associated
                            .trait_path
                            .as_ref()
                            .map(|path| Box::new(self.type_expression(path)));
                        associated.arguments = associated
                            .arguments
                            .iter()
                            .map(|argument| self.generic_argument(argument))
                            .collect();
                    }
                    TypeExpression::Reference(reference) => {
                        reference.lifetime = self.lifetime(&reference.lifetime);
                        *reference.target = self.type_expression(&reference.target);
                    }
                    TypeExpression::RawPointer(pointer) => {
                        *pointer.target = self.type_expression(&pointer.target);
                    }
                    TypeExpression::Slice(element) => {
                        **element = self.type_expression(element);
                    }
                    TypeExpression::Array(array) => {
                        *array.element = self.type_expression(&array.element);
                        array.length = self.const_expression(&array.length);
                    }
                    TypeExpression::Tuple(elements) => {
                        *elements = elements.iter().map(|element| self.type_expression(element)).collect();
                    }
                    TypeExpression::FunctionPointer(function) => {
                        function.parameters = function
                            .parameters
                            .iter()
                            .map(|parameter| self.type_expression(parameter))
                            .collect();
                        *function.return_type = self.type_expression(&function.return_type);
                    }
                    TypeExpression::TraitObject(object) => {
                        object.bounds = object
                            .bounds
                            .iter()
                            .map(|predicate| self.predicate(predicate))
                            .collect();
                    }
                    TypeExpression::Opaque(opaque) => {
                        opaque.bounds = opaque
                            .bounds
                            .iter()
                            .map(|predicate| self.predicate(predicate))
                            .collect();
                    }
                    TypeExpression::Parameter(_) | TypeExpression::SelfType | TypeExpression::Never => {}
                }
                result
            }
        }
    }

    /// Substitutes one generic argument recursively.
    ///
    /// # Parameters
    ///
    /// - `argument`: Generic argument to transform.
    ///
    /// # Returns
    ///
    /// Returns the argument with nested types, lifetimes, and predicates
    /// substituted.
    fn generic_argument(&self, argument: &GenericArgument) -> GenericArgument {
        match argument {
            GenericArgument::Type(value) => GenericArgument::Type(self.type_expression(value)),
            GenericArgument::Lifetime(value) => GenericArgument::Lifetime(self.lifetime(value)),
            GenericArgument::Const(value) => {
                let mut value = value.clone();
                value.declared_type = Box::new(self.type_expression(&value.declared_type));
                value.value = self.const_expression(&value.value);
                GenericArgument::Const(value)
            }
            GenericArgument::AssociatedType { name, value } => GenericArgument::AssociatedType {
                name: name.clone(),
                value: Box::new(self.type_expression(value)),
            },
            GenericArgument::AssociatedTypeBound { name, bounds } => GenericArgument::AssociatedTypeBound {
                name: name.clone(),
                bounds: bounds.iter().map(|predicate| self.predicate(predicate)).collect(),
            },
        }
    }

    /// Substitutes one const parameter reference.
    ///
    /// # Parameters
    ///
    /// - `expression`: Const expression to transform.
    ///
    /// # Returns
    ///
    /// Returns the concrete const expression when a matching parameter exists,
    /// or a clone of the original expression otherwise.
    fn const_expression(&self, expression: &ConstExpression) -> ConstExpression {
        match expression {
            ConstExpression::Parameter(name) => self
                .consts
                .get(name.as_str())
                .cloned()
                .unwrap_or_else(|| expression.clone()),
            _ => expression.clone(),
        }
    }

    /// Maps declaration lifetimes to the only lifetime supported by a
    /// `'static` trait-object root.
    ///
    /// # Parameters
    ///
    /// - `lifetime`: Lifetime expression to transform.
    ///
    /// # Returns
    ///
    /// Returns `'static` for a declared lifetime parameter, or the original
    /// lifetime expression when it is not substituted.
    fn lifetime(&self, lifetime: &crate::expression::LifetimeExpression) -> crate::expression::LifetimeExpression {
        match lifetime {
            crate::expression::LifetimeExpression::Named(name) if self.lifetimes.contains(name.as_str()) => {
                crate::expression::LifetimeExpression::Static
            }
            _ => lifetime.clone(),
        }
    }

    /// Substitutes types nested in one predicate.
    ///
    /// # Parameters
    ///
    /// - `predicate`: Predicate whose contained types or lifetimes are
    ///   substituted.
    ///
    /// # Returns
    ///
    /// Returns the predicate with this application's substitutions applied.
    pub(crate) fn predicate(&self, predicate: &PredicateDescriptor) -> PredicateDescriptor {
        let mut result = predicate.clone();
        match &mut result {
            PredicateDescriptor::TypeBound { subject, bounds, .. } => {
                *subject = self.type_expression(subject);
                *bounds = bounds.iter().map(|bound| self.type_expression(bound)).collect();
            }
            PredicateDescriptor::LifetimeOutlives { lifetime, bounds, .. } => {
                *lifetime = self.lifetime(lifetime);
                *bounds = bounds.iter().map(|bound| self.lifetime(bound)).collect();
            }
            PredicateDescriptor::TypeOutlives { ty, lifetime, .. } => {
                *ty = self.type_expression(ty);
                *lifetime = self.lifetime(lifetime);
            }
            PredicateDescriptor::TypeEquality { left, right, .. } => {
                *left = self.type_expression(left);
                *right = self.type_expression(right);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use super::TraitApplicationSubstitutions;
    use crate::descriptor::TraitCompleteness;
    use crate::descriptor::TraitDefinitionDescriptor;
    use crate::descriptor::TraitId;
    use crate::expression::ArrayTypeExpression;
    use crate::expression::AssociatedTypeExpression;
    use crate::expression::ConcretePathSegment;
    use crate::expression::ConcreteTypeExpression;
    use crate::expression::ConstExpression;
    use crate::expression::ConstGenericArgument;
    use crate::expression::DiagnosticText;
    use crate::expression::FunctionAbi;
    use crate::expression::FunctionPointerExpression;
    use crate::expression::FunctionSafety;
    use crate::expression::GenericArgument;
    use crate::expression::GenericDefinitionDescriptor;
    use crate::expression::GenericParameterDescriptor;
    use crate::expression::LifetimeExpression;
    use crate::expression::OpaqueTypeExpression;
    use crate::expression::PredicateDescriptor;
    use crate::expression::RawPointerTypeExpression;
    use crate::expression::ReferenceTypeExpression;
    use crate::expression::TraitBoundModifier;
    use crate::expression::TraitObjectExpression;
    use crate::expression::TypeExpression;

    /// Creates a trait application with type, const, lifetime, and
    /// associated-type bindings.
    fn substitutions_for<Marker: 'static>() -> TraitApplicationSubstitutions {
        let usize_type = concrete("usize", []);
        let generic = Box::leak(Box::new(GenericDefinitionDescriptor::new(
            [
                GenericParameterDescriptor::Lifetime {
                    name: "a".into(),
                    bounds: Box::new([]),
                    diagnostic: DiagnosticText::default(),
                },
                GenericParameterDescriptor::Type {
                    name: "T".into(),
                    bounds: Box::new([]),
                    default: None,
                    diagnostic: DiagnosticText::default(),
                },
                GenericParameterDescriptor::Const {
                    name: "N".into(),
                    ty: Box::new(usize_type.clone()),
                    default: None,
                    diagnostic: DiagnosticText::default(),
                },
            ],
            [],
        )));
        let definition = TraitDefinitionDescriptor::new(
            TraitId::Reflected(TypeId::of::<Marker>()),
            "Fixture",
            "fixture::Fixture",
            "fixture",
            TraitCompleteness::Complete,
            generic,
        );
        TraitApplicationSubstitutions::new(
            &definition,
            &[
                GenericArgument::Type(concrete("u32", [])),
                GenericArgument::Const(ConstGenericArgument::new(
                    usize_type,
                    ConstExpression::UnsignedInteger(4),
                    "4",
                )),
            ],
            &[GenericArgument::AssociatedType {
                name: "Item".into(),
                value: Box::new(concrete("String", [])),
            }],
        )
    }

    /// Builds a single-segment concrete type with the supplied generic
    /// arguments.
    fn concrete(name: &str, arguments: impl IntoIterator<Item = GenericArgument>) -> TypeExpression {
        TypeExpression::Concrete(
            ConcreteTypeExpression::new([name], arguments).expect("fixture type path is non-empty"),
        )
    }

    /// Builds a type-bound predicate with one ordered trait bound.
    fn bound(subject: TypeExpression, bound_type: TypeExpression) -> PredicateDescriptor {
        PredicateDescriptor::type_bound(subject, [bound_type], [TraitBoundModifier::None], [])
            .expect("fixture bound metadata matches")
    }

    #[test]
    fn test_trait_application_substitutions_nested_types() {
        struct NestedTypes;
        let substitutions = substitutions_for::<NestedTypes>();
        let t = TypeExpression::Parameter("T".into());
        let u32_type = concrete("u32", []);
        let input = [
            TypeExpression::RawPointer(RawPointerTypeExpression::new(false, t.clone())),
            TypeExpression::Slice(Box::new(t.clone())),
            TypeExpression::Array(ArrayTypeExpression::new(
                t.clone(),
                ConstExpression::Parameter("N".into()),
            )),
            concrete("Vec", [GenericArgument::Type(t.clone())]),
            TypeExpression::Concrete(
                ConcreteTypeExpression::from_segments([
                    ConcretePathSegment::new("Outer", [GenericArgument::Type(t.clone())]),
                    ConcretePathSegment::new(
                        "Inner",
                        [GenericArgument::Const(ConstGenericArgument::new(
                            concrete("usize", []),
                            ConstExpression::Parameter("N".into()),
                            "N",
                        ))],
                    ),
                ])
                .expect("fixture path is non-empty"),
            ),
            TypeExpression::Reference(ReferenceTypeExpression::new(
                LifetimeExpression::Named("a".into()),
                false,
                t.clone(),
            )),
            TypeExpression::Tuple(vec![t.clone(), concrete("bool", [])].into_boxed_slice()),
            TypeExpression::FunctionPointer(FunctionPointerExpression::new(
                FunctionAbi::Rust,
                FunctionSafety::Safe,
                false,
                [],
                [t.clone()],
                t.clone(),
            )),
            TypeExpression::TraitObject(TraitObjectExpression::new([bound(
                TypeExpression::SelfType,
                concrete("Bound", [GenericArgument::Type(t.clone())]),
            )])),
            TypeExpression::Opaque(OpaqueTypeExpression::new([bound(
                TypeExpression::SelfType,
                concrete("Bound", [GenericArgument::Type(t)]),
            )])),
        ];
        let expected = [
            TypeExpression::RawPointer(RawPointerTypeExpression::new(false, u32_type.clone())),
            TypeExpression::Slice(Box::new(u32_type.clone())),
            TypeExpression::Array(ArrayTypeExpression::new(
                u32_type.clone(),
                ConstExpression::UnsignedInteger(4),
            )),
            concrete("Vec", [GenericArgument::Type(u32_type.clone())]),
            TypeExpression::Concrete(
                ConcreteTypeExpression::from_segments([
                    ConcretePathSegment::new("Outer", [GenericArgument::Type(u32_type.clone())]),
                    ConcretePathSegment::new(
                        "Inner",
                        [GenericArgument::Const(ConstGenericArgument::new(
                            concrete("usize", []),
                            ConstExpression::UnsignedInteger(4),
                            "4",
                        ))],
                    ),
                ])
                .expect("fixture path is non-empty"),
            ),
            TypeExpression::Reference(ReferenceTypeExpression::new(
                LifetimeExpression::Static,
                false,
                u32_type.clone(),
            )),
            TypeExpression::Tuple(vec![u32_type.clone(), concrete("bool", [])].into_boxed_slice()),
            TypeExpression::FunctionPointer(FunctionPointerExpression::new(
                FunctionAbi::Rust,
                FunctionSafety::Safe,
                false,
                [],
                [u32_type.clone()],
                u32_type.clone(),
            )),
            TypeExpression::TraitObject(TraitObjectExpression::new([bound(
                TypeExpression::SelfType,
                concrete("Bound", [GenericArgument::Type(u32_type.clone())]),
            )])),
            TypeExpression::Opaque(OpaqueTypeExpression::new([bound(
                TypeExpression::SelfType,
                concrete("Bound", [GenericArgument::Type(u32_type)]),
            )])),
        ];
        for (original, expected) in input.iter().zip(expected) {
            let before = original.clone();
            assert_eq!(substitutions.type_expression(original), expected);
            assert_eq!(*original, before);
        }
    }

    #[test]
    fn test_trait_application_substitutions_associated_and_shadowed_names() {
        struct AssociatedAndShadowed;
        let substitutions = substitutions_for::<AssociatedAndShadowed>();
        let string_type = concrete("String", []);
        assert_eq!(
            substitutions.type_expression(&concrete("Self", [])),
            concrete("Self", [])
        );
        assert_eq!(
            substitutions.type_expression(&TypeExpression::Concrete(
                ConcreteTypeExpression::new(["Self", "Item"], []).expect("fixture path is non-empty"),
            )),
            string_type,
        );
        assert_eq!(
            substitutions.type_expression(&TypeExpression::Associated(AssociatedTypeExpression::new(
                TypeExpression::SelfType,
                None,
                "Item",
                [],
            ))),
            concrete("String", []),
        );

        let inner = GenericDefinitionDescriptor::new(
            [
                GenericParameterDescriptor::Lifetime {
                    name: "a".into(),
                    bounds: Box::new([LifetimeExpression::Named("a".into())]),
                    diagnostic: DiagnosticText::default(),
                },
                GenericParameterDescriptor::Type {
                    name: "T".into(),
                    bounds: Box::new([bound(TypeExpression::Parameter("T".into()), concrete("Bound", []))]),
                    default: Some(TypeExpression::Parameter("T".into())),
                    diagnostic: DiagnosticText::default(),
                },
                GenericParameterDescriptor::Const {
                    name: "N".into(),
                    ty: Box::new(concrete("usize", [])),
                    default: Some(ConstExpression::Parameter("N".into())),
                    diagnostic: DiagnosticText::default(),
                },
            ],
            [PredicateDescriptor::TypeEquality {
                left: TypeExpression::Parameter("T".into()),
                right: TypeExpression::Associated(AssociatedTypeExpression::new(
                    TypeExpression::SelfType,
                    None,
                    "Item",
                    [],
                )),
                diagnostic: DiagnosticText::default(),
            }],
        );
        let transformed = substitutions.generic_definition(&inner);
        assert_eq!(transformed.parameters(), inner.parameters());
        assert_eq!(
            transformed.predicates(),
            &[PredicateDescriptor::TypeEquality {
                left: TypeExpression::Parameter("T".into()),
                right: concrete("String", []),
                diagnostic: DiagnosticText::default(),
            }],
        );
        assert_eq!(
            inner.predicates()[0],
            PredicateDescriptor::TypeEquality {
                left: TypeExpression::Parameter("T".into()),
                right: TypeExpression::Associated(AssociatedTypeExpression::new(
                    TypeExpression::SelfType,
                    None,
                    "Item",
                    []
                )),
                diagnostic: DiagnosticText::default(),
            }
        );
    }

    #[test]
    fn test_trait_application_substitutions_predicates() {
        struct Predicates;
        let substitutions = substitutions_for::<Predicates>();
        let a = LifetimeExpression::Named("a".into());
        let b = LifetimeExpression::Named("b".into());
        let t = TypeExpression::Parameter("T".into());
        let u32_type = concrete("u32", []);
        let input = [
            bound(t.clone(), concrete("Bound", [GenericArgument::Type(t.clone())])),
            PredicateDescriptor::lifetime_outlives(a.clone(), [a.clone(), b.clone()])
                .expect("fixture has lifetime bounds"),
            PredicateDescriptor::TypeOutlives {
                ty: t.clone(),
                lifetime: a.clone(),
                diagnostic: DiagnosticText::default(),
            },
            PredicateDescriptor::TypeEquality {
                left: t,
                right: concrete("Vec", [GenericArgument::Type(TypeExpression::Parameter("T".into()))]),
                diagnostic: DiagnosticText::default(),
            },
        ];
        let expected = [
            bound(
                u32_type.clone(),
                concrete("Bound", [GenericArgument::Type(u32_type.clone())]),
            ),
            PredicateDescriptor::lifetime_outlives(LifetimeExpression::Static, [LifetimeExpression::Static, b])
                .expect("fixture has lifetime bounds"),
            PredicateDescriptor::TypeOutlives {
                ty: u32_type.clone(),
                lifetime: LifetimeExpression::Static,
                diagnostic: DiagnosticText::default(),
            },
            PredicateDescriptor::TypeEquality {
                left: u32_type.clone(),
                right: concrete("Vec", [GenericArgument::Type(u32_type)]),
                diagnostic: DiagnosticText::default(),
            },
        ];
        for (original, expected) in input.iter().zip(expected) {
            let before = original.clone();
            assert_eq!(substitutions.predicate(original), expected);
            assert_eq!(*original, before);
        }
    }
}
