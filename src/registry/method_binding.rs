// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Explicit compile-time method specialization registrations.

use std::any::TypeId;

use crate::descriptor::ImplDefinitionDescriptor;
use crate::descriptor::InvocationAdapter;
use crate::descriptor::MethodDescriptor;
use crate::descriptor::TraitId;
use crate::expression::GenericArgument;
use crate::identity::FragmentIdentity;
use crate::identity::MemberId;

/// The selector supplied by a call-site method binding.
///
/// The adapter is already monomorphized by Rust at the call site. This type
/// only identifies the matching reflected declaration; it never attempts to
/// create a specialization from a runtime `TypeId`.
#[derive(Clone, Debug)]
pub struct MethodBinding {
    target: &'static crate::descriptor::TypeDescriptor,
    target_application: Option<crate::expression::TypeExpression>,
    trait_selector: Option<TraitMethodSelector>,
    rust_name: Box<str>,
    method_arguments: Box<[GenericArgument]>,
    adapter: &'static InvocationAdapter,
}

/// Trait declaration and concrete arguments selected by UFCS.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TraitMethodSelector {
    definition: TraitId,
    application: crate::expression::TypeExpression,
}

/// Stable identity of one binding, used for duplicate detection.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MethodBindingIdentity {
    target: TypeId,
    trait_selector: Option<TraitMethodSelector>,
    rust_name: Box<str>,
    method_arguments: Box<[GenericArgument]>,
}

impl MethodBindingIdentity {
    /// Returns the concrete target type selected by this binding.
    #[must_use]
    pub const fn target_type_id(&self) -> TypeId {
        self.target
    }
}

/// A validated binding connected to one concrete impl and method declaration.
#[derive(Clone, Debug)]
pub struct ResolvedMethodBinding {
    binding: MethodBinding,
    target: &'static crate::descriptor::TypeDescriptor,
    implementation: &'static ImplDefinitionDescriptor,
    impl_identity: FragmentIdentity,
    declaration_identity: MemberId,
    effective_method_identity: MemberId,
    impl_arguments: Box<[GenericArgument]>,
}

impl MethodBinding {
    /// Creates a binding for an inherent method.
    ///
    /// `target` is the concrete receiver type selected by the method path;
    /// `rust_name` is its Rust identifier rather than a reflected query alias.
    #[must_use]
    pub fn inherent(
        target: &'static crate::descriptor::TypeDescriptor,
        rust_name: impl Into<Box<str>>,
        method_arguments: impl Into<Box<[GenericArgument]>>,
        adapter: &'static InvocationAdapter,
    ) -> Self {
        Self {
            target,
            target_application: None,
            trait_selector: None,
            rust_name: rust_name.into(),
            method_arguments: method_arguments.into(),
            adapter,
        }
    }

    /// Creates a binding for a trait method selected with UFCS.
    #[must_use]
    pub fn trait_method(
        target: &'static crate::descriptor::TypeDescriptor,
        trait_definition: TraitId,
        trait_application: crate::expression::TypeExpression,
        rust_name: impl Into<Box<str>>,
        method_arguments: impl Into<Box<[GenericArgument]>>,
        adapter: &'static InvocationAdapter,
    ) -> Self {
        Self {
            target,
            target_application: None,
            trait_selector: Some(TraitMethodSelector {
                definition: trait_definition,
                application: trait_application,
            }),
            rust_name: rust_name.into(),
            method_arguments: method_arguments.into(),
            adapter,
        }
    }

    /// Sets the structural concrete target application retained by the call
    /// site.
    #[must_use]
    pub fn with_target_application(mut self, application: crate::expression::TypeExpression) -> Self {
        self.target_application = Some(application);
        self
    }

    /// Returns the stable selector identity used to reject duplicate bindings.
    #[must_use]
    pub fn identity(&self) -> MethodBindingIdentity {
        MethodBindingIdentity {
            target: self.target.type_id(),
            trait_selector: self.trait_selector.clone(),
            rust_name: self.rust_name.clone(),
            method_arguments: self.method_arguments.clone(),
        }
    }

    pub(crate) fn target_descriptor(&self) -> &'static crate::descriptor::TypeDescriptor {
        self.target
    }

    pub(crate) fn target_application(&self) -> Option<&crate::expression::TypeExpression> {
        self.target_application.as_ref()
    }

    pub(crate) fn rust_name(&self) -> &str {
        &self.rust_name
    }

    pub(crate) fn method_arguments(&self) -> &[GenericArgument] {
        &self.method_arguments
    }

    pub(crate) fn trait_selector(&self) -> Option<&TraitMethodSelector> {
        self.trait_selector.as_ref()
    }
}

impl PartialEq for MethodBinding {
    fn eq(&self, other: &Self) -> bool {
        self.identity() == other.identity()
    }
}

impl Eq for MethodBinding {}

impl std::hash::Hash for MethodBinding {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.identity(), state);
    }
}

impl TraitMethodSelector {
    pub(crate) fn definition(&self) -> &TraitId {
        &self.definition
    }

    pub(crate) fn application(&self) -> &crate::expression::TypeExpression {
        &self.application
    }
}

impl ResolvedMethodBinding {
    pub(crate) fn new(
        binding: MethodBinding,
        implementation: &'static ImplDefinitionDescriptor,
        declaration: &MethodDescriptor,
        effective_method: &MethodDescriptor,
        impl_arguments: Box<[GenericArgument]>,
    ) -> Self {
        Self {
            target: binding.target,
            binding,
            implementation,
            impl_identity: implementation.fragment_identity().clone(),
            declaration_identity: declaration.identity().clone(),
            effective_method_identity: effective_method.identity().clone(),
            impl_arguments,
        }
    }

    /// Returns the selected compile-time adapter.
    #[must_use]
    pub const fn adapter(&self) -> &'static InvocationAdapter {
        self.binding.adapter
    }

    /// Returns the generic impl declaration selected by the key.
    #[must_use]
    pub const fn target(&self) -> &'static crate::descriptor::TypeDescriptor {
        self.target
    }

    /// Returns the generic impl definition selected by structural unification.
    #[must_use]
    pub const fn implementation(&self) -> &'static ImplDefinitionDescriptor {
        self.implementation
    }

    /// Returns the owning impl declaration's stable source identity.
    #[must_use]
    pub const fn impl_identity(&self) -> &FragmentIdentity {
        &self.impl_identity
    }

    /// Returns the selected method declaration's stable member identity.
    #[must_use]
    pub const fn declaration_identity(&self) -> &crate::identity::MemberId {
        &self.declaration_identity
    }

    /// Returns the effective method body's stable member identity.
    #[must_use]
    pub const fn effective_method_identity(&self) -> &crate::identity::MemberId {
        &self.effective_method_identity
    }

    /// Returns concrete generic arguments selected for the method.
    #[must_use]
    pub fn method_arguments(&self) -> &[GenericArgument] {
        self.binding.method_arguments()
    }

    /// Returns concrete generic arguments inferred for the selected impl.
    #[must_use]
    pub fn impl_arguments(&self) -> &[GenericArgument] {
        &self.impl_arguments
    }
}
