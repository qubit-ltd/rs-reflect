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
    pub(in crate::descriptor::trait_descriptor) fn is_empty(&self) -> bool {
        self.types.is_empty() && self.consts.is_empty() && self.lifetimes.is_empty() && self.associated_types.is_empty()
    }

    /// Applies outer trait arguments inside a nested item generic definition
    /// while preserving names shadowed by the item's own parameters.
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
                        concrete.arguments = concrete
                            .arguments
                            .iter()
                            .map(|argument| self.generic_argument(argument))
                            .collect();
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
    fn lifetime(&self, lifetime: &crate::expression::LifetimeExpression) -> crate::expression::LifetimeExpression {
        match lifetime {
            crate::expression::LifetimeExpression::Named(name) if self.lifetimes.contains(name.as_str()) => {
                crate::expression::LifetimeExpression::Static
            }
            _ => lifetime.clone(),
        }
    }

    /// Substitutes types nested in one predicate.
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
