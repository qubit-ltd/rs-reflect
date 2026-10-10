// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Validated construction of applied trait descriptors.

use std::collections::HashSet;
use std::ptr::eq;

use super::AppliedTraitId;
use super::AssociatedConstDescriptor;
use super::AssociatedTypeDescriptor;
use super::TraitApplicationSubstitutions;
use super::TraitCompleteness;
use super::TraitDefinitionDescriptor;
use super::TraitDescriptor;
use super::TraitDescriptorBuildError;
use super::TraitDescriptorRef;
use crate::descriptor::MethodDescriptor;
use crate::expression::ConstExpression;
use crate::expression::GenericArgument;
use crate::expression::GenericParameterDescriptor;
use crate::expression::TypeExpression;

/// Builds one applied trait and validates its supertrait closure.
///
/// # Examples
///
/// ```
/// use std::any::TypeId;
/// use std::sync::LazyLock;
/// use qubit_reflect::descriptor::{TraitCompleteness, TraitDefinitionDescriptor, TraitDescriptor, TraitId};
/// use qubit_reflect::expression::GenericDefinitionDescriptor;
///
/// struct Marker;
/// static GENERICS: LazyLock<GenericDefinitionDescriptor> =
///     LazyLock::new(|| GenericDefinitionDescriptor::new([], []));
/// static DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
///     TraitDefinitionDescriptor::new(
///         TraitId::Reflected(TypeId::of::<Marker>()),
///         "Example",
///         "example::Example",
///         "Example",
///         TraitCompleteness::Complete,
///         &GENERICS,
///     )
/// });
/// let applied = TraitDescriptor::builder(&DEFINITION).build().expect("valid application");
/// assert_eq!(applied.rust_name(), "Example");
/// ```
#[derive(Debug)]
pub struct TraitDescriptorBuilder {
    /// Shared declaration for the applied trait being built.
    definition: &'static TraitDefinitionDescriptor,
    /// Concrete type and const arguments in declaration order.
    arguments: Vec<GenericArgument>,
    /// Concrete associated-type equalities in declaration order.
    associated_type_arguments: Vec<GenericArgument>,
    /// Direct applied supertraits in source declaration order.
    direct_supertraits: Vec<TraitDescriptorRef>,
    /// Applied method declarations in source order.
    methods: &'static [MethodDescriptor],
    /// Applied associated-type descriptors in source order.
    associated_types: Vec<AssociatedTypeDescriptor>,
    /// Applied associated-constant descriptors in source order.
    associated_consts: Vec<AssociatedConstDescriptor>,
}

impl TraitDescriptorBuilder {
    /// Creates an empty applied view for `definition`.
    ///
    /// # Parameters
    ///
    /// - `definition`: Shared source declaration for this application.
    ///
    /// # Returns
    ///
    /// Returns a builder with no application-specific facts.
    #[inline]
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
    ///
    /// # Parameters
    ///
    /// - `arguments`: Concrete type and const arguments.
    ///
    /// # Returns
    ///
    /// Returns this builder with the generic arguments replaced.
    #[must_use]
    #[inline]
    pub fn arguments(mut self, arguments: Vec<GenericArgument>) -> Self {
        self.arguments = arguments;
        self
    }

    /// Sets concrete associated-type equalities in declaration order.
    ///
    /// # Parameters
    ///
    /// - `arguments`: Concrete associated-type bindings.
    ///
    /// # Returns
    ///
    /// Returns this builder with the bindings replaced.
    #[must_use]
    #[inline]
    pub fn associated_type_arguments(mut self, arguments: Vec<GenericArgument>) -> Self {
        self.associated_type_arguments = arguments;
        self
    }

