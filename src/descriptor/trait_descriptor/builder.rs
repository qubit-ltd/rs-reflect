// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Validated construction of applied trait descriptors.

use std::fmt;

use super::AppliedTraitId;
use super::AssociatedConstDescriptor;
use super::AssociatedTypeDescriptor;
use super::TraitApplicationSubstitutions;
use super::TraitCompleteness;
use super::TraitDefinitionDescriptor;
use super::TraitDescriptor;
use super::TraitDescriptorRef;
use crate::descriptor::MethodDescriptor;
use crate::expression::ConstExpression;
use crate::expression::GenericArgument;
use crate::expression::GenericParameterDescriptor;
use crate::expression::TypeExpression;

/// An invalid applied trait graph or incomplete external declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraitDescriptorBuildError {
    /// A supertrait resolves to the application currently being built.
    RecursiveSupertrait {
        /// Diagnostic Rust path of the recursive application.
        rust_path: &'static str,
    },
    /// An external incomplete trait attempted to claim unobservable facts.
    ExternalTraitHasUnprovenFacts,
    /// The number of concrete type/const arguments does not match the
    /// definition.
    GenericArgumentCount {
        /// Number of runtime identity arguments required by the definition.
        expected: usize,
        /// Number of arguments supplied by the applied descriptor.
        actual: usize,
    },
    /// An argument kind does not match its type or const parameter.
    GenericArgumentKind {
        /// Zero-based runtime identity argument index.
        index: usize,
    },
    /// An argument still contains a symbolic type or const parameter.
    NonConcreteGenericArgument {
        /// Zero-based runtime identity argument index.
        index: usize,
    },
    /// An associated-type argument is unknown, duplicated, or non-concrete.
    InvalidAssociatedTypeArgument,
    /// A method declaration belongs to another trait or an impl.
    ForeignMethod,
}

impl fmt::Display for TraitDescriptorBuildError {
    /// Formats a stable diagnostic message.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecursiveSupertrait { rust_path } => {
                write!(formatter, "recursive supertrait application: {rust_path}")
            }
            Self::ExternalTraitHasUnprovenFacts => {
                formatter.write_str("an external incomplete trait cannot claim supertraits or associated items")
            }
            Self::GenericArgumentCount { expected, actual } => write!(
                formatter,
                "trait application requires {expected} concrete arguments but received {actual}"
            ),
            Self::GenericArgumentKind { index } => {
                write!(formatter, "trait argument {index} has the wrong generic kind")
            }
            Self::NonConcreteGenericArgument { index } => {
                write!(formatter, "trait argument {index} is not concrete")
            }
            Self::InvalidAssociatedTypeArgument => {
                formatter.write_str("an associated-type argument must name one declared item and have a concrete value")
            }
            Self::ForeignMethod => formatter.write_str("applied trait contains a foreign method declaration"),
        }
    }
}

impl std::error::Error for TraitDescriptorBuildError {}

/// Builds one applied trait and validates its supertrait closure.
#[derive(Debug)]
pub struct TraitDescriptorBuilder {
    definition: &'static TraitDefinitionDescriptor,
    arguments: Vec<GenericArgument>,
    associated_type_arguments: Vec<GenericArgument>,
    direct_supertraits: Vec<TraitDescriptorRef>,
    methods: &'static [MethodDescriptor],
    associated_types: Vec<AssociatedTypeDescriptor>,
    associated_consts: Vec<AssociatedConstDescriptor>,
}

impl TraitDescriptorBuilder {
    /// Creates an empty applied view for `definition`.
    pub(super) fn new(definition: &'static TraitDefinitionDescriptor) -> Self {
        Self {
            definition,
            arguments: Vec::new(),
            associated_type_arguments: Vec::new(),
            direct_supertraits: Vec::new(),
            methods: &[],
            associated_types: Vec::new(),
            associated_consts: Vec::new(),
        }
    }

    /// Sets concrete generic arguments in declaration order.
    #[must_use]
    pub fn arguments(mut self, arguments: Vec<GenericArgument>) -> Self {
        self.arguments = arguments;
        self
    }

    /// Sets concrete associated-type equalities in declaration order.
    #[must_use]
    pub fn associated_type_arguments(
        mut self,
        arguments: Vec<GenericArgument>,
    ) -> Self {
        self.associated_type_arguments = arguments;
        self
    }

