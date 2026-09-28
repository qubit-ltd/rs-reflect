// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Immutable declarations and concrete instances of reflected methods.

mod instance;
mod invocation_adapter;
mod method_declaration_owner;
mod method_descriptor_builder;
mod signature;

pub use self::instance::MethodImplementationSource;
pub use self::instance::MethodInstanceBuildError;
pub use self::instance::MethodInstanceDescriptor;
pub use self::invocation_adapter::CatchingAvailability;
pub use self::invocation_adapter::InvocationAdapter;
pub use self::invocation_adapter::InvocationUnavailableReason;
pub use self::method_declaration_owner::MethodDeclarationOwner;
pub use self::method_descriptor_builder::MethodDescriptorBuilder;
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
/// ).build();
/// assert_eq!(method.rust_name(), "ping");
/// ```
#[derive(Clone, Debug)]
pub struct MethodDescriptor {
    /// Stable source identity retained independently of query renaming.
    identity: MemberId,
    /// Rust identifier declared by the source method.
    rust_name: &'static str,
    /// Name used to find this method through reflection.
    query_name: &'static str,
    /// Source visibility normalized into portable descriptor facts.
    visibility: MethodVisibility,
    /// Receiver form, absent for an associated function.
    receiver: Option<ReceiverDescriptor>,
    /// Non-receiver parameters in source order.
    parameters: Box<[ParameterDescriptor]>,
    /// Declared output kind and optional type expression.
    return_value: ReturnDescriptor,
    /// Async, unsafe, ABI, and other callability qualifiers.
    qualifiers: MethodQualifiers,
    /// Method-level generic parameters and predicates.
    generic_definition: GenericDefinitionDescriptor,
    /// Whether a trait declaration supplies a default body.
    has_default: bool,
    /// Trait or impl declaration that owns this method.
    declaration_owner: MethodDeclarationOwner,
}

impl MethodDescriptor {
    /// Starts a builder for one method declaration.
    ///
    /// The member identity remains independent of `query_name`, so renaming a
    /// method does not change its Rust identity.
    ///
    /// # Parameters
    ///
    /// - `identity`: Stable source identity of the method.
    /// - `rust_name`: Rust identifier used in the declaration.
    /// - `query_name`: Name used by reflected lookup.
    /// - `declaration_owner`: Trait or impl declaration that owns the method.
    ///
    /// # Returns
    ///
    /// Returns a builder initialized with the method's stable declaration
    /// facts.
    #[inline]
    #[must_use]
    pub fn builder(
        identity: MemberId,
        rust_name: &'static str,
        query_name: &'static str,
        declaration_owner: MethodDeclarationOwner,
    ) -> MethodDescriptorBuilder {
        MethodDescriptorBuilder::new(identity, rust_name, query_name, declaration_owner)
    }

    /// Returns the stable composite member identity.
    ///
    /// # Returns
    ///
    /// Returns the source identity independent of the reflected query name.
    #[must_use]
    #[inline]
    pub const fn identity(&self) -> &MemberId {
        &self.identity
    }

    /// Returns the Rust declaration name.
    ///
    /// # Returns
    ///
    /// Returns the source identifier used by the Rust declaration.
    #[must_use]
    #[inline]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the lookup name.
    ///
    /// # Returns
    ///
    /// Returns the reflected query name, which may differ from the Rust name.
    #[must_use]
    #[inline]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns normalized source visibility facts.
    ///
    /// # Returns
    ///
    /// Returns visibility and any restricted path recorded by the declaration.
    #[must_use]
    #[inline]
    pub const fn visibility(&self) -> &MethodVisibility {
        &self.visibility
    }

    /// Returns the receiver, or `None` for an associated function.
    ///
    /// # Returns
    ///
    /// Returns the declared receiver, or `None` when the method is associated.
    #[must_use]
    #[inline]
    pub const fn receiver(&self) -> Option<&ReceiverDescriptor> {
        self.receiver.as_ref()
    }

    /// Returns non-receiver parameters in source declaration order.
    ///
    /// # Returns
    ///
    /// Returns the parameter descriptors in their original declaration order.
    #[must_use]
    #[inline]
    pub const fn parameters(&self) -> &[ParameterDescriptor] {
        &self.parameters
    }