    /// Sets direct supertraits in source declaration order.
    ///
    /// # Type Parameters
    ///
    /// - `N`: Number of direct supertraits.
    ///
    /// # Parameters
    ///
    /// - `direct_supertraits`: Applied direct supertraits in source order.
    ///
    /// # Returns
    ///
    /// Returns this builder with its direct supertraits replaced.
    #[must_use]
    #[inline]
    pub fn direct_supertraits<const N: usize>(mut self, direct_supertraits: [&'static TraitDescriptor; N]) -> Self {
        self.direct_supertraits = direct_supertraits.into_iter().map(TraitDescriptorRef::new).collect();
        self
    }

    /// Sets applied method declarations in source order.
    ///
    /// # Parameters
    ///
    /// - `methods`: Method descriptors declared by this trait application.
    ///
    /// # Returns
    ///
    /// Returns this builder with its methods replaced.
    #[must_use]
    #[inline]
    pub fn methods(mut self, methods: &'static [MethodDescriptor]) -> Self {
        self.methods = methods;
        self
    }

    /// Sets applied associated types in source order.
    ///
    /// # Parameters
    ///
    /// - `associated_types`: Applied associated-type descriptors.
    ///
    /// # Returns
    ///
    /// Returns this builder with its associated types replaced.
    #[must_use]
    #[inline]
    pub fn associated_types(mut self, associated_types: Vec<AssociatedTypeDescriptor>) -> Self {
        self.associated_types = associated_types;
        self
    }

    /// Sets applied associated constants in source order.
    ///
    /// # Parameters
    ///
    /// - `associated_consts`: Applied associated-constant descriptors.
    ///
    /// # Returns
    ///
    /// Returns this builder with its associated constants replaced.
    #[must_use]
    #[inline]
    pub fn associated_consts(mut self, associated_consts: Vec<AssociatedConstDescriptor>) -> Self {
        self.associated_consts = associated_consts;
        self
    }

    /// Validates and builds the applied trait descriptor.
    ///
    /// Returns [`TraitDescriptorBuildError`] for recursive supertraits or when
    /// an incomplete external trait claims supertraits or associated items.
    ///
    /// # Returns
    ///
    /// Returns the validated applied trait descriptor.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid generic arguments, associated-type
    /// bindings, methods, supertraits, or incomplete external facts.
    pub fn build(self) -> Result<TraitDescriptor, TraitDescriptorBuildError> {
        self.validate_arguments()?;
        self.validate_associated_type_arguments()?;
        if self.methods.iter().any(|method| {
            !method
                .declaring_trait()
                .is_some_and(|owner| eq(owner, self.definition))
        }) {
            return Err(TraitDescriptorBuildError::ForeignMethod);
        }
        if self.definition.completeness() == TraitCompleteness::ExternalIncomplete
            && (!self.direct_supertraits.is_empty()
                || !self.associated_types.is_empty()
                || !self.associated_consts.is_empty())
        {
            return Err(TraitDescriptorBuildError::ExternalTraitHasUnprovenFacts);
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
            associated_type_arguments: self.associated_type_arguments.clone().into_boxed_slice(),
        };
        let substitutions =
            TraitApplicationSubstitutions::new(self.definition, &self.arguments, &self.associated_type_arguments);
        let methods = if substitutions.is_empty()
            || !self
                .methods
                .iter()
                .any(|method| method.needs_trait_application_substitution(&substitutions))
        {
            self.methods
        } else {
            Box::leak(
                self.methods
                    .iter()
                    .map(|method| method.substituted_for_trait_application(&substitutions))
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
            associated_type_arguments: self.associated_type_arguments.into_boxed_slice(),
            direct_supertraits: self.direct_supertraits.into_boxed_slice(),
            all_supertraits: all_supertraits.into_boxed_slice(),
            methods,
            associated_types: associated_types.into_boxed_slice(),
            associated_consts: associated_consts.into_boxed_slice(),
        })
    }

    /// Adds `candidate` and its direct ancestors while rejecting recursion and
    /// duplicate applied identities.
    ///
    /// # Parameters
    ///
    /// - `candidate`: Direct or transitive applied supertrait to inspect.
    /// - `closure`: Accumulated duplicate-free transitive closure.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after adding all ancestors.
    ///
    /// # Errors
    ///
    /// Returns `RecursiveSupertrait` if the candidate graph reaches the
    /// application being built.
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
        if closure.iter().any(|existing| existing.same_application(candidate)) {
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
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the supplied arguments satisfy the declaration.
    ///
    /// # Errors
    ///
    /// Returns an error for the wrong argument count, kind, or concreteness.
    fn validate_arguments(&self) -> Result<(), TraitDescriptorBuildError> {
        if self.definition.completeness() == TraitCompleteness::ExternalIncomplete {
            for (index, argument) in self.arguments.iter().enumerate() {
                if !generic_argument_is_concrete(argument) {
                    return Err(TraitDescriptorBuildError::NonConcreteGenericArgument { index });
                }
            }
            return Ok(());
        }
        let parameters: Vec<_> = self
            .definition
            .generic_definition()
            .parameters
            .iter()
            .filter(|parameter| !matches!(parameter, GenericParameterDescriptor::Lifetime { .. }))
            .collect();
        if parameters.len() != self.arguments.len() {
            return Err(TraitDescriptorBuildError::GenericArgumentCount {
                expected: parameters.len(),
                actual: self.arguments.len(),
            });
        }
        for (index, (parameter, argument)) in parameters.into_iter().zip(&self.arguments).enumerate() {
            let kind_matches = matches!(
                (parameter, argument),
                (GenericParameterDescriptor::Type { .. }, GenericArgument::Type(_))
                    | (GenericParameterDescriptor::Const { .. }, GenericArgument::Const(_))
            );
            if !kind_matches {
                return Err(TraitDescriptorBuildError::GenericArgumentKind { index });
            }
            if !generic_argument_is_concrete(argument) {
                return Err(TraitDescriptorBuildError::NonConcreteGenericArgument { index });
            }
        }
        Ok(())
    }

    /// Verifies associated-type equalities are concrete, unique, and declared
    /// by this trait or one of its direct supertraits.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when every associated-type binding is valid.
    ///
    /// # Errors
    ///
    /// Returns `InvalidAssociatedTypeArgument` for an unknown, duplicate,
    /// symbolic, or malformed binding.
    fn validate_associated_type_arguments(&self) -> Result<(), TraitDescriptorBuildError> {
        let mut names = HashSet::new();
        for argument in &self.associated_type_arguments {
            let GenericArgument::AssociatedType { name, value } = argument else {
                return Err(TraitDescriptorBuildError::InvalidAssociatedTypeArgument);
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
                            || descriptor.all_supertraits().iter().any(|ancestor| {
                                ancestor
                                    .associated_types()
                                    .iter()
                                    .any(|item| item.rust_name() == name.as_ref())
                            })
                    }))
                || !type_expression_is_concrete(value)
            {
                return Err(TraitDescriptorBuildError::InvalidAssociatedTypeArgument);
            }
        }
        Ok(())
    }
}

