// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Immutable declarations and concrete instances of reflected methods.

mod builder;
mod instance;
mod invocation_adapter;
mod signature;

pub use self::builder::MethodDescriptorBuilder;
pub use self::instance::MethodImplementationSource;
pub use self::instance::MethodInstanceBuildError;
pub use self::instance::MethodInstanceDescriptor;
pub use self::invocation_adapter::CatchingAvailability;
pub use self::invocation_adapter::InvocationAdapter;
pub use self::invocation_adapter::InvocationUnavailableReason;
pub use self::signature::MethodQualifiers;
pub use self::signature::MethodVisibility;
pub use self::signature::ParameterDescriptor;
pub use self::signature::ParameterPassingMode;
pub use self::signature::ParameterPatternDescriptor;
pub use self::signature::ReceiverDescriptor;
pub use self::signature::ReturnDescriptor;
pub use self::signature::ReturnKind;
use crate::descriptor::ImplDefinitionDescriptor;
use crate::descriptor::TraitDefinitionDescriptor;
use crate::descriptor::trait_descriptor::TraitApplicationSubstitutions;
use crate::expression::GenericDefinitionDescriptor;
use crate::identity::MemberId;

/// An immutable reflected method declaration.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// use qubit_reflect::{Reflect, TypeDescriptor};
/// #[cfg(feature = "derive")]
/// use qubit_reflect::reflect_impl;
///
/// #[cfg(feature = "derive")]
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// struct Service;
///
/// #[cfg(feature = "derive")]
/// #[reflect_impl(crate = qubit_reflect)]
/// impl Service {
///     fn ping(&self) {}
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn main() -> Result<(), qubit_reflect::error::RegistryError> {
/// let method = TypeDescriptor::of::<Service>()
///     .impls()?
///     .first()
///     .and_then(|implementation| implementation.method("ping"))
///     .expect("reflected method");
/// assert_eq!(method.rust_name(), "ping");
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "derive"))]
/// # fn main() {}
/// ```
#[derive(Clone, Debug)]
pub struct MethodDescriptor {
    identity: MemberId,
    rust_name: &'static str,
    query_name: &'static str,
    visibility: MethodVisibility,
    receiver: Option<ReceiverDescriptor>,
    parameters: Box<[ParameterDescriptor]>,
    return_value: ReturnDescriptor,
    qualifiers: MethodQualifiers,
    generic_definition: GenericDefinitionDescriptor,
    has_default: bool,
    declaration_owner: MethodDeclarationOwner,
}

