// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
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
#[derive(Debug)]
pub struct MethodDescriptorBuilder {
    identity: MemberId,
    rust_name: &'static str,
    query_name: &'static str,
    visibility: MethodVisibility,
    receiver: Option<ReceiverDescriptor>,
    parameters: Vec<ParameterDescriptor>,
    return_value: ReturnDescriptor,
    qualifiers: MethodQualifiers,
    generic_definition: GenericDefinitionDescriptor,
    has_default: bool,
    declaration_owner: MethodDeclarationOwner,
}

impl MethodDescriptorBuilder {
    /// Creates a builder with ordinary private, non-generic method defaults.
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
    #[must_use]
    pub fn visibility(mut self, visibility: MethodVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Sets the receiver; `None` describes an associated function.
    #[must_use]
    pub fn receiver(mut self, receiver: Option<ReceiverDescriptor>) -> Self {
        self.receiver = receiver;
        self
    }

    /// Sets non-receiver parameters in source order.
    #[must_use]
    pub fn parameters(mut self, parameters: Vec<ParameterDescriptor>) -> Self {
        self.parameters = parameters;
        self
    }

    /// Sets the return declaration.
    #[must_use]
    pub fn return_value(mut self, return_value: ReturnDescriptor) -> Self {
        self.return_value = return_value;
        self
    }

    /// Sets source qualifiers that affect invocation availability.
    #[must_use]
    pub fn qualifiers(mut self, qualifiers: MethodQualifiers) -> Self {
        self.qualifiers = qualifiers;
        self
    }

    /// Copies the method's generic declaration and preserves source order.
    #[must_use]
    pub fn generic_definition(
        mut self,
        generic_definition: &GenericDefinitionDescriptor,
    ) -> Self {
        self.generic_definition = generic_definition.clone();
        self
    }

    /// Records whether the declared trait method has a default body.
    #[must_use]
    pub fn has_default(mut self, has_default: bool) -> Self {
        self.has_default = has_default;
        self
    }

    /// Builds the immutable declaration.
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