/// Returns whether an argument contains only concrete runtime identity facts.
///
/// # Parameters
///
/// - `argument`: Generic argument to inspect.
///
/// # Returns
///
/// Returns `true` when the argument can be used in a concrete trait identity.
pub(in crate::descriptor) fn generic_argument_is_concrete(argument: &GenericArgument) -> bool {
    match argument {
        GenericArgument::Type(expression) => type_expression_is_concrete(expression),
        GenericArgument::Const(argument) => !matches!(argument.value, ConstExpression::Parameter(_)),
        GenericArgument::Lifetime(_) => true,
        GenericArgument::AssociatedType { value, .. } => type_expression_is_concrete(value),
        GenericArgument::AssociatedTypeBound { .. } => false,
    }
}

/// Returns whether a substituted type expression contains no symbolic type.
///
/// # Parameters
///
/// - `expression`: Type expression to inspect.
///
/// # Returns
///
/// Returns `true` when every type and const component is concrete.
fn type_expression_is_concrete(expression: &TypeExpression) -> bool {
    match expression {
        TypeExpression::Concrete(concrete) => concrete.arguments.iter().all(generic_argument_is_concrete),
        TypeExpression::Reference(reference) => type_expression_is_concrete(&reference.target),
        TypeExpression::RawPointer(pointer) => type_expression_is_concrete(&pointer.target),
        TypeExpression::Slice(element) => type_expression_is_concrete(element),
        TypeExpression::Array(array) => {
            type_expression_is_concrete(&array.element) && !matches!(array.length, ConstExpression::Parameter(_))
        }
        TypeExpression::Tuple(elements) => elements.iter().all(type_expression_is_concrete),
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