/// The declaration that owns a method descriptor.
#[derive(Clone, Copy, Debug)]
pub enum MethodDeclarationOwner {
    /// A method declared by a trait definition.
    Trait(&'static TraitDefinitionDescriptor),
    /// A method explicitly declared by an impl definition.
    Impl(&'static ImplDefinitionDescriptor),
}

impl MethodDescriptor {
    /// Starts a builder for one method declaration.
    ///
    /// The member identity remains independent of `query_name`, so renaming a
    /// method does not change its Rust identity.
    #[must_use]
    pub fn builder(
        identity: MemberId,
        rust_name: &'static str,
        query_name: &'static str,
        declaration_owner: MethodDeclarationOwner,
    ) -> MethodDescriptorBuilder {
        MethodDescriptorBuilder::new(
            identity,
            rust_name,
            query_name,
            declaration_owner,
        )
    }

    /// Returns the stable composite member identity.
    #[must_use]
    #[inline(always)]
    pub const fn identity(&self) -> &MemberId {
        &self.identity
    }

    /// Returns the Rust declaration name.
    #[must_use]
    #[inline(always)]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the lookup name.
    #[must_use]
    #[inline(always)]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns normalized source visibility facts.
    #[must_use]
    #[inline(always)]
    pub const fn visibility(&self) -> &MethodVisibility {
        &self.visibility
    }

    /// Returns the receiver, or `None` for an associated function.
    #[must_use]
    #[inline(always)]
    pub const fn receiver(&self) -> Option<&ReceiverDescriptor> {
        self.receiver.as_ref()
    }

    /// Returns non-receiver parameters in source declaration order.
    #[must_use]
    #[inline(always)]
    pub const fn parameters(&self) -> &[ParameterDescriptor] {
        &self.parameters
    }

    /// Finds a uniquely named identifier parameter.
    ///
    /// `None` means no parameter has the requested identifier.
    #[must_use]
    pub fn parameter(&self, name: &str) -> Option<&ParameterDescriptor> {
        self.parameters
            .iter()
            .find(|parameter| parameter.name() == Some(name))
    }

    /// Returns a non-receiver parameter by declaration index.
    ///
    /// `None` means `index` is outside the parameter range.
    #[must_use]
    pub fn parameter_at(&self, index: usize) -> Option<&ParameterDescriptor> {
        self.parameters.get(index)
    }

    /// Returns the declared return facts.
    #[must_use]
    #[inline(always)]
    pub const fn return_value(&self) -> &ReturnDescriptor {
        &self.return_value
    }

    /// Returns callability-related source qualifiers.
    #[must_use]
    #[inline(always)]
    pub const fn qualifiers(&self) -> &MethodQualifiers {
        &self.qualifiers
    }

    /// Returns generic parameters and predicates in source order.
    #[must_use]
    #[inline(always)]
    pub const fn generic_definition(&self) -> &GenericDefinitionDescriptor {
        &self.generic_definition
    }

    /// Returns whether the trait declaration supplies a default body.
    #[must_use]
    #[inline(always)]
    pub const fn has_default(&self) -> bool {
        self.has_default
    }

    /// Returns the owning trait definition for a trait method.
    ///
    /// `None` means this method is declared by an impl definition.
    #[must_use]
    #[inline(always)]
    pub const fn declaring_trait(
        &self,
    ) -> Option<&'static TraitDefinitionDescriptor> {
        match self.declaration_owner {
            MethodDeclarationOwner::Trait(descriptor) => Some(descriptor),
            MethodDeclarationOwner::Impl(_) => None,
        }
    }

    /// Returns the owning impl definition for an implementation method.
    ///
    /// `None` means this method is declared by a trait definition.
    #[must_use]
    #[inline(always)]
    pub const fn declaring_impl(
        &self,
    ) -> Option<&'static ImplDefinitionDescriptor> {
        match self.declaration_owner {
            MethodDeclarationOwner::Trait(_) => None,
            MethodDeclarationOwner::Impl(descriptor) => Some(descriptor),
        }
    }

    /// Applies concrete trait arguments to every signature relationship while
    /// preserving the declaration identity and source metadata.
    pub(crate) fn substituted_for_trait_application(
        &self,
        substitutions: &TraitApplicationSubstitutions,
    ) -> Self {
        let mut result = self.clone();
        for parameter in &mut result.parameters {
            parameter.signature_type =
                substitutions.type_expression(&parameter.signature_type);
        }
        result.return_value.signature_type = result
            .return_value
            .signature_type
            .as_ref()
            .map(|expression| substitutions.type_expression(expression));
        result.generic_definition.predicates = result
            .generic_definition
            .predicates
            .iter()
            .map(|predicate| substitutions.predicate(predicate))
            .collect();
        result
    }

    /// Returns whether applying the substitutions changes any method-level
    /// signature or predicate fact.
    pub(crate) fn needs_trait_application_substitution(
        &self,
        substitutions: &TraitApplicationSubstitutions,
    ) -> bool {
        self.parameters.iter().any(|parameter| {
            substitutions.type_expression(&parameter.signature_type)
                != parameter.signature_type
        }) || self.return_value.signature_type.as_ref().is_some_and(
            |expression| {
                substitutions.type_expression(expression) != *expression
            },
        ) || self
            .generic_definition
            .predicates
            .iter()
            .any(|predicate| substitutions.predicate(predicate) != *predicate)
    }
}
