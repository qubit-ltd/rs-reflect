// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! ImplDescriptorBuilder metadata and behavior.

use super::impl_kind::validate_kind;
use crate::descriptor::AssociatedConstBindingDescriptor;
use crate::descriptor::AssociatedTypeBindingDescriptor;
use crate::descriptor::ImplDefinitionDescriptor;
use crate::descriptor::ImplDescriptor;
use crate::descriptor::ImplDescriptorBuildError;
use crate::descriptor::MethodDescriptor;
use crate::descriptor::MethodInstanceDescriptor;
use crate::descriptor::TraitDescriptor;
use crate::descriptor::TypeDescriptorResolver;
use crate::descriptor::trait_descriptor::generic_argument_is_concrete;
use crate::expression::GenericArgument;
use crate::expression::GenericParameterDescriptor;

/// Builds a concrete impl while preserving source order.
///
/// The builder validates generic arguments and member ownership before
/// producing an [`ImplDescriptor`].
///
/// # Examples
///
/// Generated registration normally supplies the declaration descriptor. This
/// example constructs the declaration facts directly and builds one concrete
/// descriptor without initializing the global inventory registry.
///
/// ```
/// mod example {
///     use std::sync::LazyLock;
///     use qubit_reflect::descriptor::ImplDefinitionDescriptor;
///     use qubit_reflect::descriptor::ImplKind;
///     use qubit_reflect::expression::ConcreteTypeExpression;
///     use qubit_reflect::expression::GenericDefinitionDescriptor;
///     use qubit_reflect::expression::TypeExpression;
///     use qubit_reflect::identity::FragmentIdentity;
///
///     static GENERICS: LazyLock<GenericDefinitionDescriptor> =
///         LazyLock::new(|| GenericDefinitionDescriptor::new([], []));
///     pub static DEFINITION: LazyLock<ImplDefinitionDescriptor> = LazyLock::new(|| {
///         ImplDefinitionDescriptor::new(
///             FragmentIdentity::new("example", module_path!(), line!(), 1, "impl", 1),
///             TypeExpression::Concrete(
///                 ConcreteTypeExpression::new(["example", "Service"], [])
///                     .expect("non-empty target path"),
///             ),
///             ImplKind::Inherent,
///             None,
///             &GENERICS,
///         )
///         .expect("valid inherent impl")
///     });
/// }
/// let target = qubit_reflect::TypeDescriptor::of::<u64>();
/// let implementation = qubit_reflect::descriptor::ImplDescriptor::builder(
///     &example::DEFINITION,
///     qubit_reflect::TypeDescriptor::of::<u64>,
/// )
/// .build()
/// .expect("valid concrete impl");
/// assert_eq!(implementation.target_type().type_id(), target.type_id());
/// ```
#[derive(Debug)]
pub struct ImplDescriptorBuilder {
    /// Impl declaration being instantiated.
    definition: &'static ImplDefinitionDescriptor,
    /// Resolver for the concrete target type.
    target_type: TypeDescriptorResolver,
    /// Concrete trait namespace, when this is a trait impl.
    implemented_trait: Option<&'static TraitDescriptor>,
    /// Methods declared by the impl definition.
    methods: &'static [MethodDescriptor],
    /// Effective concrete method instances.
    method_instances: Vec<MethodInstanceDescriptor>,
    /// Concrete associated type bindings.
    associated_types: Vec<AssociatedTypeBindingDescriptor>,
    /// Concrete associated constant bindings.
    associated_consts: Vec<AssociatedConstBindingDescriptor>,
    /// Concrete generic arguments in definition order.
    arguments: Vec<GenericArgument>,
}

impl ImplDescriptorBuilder {
    /// Creates an empty concrete instance builder.
    ///
    /// # Parameters
    ///
    /// - `definition`: Impl declaration to instantiate.
    /// - `target_type`: Resolver for its concrete target type.
    ///
    /// # Returns
    ///
    /// Returns an empty builder for the impl application.
    #[must_use]
    pub(in crate::descriptor::impl_descriptor) fn new(
        definition: &'static ImplDefinitionDescriptor,
        target_type: TypeDescriptorResolver,
    ) -> Self {
        Self {
            definition,
            target_type,
            implemented_trait: None,
            methods: &[],
            method_instances: Vec::new(),
            associated_types: Vec::new(),
            associated_consts: Vec::new(),
            arguments: Vec::new(),
        }
    }

    /// Sets the applied trait implemented by this instance.
    ///
    /// # Parameters
    ///
    /// - `implemented_trait`: Concrete applied trait namespace.
    ///
    /// # Returns
    ///
    /// Returns the builder with the trait namespace set.
    #[must_use]
    #[inline]
    pub fn implemented_trait(mut self, implemented_trait: &'static TraitDescriptor) -> Self {
        self.implemented_trait = Some(implemented_trait);
        self
    }

    /// Sets methods explicitly declared by the impl definition.
    ///
    /// # Parameters
    ///
    /// - `methods`: Methods declared by the source impl.
    ///
    /// # Returns
    ///
    /// Returns the builder with its declared methods set.
    #[must_use]
    #[inline]
    pub fn methods(mut self, methods: &'static [MethodDescriptor]) -> Self {
        self.methods = methods;
        self
    }