    /// Sets direct supertraits in source declaration order.
    pub fn direct_supertraits<const N: usize>(
        mut self,
        direct_supertraits: [&'static TraitDescriptor; N],
    ) -> Self {
        self.direct_supertraits = direct_supertraits
            .into_iter()
            .map(TraitDescriptorRef::new)
            .collect();
        self
    }

    /// Sets applied method declarations in source order.
    #[must_use]
    pub fn methods(mut self, methods: &'static [MethodDescriptor]) -> Self {
        self.methods = methods;
        self
    }

    /// Sets applied associated types in source order.
    #[must_use]
    pub fn associated_types(
        mut self,
        associated_types: Vec<AssociatedTypeDescriptor>,
    ) -> Self {
        self.associated_types = associated_types;
        self
    }

    /// Sets applied associated constants in source order.
    #[must_use]
    pub fn associated_consts(
        mut self,
        associated_consts: Vec<AssociatedConstDescriptor>,
    ) -> Self {
        self.associated_consts = associated_consts;
        self
    }

    /// Validates and builds the applied trait descriptor.
    ///
    /// Returns [`TraitDescriptorBuildError`] for recursive supertraits or when
    /// an incomplete external trait claims supertraits or associated items.
    pub fn build(self) -> Result<TraitDescriptor, TraitDescriptorBuildError> {
        self.validate_arguments()?;
        self.validate_associated_type_arguments()?;
        if self.methods.iter().any(|method| {
            !method
                .declaring_trait()
                .is_some_and(|owner| std::ptr::eq(owner, self.definition))
        }) {
            return Err(TraitDescriptorBuildError::ForeignMethod);
        }
        if self.definition.completeness()
            == TraitCompleteness::ExternalIncomplete
            && (!self.direct_supertraits.is_empty()
                || !self.associated_types.is_empty()
                || !self.associated_consts.is_empty())
        {
            return Err(
                TraitDescriptorBuildError::ExternalTraitHasUnprovenFacts,
            );
        }

        let mut all_supertraits = Vec::new();
        for direct in &self.direct_supertraits {
            self.collect_supertrait(direct.descriptor(), &mut all_supertraits)?;
        }
        all_supertraits.sort_by(|left, right| {
            left.rust_path()
                .cmp(right.rust_path())
                .then_with(|| left.query_name().cmp(right.query_name()))
        });

        let trait_id = AppliedTraitId {
            definition: self.definition.trait_id().clone(),
            arguments: self.arguments.clone().into_boxed_slice(),
            associated_type_arguments: self
                .associated_type_arguments
                .clone()
                .into_boxed_slice(),
        };
        let substitutions = TraitApplicationSubstitutions::new(
            self.definition,
            &self.arguments,
            &self.associated_type_arguments,
        );
        let methods = if substitutions.is_empty()
            || !self.methods.iter().any(|method| {
                method.needs_trait_application_substitution(&substitutions)
            }) {
            self.methods
        } else {
            Box::leak(
                self.methods
                    .iter()
                    .map(|method| {
                        method.substituted_for_trait_application(&substitutions)
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            )
        };
        let associated_types = self
            .associated_types
            .into_iter()
            .map(|descriptor| descriptor.substituted(&substitutions))
            .collect::<Vec<_>>();
        let associated_consts = self
            .associated_consts
            .into_iter()
            .map(|descriptor| descriptor.substituted(&substitutions))
            .collect::<Vec<_>>();
        Ok(TraitDescriptor {
            definition: self.definition,
            trait_id,
            arguments: self.arguments.into_boxed_slice(),
            associated_type_arguments: self
                .associated_type_arguments
                .into_boxed_slice(),
            direct_supertraits: self.direct_supertraits.into_boxed_slice(),
            all_supertraits: all_supertraits.into_boxed_slice(),
            methods,
            associated_types: associated_types.into_boxed_slice(),
            associated_consts: associated_consts.into_boxed_slice(),
        })
    }

    /// Adds `candidate` and its direct ancestors while rejecting recursion and
    /// duplicate applied identities.
    fn collect_supertrait(
        &self,
        candidate: &'static TraitDescriptor,
        closure: &mut Vec<TraitDescriptorRef>,
    ) -> Result<(), TraitDescriptorBuildError> {
        if candidate.trait_id().definition() == self.definition.trait_id()
            && candidate.trait_id().arguments() == self.arguments
        {
            return Err(TraitDescriptorBuildError::RecursiveSupertrait {
                rust_path: self.definition.rust_path(),
            });
        }
        if closure
            .iter()
            .any(|existing| existing.same_application(candidate))
        {
            return Ok(());
        }
        closure.push(TraitDescriptorRef::new(candidate));
        for ancestor in candidate.direct_supertraits() {
            self.collect_supertrait(ancestor.descriptor(), closure)?;
        }
        Ok(())
    }

    /// Verifies every runtime identity parameter has one concrete argument of
    /// the matching generic kind.
    fn validate_arguments(&self) -> Result<(), TraitDescriptorBuildError> {
        if self.definition.completeness()
            == TraitCompleteness::ExternalIncomplete
        {
            for (index, argument) in self.arguments.iter().enumerate() {
                if !generic_argument_is_concrete(argument) {
                    return Err(
                        TraitDescriptorBuildError::NonConcreteGenericArgument {
                            index,
                        },
                    );
                }
            }
            return Ok(());
        }
        let parameters: Vec<_> = self
            .definition
            .generic_definition()
            .parameters
            .iter()
            .filter(|parameter| {
                !matches!(
                    parameter,
                    GenericParameterDescriptor::Lifetime { .. }
                )
            })
            .collect();
        if parameters.len() != self.arguments.len() {
            return Err(TraitDescriptorBuildError::GenericArgumentCount {
                expected: parameters.len(),
                actual: self.arguments.len(),
            });
        }
        for (index, (parameter, argument)) in
            parameters.into_iter().zip(&self.arguments).enumerate()
        {
            let kind_matches = matches!(
                (parameter, argument),
                (
                    GenericParameterDescriptor::Type { .. },
                    GenericArgument::Type(_)
                ) | (
                    GenericParameterDescriptor::Const { .. },
                    GenericArgument::Const(_)
                )
            );
            if !kind_matches {
                return Err(TraitDescriptorBuildError::GenericArgumentKind {
                    index,
                });
            }
            if !generic_argument_is_concrete(argument) {
                return Err(
                    TraitDescriptorBuildError::NonConcreteGenericArgument {
                        index,
                    },
                );
            }
        }
        Ok(())
    }

    /// Verifies associated-type equalities are concrete, unique, and declared
    /// by this trait or one of its direct supertraits.
    fn validate_associated_type_arguments(
        &self,
    ) -> Result<(), TraitDescriptorBuildError> {
        let mut names = std::collections::HashSet::new();
        for argument in &self.associated_type_arguments {
            let GenericArgument::AssociatedType { name, value } = argument
            else {
                return Err(
                    TraitDescriptorBuildError::InvalidAssociatedTypeArgument,
                );
            };
            if !names.insert(name.as_ref())
                || !(self
                    .associated_types
                    .iter()
                    .any(|descriptor| descriptor.rust_name() == name.as_ref())
                    || self.direct_supertraits.iter().any(|supertrait| {
                        let descriptor = supertrait.descriptor();
                        descriptor
                            .associated_types()
                            .iter()
                            .any(|item| item.rust_name() == name.as_ref())
                            || descriptor.all_supertraits().iter().any(
                                |ancestor| {
                                    ancestor.associated_types().iter().any(
                                        |item| {
                                            item.rust_name() == name.as_ref()
                                        },
                                    )
                                },
                            )
                    }))
                || !type_expression_is_concrete(value)
            {
                return Err(
                    TraitDescriptorBuildError::InvalidAssociatedTypeArgument,
                );
            }
        }
        Ok(())
    }
}

/// Returns whether an argument contains only concrete runtime identity facts.
pub(in crate::descriptor) fn generic_argument_is_concrete(
    argument: &GenericArgument,
) -> bool {
    match argument {
        GenericArgument::Type(expression) => {
            type_expression_is_concrete(expression)
        }
        GenericArgument::Const(argument) => {
            !matches!(argument.value, ConstExpression::Parameter(_))
        }
        GenericArgument::Lifetime(_) => true,
        GenericArgument::AssociatedType { value, .. } => {
            type_expression_is_concrete(value)
        }
        GenericArgument::AssociatedTypeBound { .. } => false,
    }
}

/// Returns whether a substituted type expression contains no symbolic type.
fn type_expression_is_concrete(expression: &TypeExpression) -> bool {
    match expression {
        TypeExpression::Concrete(concrete) => {
            concrete.arguments.iter().all(generic_argument_is_concrete)
        }
        TypeExpression::Reference(reference) => {
            type_expression_is_concrete(&reference.target)
        }
        TypeExpression::RawPointer(pointer) => {
            type_expression_is_concrete(&pointer.target)
        }
        TypeExpression::Slice(element) => type_expression_is_concrete(element),
        TypeExpression::Array(array) => {
            type_expression_is_concrete(&array.element)
                && !matches!(array.length, ConstExpression::Parameter(_))
        }
        TypeExpression::Tuple(elements) => {
            elements.iter().all(type_expression_is_concrete)
        }
        TypeExpression::FunctionPointer(function) => {
            function.parameters.iter().all(type_expression_is_concrete)
                && type_expression_is_concrete(&function.return_type)
        }
        TypeExpression::TraitObject(_) | TypeExpression::Never => true,
        TypeExpression::Parameter(_)
        | TypeExpression::SelfType
        | TypeExpression::Associated(_)
        | TypeExpression::Opaque(_) => false,
    }
}