    /// Finds a uniquely named identifier parameter.
    ///
    /// `None` means no parameter has the requested identifier.
    ///
    /// # Parameters
    ///
    /// - `name`: Parameter identifier to find.
    ///
    /// # Returns
    ///
    /// Returns the uniquely matching parameter, or `None` when absent.
    #[must_use]
    pub fn parameter(&self, name: &str) -> Option<&ParameterDescriptor> {
        self.parameters.iter().find(|parameter| parameter.name() == Some(name))
    }

    /// Returns a non-receiver parameter by declaration index.
    ///
    /// `None` means `index` is outside the parameter range.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based position in the non-receiver parameter list.
    ///
    /// # Returns
    ///
    /// Returns the parameter at that position, or `None` when out of range.
    #[must_use]
    pub fn parameter_at(&self, index: usize) -> Option<&ParameterDescriptor> {
        self.parameters.get(index)
    }

    /// Returns the declared return facts.
    ///
    /// # Returns
    ///
    /// Returns the source-level return kind and type expression, when present.
    #[must_use]
    #[inline]
    pub const fn return_value(&self) -> &ReturnDescriptor {
        &self.return_value
    }

    /// Returns callability-related source qualifiers.
    ///
    /// # Returns
    ///
    /// Returns the qualifiers recorded for this method declaration.
    #[must_use]
    #[inline]
    pub const fn qualifiers(&self) -> &MethodQualifiers {
        &self.qualifiers
    }

    /// Returns generic parameters and predicates in source order.
    ///
    /// # Returns
    ///
    /// Returns the method's generic definition and predicates.
    #[must_use]
    #[inline]
    pub const fn generic_definition(&self) -> &GenericDefinitionDescriptor {
        &self.generic_definition
    }

    /// Returns whether the trait declaration supplies a default body.
    ///
    /// # Returns
    ///
    /// Returns `true` when the trait method has a default implementation.
    #[must_use]
    #[inline]
    pub const fn has_default(&self) -> bool {
        self.has_default
    }

    /// Returns the owning trait definition for a trait method.
    ///
    /// `None` means this method is declared by an impl definition.
    ///
    /// # Returns
    ///
    /// Returns the owning trait declaration, or `None` for an impl method.
    #[must_use]
    #[inline]
    pub const fn declaring_trait(&self) -> Option<&'static TraitDefinitionDescriptor> {
        match self.declaration_owner {
            MethodDeclarationOwner::Trait(descriptor) => Some(descriptor),
            MethodDeclarationOwner::Impl(_) => None,
        }
    }

    /// Returns the owning impl definition for an implementation method.
    ///
    /// `None` means this method is declared by a trait definition.
    ///
    /// # Returns
    ///
    /// Returns the owning impl declaration, or `None` for a trait method.
    #[must_use]
    #[inline]
    pub const fn declaring_impl(&self) -> Option<&'static ImplDefinitionDescriptor> {
        match self.declaration_owner {
            MethodDeclarationOwner::Trait(_) => None,
            MethodDeclarationOwner::Impl(descriptor) => Some(descriptor),
        }
    }

    /// Applies concrete trait arguments to every signature relationship while
    /// preserving the declaration identity and source metadata.
    ///
    /// # Parameters
    ///
    /// - `substitutions`: Concrete application substitutions to apply.
    ///
    /// # Returns
    ///
    /// Returns a cloned method descriptor with substituted signature facts.
    #[must_use]
    pub(crate) fn substituted_for_trait_application(&self, substitutions: &TraitApplicationSubstitutions) -> Self {
        let mut result = self.clone();
        for parameter in &mut result.parameters {
            parameter.signature_type = substitutions.type_expression(&parameter.signature_type);
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
    ///
    /// # Parameters
    ///
    /// - `substitutions`: Concrete application substitutions to compare.
    ///
    /// # Returns
    ///
    /// Returns `true` when at least one signature or predicate changes.
    #[must_use]
    pub(crate) fn needs_trait_application_substitution(&self, substitutions: &TraitApplicationSubstitutions) -> bool {
        self.parameters
            .iter()
            .any(|parameter| substitutions.type_expression(&parameter.signature_type) != parameter.signature_type)
            || self
                .return_value
                .signature_type
                .as_ref()
                .is_some_and(|expression| substitutions.type_expression(expression) != *expression)
            || self
                .generic_definition
                .predicates
                .iter()
                .any(|predicate| substitutions.predicate(predicate) != *predicate)
    }
}
