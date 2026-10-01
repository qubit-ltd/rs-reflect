// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Construction of immutable method declarations.

use super::MethodDeclarationOwner;
use super::MethodDescriptor;
use super::MethodQualifiers;
use super::MethodVisibility;
use super::ParameterDescriptor;
use super::ReceiverDescriptor;
use super::ReturnDescriptor;
use crate::expression::GenericDefinitionDescriptor;
use crate::identity::MemberId;
use crate::identity::Visibility;

/// Builds a declaration while preserving source order for all collections.
///
/// # Examples
///
/// ```
/// use std::any::TypeId;
/// use std::sync::LazyLock;
/// use qubit_reflect::descriptor::{MethodDeclarationOwner, MethodDescriptor, TraitCompleteness, TraitDefinitionDescriptor, TraitId};
/// use qubit_reflect::expression::GenericDefinitionDescriptor;
/// use qubit_reflect::identity::{FragmentIdentity, MemberId};
///
/// struct Marker;
/// static GENERICS: LazyLock<GenericDefinitionDescriptor> =
///     LazyLock::new(|| GenericDefinitionDescriptor::new([], []));
/// static DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
///     TraitDefinitionDescriptor::new(
///         TraitId::Reflected(TypeId::of::<Marker>()), "Service", "example::Service", "Service",
///         TraitCompleteness::Complete, &GENERICS,
///     )
/// });
/// let method = MethodDescriptor::builder(
///     MemberId::new("example::Service", "method", 0,
///         FragmentIdentity::new("example", "example", 1, 1, "method", 1)),
///     "ping", "ping", MethodDeclarationOwner::Trait(&DEFINITION),
/// );
/// let rebuilt = method.visibility(qubit_reflect::descriptor::MethodVisibility::InheritedFromTrait).build();
/// assert_eq!(rebuilt.rust_name(), "ping");
/// ```
#[derive(Debug)]
pub struct MethodDescriptorBuilder {
    /// Stable source identity, independent of reflected query naming.
    identity: MemberId,
    /// Rust declaration identifier.
    rust_name: &'static str,
    /// Reflected lookup identifier.
    query_name: &'static str,
    /// Normalized source visibility.
    visibility: MethodVisibility,
    /// Receiver declaration, absent for an associated function.
    receiver: Option<ReceiverDescriptor>,
    /// Non-receiver parameters in source order.
    parameters: Vec<ParameterDescriptor>,
    /// Declared return kind and type.
    return_value: ReturnDescriptor,
    /// Source qualifiers affecting invocation.
    qualifiers: MethodQualifiers,
    /// Method generic parameters and predicates.
    generic_definition: GenericDefinitionDescriptor,
    /// Whether a trait declaration supplies a default body.
    has_default: bool,
    /// Trait or impl declaration that owns the method.
    declaration_owner: MethodDeclarationOwner,
}

impl MethodDescriptorBuilder {
    /// Creates a builder with ordinary private, non-generic method defaults.
    ///
    /// # Parameters
    ///
    /// - `identity`: Stable source identity for the method.
    /// - `rust_name`: Rust declaration identifier.
    /// - `query_name`: Reflected lookup name.
    /// - `declaration_owner`: Trait or impl declaration that owns the method.
    ///
    /// # Returns
    ///
    /// Returns a builder initialized with private visibility, no receiver or
    /// parameters, unit return, and no method generics.
    pub(super) fn new(
        identity: MemberId,
        rust_name: &'static str,
        query_name: &'static str,
        declaration_owner: MethodDeclarationOwner,
    ) -> Self {
        Self {
            identity,
            rust_name,
            query_name,
            visibility: MethodVisibility::Declared(Visibility::Private),
            receiver: None,
            parameters: Vec::new(),
            return_value: ReturnDescriptor::unit(),
            qualifiers: MethodQualifiers::default(),
            generic_definition: GenericDefinitionDescriptor {
                parameters: Box::new([]),
                predicates: Box::new([]),
                diagnostic: Default::default(),
            },
            has_default: false,
            declaration_owner,
        }
    }

    /// Sets normalized source visibility.
    ///
    /// # Parameters
    ///
    /// - `visibility`: Visibility facts extracted from the source declaration.
    ///
    /// # Returns
    ///
    /// Returns this builder with the visibility replaced.
    #[must_use]
    pub fn visibility(mut self, visibility: MethodVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Sets the receiver; `None` describes an associated function.
    ///
    /// # Parameters
    ///
    /// - `receiver`: Declared receiver form, or `None` for an associated
    ///   function.
    ///
    /// # Returns
    ///
    /// Returns this builder with the receiver replaced.
    #[must_use]
    pub fn receiver(mut self, receiver: Option<ReceiverDescriptor>) -> Self {
        self.receiver = receiver;
        self
    }

    /// Sets non-receiver parameters in source order.
    ///
    /// # Parameters
    ///
    /// - `parameters`: Non-receiver parameter descriptors in declaration order.
    ///
    /// # Returns
    ///
    /// Returns this builder with its parameters replaced.
    #[must_use]
    pub fn parameters(mut self, parameters: Vec<ParameterDescriptor>) -> Self {
        self.parameters = parameters;
        self
    }

    /// Sets the return declaration.
    ///
    /// # Parameters
    ///
    /// - `return_value`: Return kind and type facts from the declaration.
    ///
    /// # Returns
    ///
    /// Returns this builder with its return declaration replaced.
    #[must_use]
    pub fn return_value(mut self, return_value: ReturnDescriptor) -> Self {
        self.return_value = return_value;
        self
    }

    /// Sets source qualifiers that affect invocation availability.
    ///
    /// # Parameters
    ///
    /// - `qualifiers`: Async, unsafe, ABI, and related source facts.
    ///
    /// # Returns
    ///
    /// Returns this builder with the qualifiers replaced.
    #[must_use]
    pub fn qualifiers(mut self, qualifiers: MethodQualifiers) -> Self {
        self.qualifiers = qualifiers;
        self
    }

    /// Copies the method's generic declaration and preserves source order.
    ///
    /// # Parameters
    ///
    /// - `generic_definition`: Generic parameters and predicates to copy.
    ///
    /// # Returns
    ///
    /// Returns this builder with the method's generic declaration replaced.
    #[must_use]
    pub fn generic_definition(mut self, generic_definition: &GenericDefinitionDescriptor) -> Self {
        self.generic_definition = generic_definition.clone();
        self
    }

    /// Records whether the declared trait method has a default body.
    ///
    /// # Parameters
    ///
    /// - `has_default`: Whether the trait declaration supplies a method body.
    ///
    /// # Returns
    ///
    /// Returns this builder with the default-body fact replaced.
    #[must_use]
    pub fn has_default(mut self, has_default: bool) -> Self {
        self.has_default = has_default;
        self
    }

    /// Builds the immutable declaration.
    ///
    /// # Returns
    ///
    /// Returns the method declaration with all configured source facts.
    #[must_use]
    pub fn build(self) -> MethodDescriptor {
        MethodDescriptor {
            identity: self.identity,
            rust_name: self.rust_name,
            query_name: self.query_name,
            visibility: self.visibility,
            receiver: self.receiver,
            parameters: self.parameters.into_boxed_slice(),
            return_value: self.return_value,
            qualifiers: self.qualifiers,
            generic_definition: self.generic_definition,
            has_default: self.has_default,
            declaration_owner: self.declaration_owner,
        }
    }
}