    /// Sets concrete effective method instances.
    ///
    /// # Parameters
    ///
    /// - `instances`: Effective method instances for this application.
    ///
    /// # Returns
    ///
    /// Returns the builder with its effective method instances set.
    #[must_use]
    #[inline]
    pub fn method_instances(mut self, instances: Vec<MethodInstanceDescriptor>) -> Self {
        self.method_instances = instances;
        self
    }

    /// Sets associated type bindings in declaration order.
    ///
    /// # Parameters
    ///
    /// - `bindings`: Concrete associated type bindings.
    ///
    /// # Returns
    ///
    /// Returns the builder with its associated type bindings set.
    #[must_use]
    #[inline]
    pub fn associated_types(mut self, bindings: Vec<AssociatedTypeBindingDescriptor>) -> Self {
        self.associated_types = bindings;
        self
    }

    /// Sets associated constant bindings in declaration order.
    ///
    /// # Parameters
    ///
    /// - `bindings`: Concrete associated constant bindings.
    ///
    /// # Returns
    ///
    /// Returns the builder with its associated constant bindings set.
    #[must_use]
    #[inline]
    pub fn associated_consts(mut self, bindings: Vec<AssociatedConstBindingDescriptor>) -> Self {
        self.associated_consts = bindings;
        self
    }

    /// Sets concrete impl arguments in definition parameter order.
    ///
    /// # Parameters
    ///
    /// - `arguments`: Concrete type and const arguments in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the builder with its generic arguments set.
    #[must_use]
    #[inline]
    pub fn arguments(mut self, arguments: Vec<GenericArgument>) -> Self {
        self.arguments = arguments;
        self
    }

    /// Validates and builds the concrete impl descriptor.
    ///
    /// Returns [`ImplDescriptorBuildError`] for inconsistent trait, generic,
    /// method, or associated-item relationships.
    ///
    /// # Returns
    ///
    /// Returns the validated concrete implementation descriptor.
    ///
    /// # Errors
    ///
    /// Returns [`ImplDescriptorBuildError`] when implementation facts do not
    /// match their source definition or descriptor graph.
    pub fn build(self) -> Result<ImplDescriptor, ImplDescriptorBuildError> {
        validate_kind(self.definition.kind(), self.implemented_trait.is_some())?;
        let expected_arguments = self
            .definition
            .generic_definition()
            .parameters
            .iter()
            .filter(|parameter| !matches!(parameter, GenericParameterDescriptor::Lifetime { .. }))
            .count();
        if expected_arguments != self.arguments.len()
            || self
                .arguments
                .iter()
                .any(|argument| !generic_argument_is_concrete(argument))
        {
            return Err(ImplDescriptorBuildError::GenericArgumentsDoNotMatchDefinition);
        }
        let kinds_match = self
            .definition
            .generic_definition()
            .parameters
            .iter()
            .filter(|parameter| !matches!(parameter, GenericParameterDescriptor::Lifetime { .. }))
            .zip(&self.arguments)
            .all(|(parameter, argument)| {
                matches!(
                    (parameter, argument),
                    (GenericParameterDescriptor::Type { .. }, GenericArgument::Type(_))
                        | (GenericParameterDescriptor::Const { .. }, GenericArgument::Const(_))
                )
            });
        if !kinds_match {
            return Err(ImplDescriptorBuildError::GenericArgumentsDoNotMatchDefinition);
        }
        if let (Some(expected), Some(actual)) = (self.definition.implemented_trait(), self.implemented_trait)
            && actual.definition().trait_id() != expected.trait_id()
        {
            return Err(ImplDescriptorBuildError::ImplementedTraitDefinitionMismatch);
        }
        if self.methods.iter().any(|method| {
            !method
                .declaring_impl()
                .is_some_and(|owner| std::ptr::eq(owner, self.definition))
        }) {
            return Err(ImplDescriptorBuildError::ForeignMember);
        }
        if let Some(applied_trait) = self.implemented_trait {
            let foreign_method = self.method_instances.iter().any(|instance| {
                !applied_trait
                    .methods()
                    .iter()
                    .any(|method| std::ptr::eq(method, instance.declaration()))
                    || instance
                        .implementation_method()
                        .is_some_and(|method| !self.methods.iter().any(|candidate| std::ptr::eq(candidate, method)))
            });
            let foreign_type = self.associated_types.iter().any(|binding| {
                !applied_trait
                    .associated_types()
                    .iter()
                    .any(|item| std::ptr::eq(item, binding.declaration()))
            });
            let foreign_const = self.associated_consts.iter().any(|binding| {
                !applied_trait
                    .associated_consts()
                    .iter()
                    .any(|item| std::ptr::eq(item, binding.declaration()))
            });
            if foreign_method || foreign_type || foreign_const {
                return Err(ImplDescriptorBuildError::ForeignMember);
            }
        } else if self.method_instances.iter().any(|instance| {
            instance.implementation_source() != crate::descriptor::MethodImplementationSource::Declared
                || !self
                    .methods
                    .iter()
                    .any(|method| std::ptr::eq(method, instance.declaration()))
                || instance.implementation_method().is_some()
        }) {
            return Err(ImplDescriptorBuildError::ForeignMember);
        }
        Ok(ImplDescriptor {
            definition: self.definition,
            target_type: self.target_type,
            implemented_trait: self.implemented_trait,
            methods: self.methods,
            method_instances: self.method_instances.into_boxed_slice(),
            associated_types: self.associated_types.into_boxed_slice(),
            associated_consts: self.associated_consts.into_boxed_slice(),
            arguments: self.arguments.into_boxed_slice(),
        })
    }
}
