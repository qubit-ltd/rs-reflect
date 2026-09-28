// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Static records submitted by generated reflection code.

use std::any::TypeId;

use crate::capability::CapabilityDescriptor;
use crate::descriptor::ImplDefinitionDescriptor;
use crate::descriptor::ImplDescriptor;
use crate::descriptor::TraitDefinitionDescriptor;
use crate::descriptor::TraitId;
use crate::descriptor::TypeDefinitionDescriptor;
use crate::descriptor::TypeDefinitionId;
use crate::descriptor::TypeDescriptor;
use crate::identity::FragmentIdentity;

/// Const-constructible stable source and content facts for one fragment.
///
/// Generated code stores this borrowed form directly in linker inventory. The
/// registry converts it to the owned public [`FragmentIdentity`] only while
/// building diagnostics and audit indexes.
///
/// # Examples
///
/// ```
/// use qubit_reflect::__private::codegen_v3::registration::StaticFragmentIdentity;
///
/// let identity = StaticFragmentIdentity::new("example", "example::settings", 1, 1, "type", 42);
/// assert_eq!(identity, StaticFragmentIdentity::new("example", "example::settings", 1, 1, "type", 42));
/// ```
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StaticFragmentIdentity {
    /// Crate containing the source declaration.
    declaring_crate: &'static str,
    /// Rust module path containing the declaration.
    module_path: &'static str,
    /// Source line of the declaration.
    line: u32,
    /// Source column of the declaration.
    column: u32,
    /// Category of the registered member.
    member_kind: &'static str,
    /// Deterministic fingerprint of normalized declaration content.
    content_fingerprint: u64,
}

impl StaticFragmentIdentity {
    /// Creates static identity facts from macro-provided constants.
    ///
    /// # Parameters
    ///
    /// - `declaring_crate`: Crate containing the source declaration.
    /// - `module_path`: Rust module path containing the declaration.
    /// - `line`: Source line of the declaration.
    /// - `column`: Source column of the declaration.
    /// - `member_kind`: Category of the registered member.
    /// - `content_fingerprint`: Deterministic normalized content fingerprint.
    ///
    /// # Returns
    ///
    /// Returns a borrowed, const-constructible fragment identity.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        declaring_crate: &'static str,
        module_path: &'static str,
        line: u32,
        column: u32,
        member_kind: &'static str,
        content_fingerprint: u64,
    ) -> Self {
        Self {
            declaring_crate,
            module_path,
            line,
            column,
            member_kind,
            content_fingerprint,
        }
    }

    /// Copies borrowed identity facts into the owned public representation.
    ///
    /// # Returns
    ///
    /// Returns an owned identity suitable for diagnostics and indexes.
    #[must_use]
    pub(crate) fn to_owned(self) -> FragmentIdentity {
        FragmentIdentity::new(
            self.declaring_crate,
            self.module_path,
            self.line,
            self.column,
            self.member_kind,
            self.content_fingerprint,
        )
    }
}

/// The payload category of a distributed registration fragment.
///
/// # Examples
///
/// ```
/// use qubit_reflect::__private::codegen_v3::registration::FragmentKind;
///
/// assert_eq!(FragmentKind::Type, FragmentKind::Type);
/// ```
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FragmentKind {
    /// A concrete root type descriptor.
    Type,
    /// A source-level generic type declaration.
    TypeDefinition,
    /// A reflected or external trait definition descriptor.
    Trait,
    /// A generic, blanket, constrained, or concrete impl definition.
    ImplDefinition,
    /// A concrete inherent or trait implementation descriptor.
    Impl,
    /// One capability fact for an exact concrete type.
    Capability,
}

/// The process-local target claimed by a registration fragment.
///
/// # Examples
///
/// ```
/// use std::any::TypeId;
/// use qubit_reflect::__private::codegen_v3::registration::RuntimeIdentity;
///
/// let target = RuntimeIdentity::Type(TypeId::of::<u32>());
/// assert!(matches!(target, RuntimeIdentity::Type(id) if id == TypeId::of::<u32>()));
/// ```
#[doc(hidden)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeIdentity {
    /// An exact concrete Rust type.
    Type(TypeId),
    /// A source-level generic type declaration.
    TypeDefinition(TypeDefinitionId),
    /// A reflected marker or stable external trait identity.
    Trait(TraitId),
    /// An impl declaration identified by its stable source fragment.
    ImplDefinition(FragmentIdentity),
    /// A concrete implementation target.
    Impl(TypeId),
    /// Capability facts attached to an exact concrete type.
    Capabilities(CapabilityTarget),
}

/// The process-local target of one capability registration.
///
/// # Examples
///
/// ```
/// use std::any::TypeId;
/// use qubit_reflect::registry::CapabilityTarget;
///
/// let target = CapabilityTarget::Type(TypeId::of::<u32>());
/// assert!(matches!(target, CapabilityTarget::Type(id) if id == TypeId::of::<u32>()));
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CapabilityTarget {
    /// An exact concrete reflected type.
    Type(TypeId),
    /// A source-level generic type declaration.
    TypeDefinition(TypeDefinitionId),
}

/// A descriptor provider retained by a capability registration.
#[derive(Clone, Copy, Debug)]
enum CapabilityRegistrationTarget {
    /// A registration tied to a reflected root descriptor.
    Type(&'static TypeDescriptor),
    /// A benchmark registration tied only to an exact type ID.
    TypeId(TypeId),
    /// A registration tied to a generic type declaration.
    TypeDefinition(&'static TypeDefinitionDescriptor),
}

/// One capability payload contributed by generated registration code.
///
/// # Examples
///
/// ```
/// use qubit_reflect::__private::codegen_v3::registration::CapabilityRegistration;
/// use qubit_reflect::TypeDescriptor;
///
/// let registration = CapabilityRegistration::for_type(TypeDescriptor::of::<u32>(), vec![]);
/// assert_eq!(registration.target(), qubit_reflect::registry::CapabilityTarget::Type(std::any::TypeId::of::<u32>()));
/// ```
#[doc(hidden)]
#[derive(Debug)]
pub struct CapabilityRegistration {
    /// Concrete or generic target carrying this registration.
    target: CapabilityRegistrationTarget,
    /// Capability facts contributed by the fragment.
    descriptors: Vec<CapabilityDescriptor>,
}

impl CapabilityRegistration {
    /// Creates a capability payload for `target_type_id`.
    ///
    /// # Parameters
    ///
    /// - `target`: Reflected root carrying the capability facts.
    /// - `descriptors`: Capability facts contributed for that root.
    ///
    /// # Returns
    ///
    /// Returns the registration payload.
    #[doc(hidden)]
    #[must_use]
    pub const fn for_type(
        target: &'static TypeDescriptor,
        descriptors: Vec<CapabilityDescriptor>,
    ) -> Self {
        Self {
            target: CapabilityRegistrationTarget::Type(target),
            descriptors,
        }
    }

    /// Creates a capability payload for one generic type declaration.
    ///
    /// # Parameters
    ///
    /// - `target`: Generic declaration carrying the capability facts.
    /// - `descriptors`: Capability facts contributed for that declaration.
    ///
    /// # Returns
    ///
    /// Returns the registration payload.
    #[doc(hidden)]
    #[must_use]
    pub const fn for_definition(
        target: &'static TypeDefinitionDescriptor,
        descriptors: Vec<CapabilityDescriptor>,
    ) -> Self {
        Self {
            target: CapabilityRegistrationTarget::TypeDefinition(target),
            descriptors,
        }
    }

    /// Creates a benchmark-only payload when no reflected descriptor exists.
    ///
    /// # Parameters
    ///
    /// - `target`: Exact process-local Rust type identity.
    /// - `descriptors`: Capability facts associated with the type.
    ///
    /// # Returns
    ///
    /// Returns a registration payload for benchmark fixtures.
    #[doc(hidden)]
    #[must_use]
    pub const fn for_type_id(target: TypeId, descriptors: Vec<CapabilityDescriptor>) -> Self {
        Self {
            target: CapabilityRegistrationTarget::TypeId(target),
            descriptors,
        }
    }

    /// Returns the process-local target identity.
    ///
    /// # Returns
    ///
    /// Returns the concrete type or generic declaration identity.
    #[must_use]
    #[inline]
    pub fn target(&self) -> CapabilityTarget {
        match self.target {
            CapabilityRegistrationTarget::Type(descriptor) => {
                CapabilityTarget::Type(descriptor.type_id())
            }
            CapabilityRegistrationTarget::TypeId(type_id) => CapabilityTarget::Type(type_id),
            CapabilityRegistrationTarget::TypeDefinition(descriptor) => {
                CapabilityTarget::TypeDefinition(descriptor.id())
            }
        }
    }

    /// Returns the concrete target descriptor when this registration targets a
    /// type.
    ///
    /// # Returns
    ///
    /// Returns the root descriptor for concrete type registrations, or `None`
    /// for type-ID and generic-definition registrations.
    #[must_use]
    pub(crate) const fn type_descriptor(&self) -> Option<&'static TypeDescriptor> {
        match self.target {
            CapabilityRegistrationTarget::Type(descriptor) => Some(descriptor),
            CapabilityRegistrationTarget::TypeId(_)
            | CapabilityRegistrationTarget::TypeDefinition(_) => None,
        }
    }

    /// Returns the immutable capability descriptors.
    ///
    /// # Returns
    ///
    /// Returns the capability facts contributed by this registration.
    #[must_use]
    #[inline]
    pub(crate) fn descriptors(&self) -> &[CapabilityDescriptor] {
        &self.descriptors
    }
}

/// Materialized data returned by a static registration fragment.
///
/// # Examples
///
/// ```
/// use qubit_reflect::__private::codegen_v3::registration::FragmentPayload;
/// use qubit_reflect::TypeDescriptor;
///
/// let payload = FragmentPayload::Type(TypeDescriptor::of::<u32>());
/// assert!(matches!(payload, FragmentPayload::Type(_)));
/// ```
#[doc(hidden)]
#[derive(Debug)]
pub enum FragmentPayload {
    /// One concrete root type descriptor.
    Type(&'static TypeDescriptor),
    /// One source-level generic type declaration.
    TypeDefinition(&'static TypeDefinitionDescriptor),
    /// One reflected or external trait definition descriptor.
    Trait(&'static TraitDefinitionDescriptor),
    /// One impl declaration descriptor without a concrete target instance.
    ImplDefinition(&'static ImplDefinitionDescriptor),
    /// One concrete implementation descriptor.
    Impl(&'static ImplDescriptor),
    /// One capability fact for an exact concrete type.
    Capability(CapabilityRegistration),
}

impl FragmentPayload {
    /// Returns the payload category used to validate its static declaration.
    ///
    /// # Returns
    ///
    /// Returns the stable category corresponding to this payload variant.
    #[must_use]
    #[inline]
    pub(crate) const fn kind(&self) -> FragmentKind {
        match self {
            Self::Type(_) => FragmentKind::Type,
            Self::TypeDefinition(_) => FragmentKind::TypeDefinition,
            Self::Trait(_) => FragmentKind::Trait,
            Self::ImplDefinition(_) => FragmentKind::ImplDefinition,
            Self::Impl(_) => FragmentKind::Impl,
            Self::Capability(_) => FragmentKind::Capability,
        }
    }

    /// Computes the process-local target represented by this payload.
    ///
    /// # Returns
    ///
    /// Returns the runtime identity used to verify the static declaration.
    #[must_use]
    pub(crate) fn runtime_identity(&self) -> RuntimeIdentity {
        match self {
            Self::Type(descriptor) => RuntimeIdentity::Type(descriptor.type_id()),
            Self::TypeDefinition(descriptor) => RuntimeIdentity::TypeDefinition(descriptor.id()),
            Self::Trait(descriptor) => RuntimeIdentity::Trait(descriptor.trait_id().clone()),
            Self::ImplDefinition(descriptor) => {
                RuntimeIdentity::ImplDefinition(descriptor.fragment_identity().clone())
            }
            Self::Impl(descriptor) => RuntimeIdentity::Impl(descriptor.target_type().type_id()),
            Self::Capability(registration) => RuntimeIdentity::Capabilities(registration.target()),
        }
    }
}

/// An immutable linker-discovered reflection registration record.
///
/// The record contains only static identity facts, function pointers, and an
/// enum tag. Calling the functions is deferred until registry initialization,
/// so linker discovery never executes generated or user code.
///
/// # Examples
///
/// ```
/// use qubit_reflect::__private::codegen_v3::registration::{FragmentKind, FragmentPayload, RegistrationFragment, RuntimeIdentity, StaticFragmentIdentity};
/// use qubit_reflect::TypeDescriptor;
///
/// fn target() -> RuntimeIdentity {
///     RuntimeIdentity::Type(std::any::TypeId::of::<u32>())
/// }
/// fn payload() -> FragmentPayload {
///     FragmentPayload::Type(TypeDescriptor::of::<u32>())
/// }
/// let fragment = RegistrationFragment::new(
///     FragmentKind::Type,
///     StaticFragmentIdentity::new("example", "example", 1, 1, "type", 1),
///     target,
///     payload,
/// );
/// let _ = fragment;
/// ```
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct RegistrationFragment {
    /// Payload category declared in the static inventory record.
    kind: FragmentKind,
    /// Borrowed source and content facts for this fragment.
    identity: StaticFragmentIdentity,
    /// Deferred function that determines the process-local target.
    target_identity: fn() -> RuntimeIdentity,
    /// Deferred function that materializes the fragment payload.
    build: fn() -> FragmentPayload,
}

impl RegistrationFragment {
    /// Creates a static fragment from generated factories.
    ///
    /// # Parameters
    ///
    /// - `kind`: Declared category of the registration payload.
    /// - `identity`: Borrowed source and content identity.
    /// - `target_identity`: Deferred factory for the process-local target.
    /// - `build`: Deferred factory for the immutable payload.
    ///
    /// # Returns
    ///
    /// Returns a static linker-discovered fragment record.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        kind: FragmentKind,
        identity: StaticFragmentIdentity,
        target_identity: fn() -> RuntimeIdentity,
        build: fn() -> FragmentPayload,
    ) -> Self {
        Self {
            kind,
            identity,
            target_identity,
            build,
        }
    }

    /// Returns the statically declared payload category.
    #[must_use]
    #[inline]
    pub(crate) const fn kind(&self) -> FragmentKind {
        self.kind
    }

    /// Materializes the stable source and content identity.
    #[must_use]
    pub(crate) fn identity(&self) -> FragmentIdentity {
        self.identity.to_owned()
    }

    /// Materializes the process-local target identity.
    #[must_use]
    pub(crate) fn target_identity(&self) -> RuntimeIdentity {
        (self.target_identity)()
    }

    /// Builds the immutable payload during registry initialization.
    #[must_use]
    pub(crate) fn build(&self) -> FragmentPayload {
        (self.build)()
    }
}

inventory::collect!(RegistrationFragment);

macro_rules! register_builtin_type {
    ($module:ident, $type:ty, $fingerprint:expr) => {
        mod $module {
            use super::FragmentKind;
            use super::FragmentPayload;
            use super::RegistrationFragment;
            use super::RuntimeIdentity;
            use super::StaticFragmentIdentity;
            use super::TypeDescriptor;
            use super::TypeId;

            /// Returns the exact process-local built-in type identity.
            ///
            /// # Returns
            ///
            /// Returns the runtime identity for the registered built-in type.
            #[must_use]
            fn runtime_identity() -> RuntimeIdentity {
                RuntimeIdentity::Type(TypeId::of::<$type>())
            }

            /// Returns the existing unique built-in descriptor root.
            ///
            /// # Returns
            ///
            /// Returns the canonical descriptor for the registered type.
            #[must_use]
            fn payload() -> FragmentPayload {
                FragmentPayload::Type(TypeDescriptor::of::<$type>())
            }

            inventory::submit! {
                RegistrationFragment::new(
                    FragmentKind::Type,
                    StaticFragmentIdentity::new(
                        env!("CARGO_PKG_NAME"),
                        module_path!(),
                        0,
                        0,
                        "type",
                        $fingerprint,
                    ),
                    runtime_identity,
                    payload,
                )
            }
        }
    };
}

register_builtin_type!(bool_registration, bool, 1);
register_builtin_type!(char_registration, char, 2);
register_builtin_type!(i8_registration, i8, 3);
register_builtin_type!(i16_registration, i16, 4);
register_builtin_type!(i32_registration, i32, 5);
register_builtin_type!(i64_registration, i64, 6);
register_builtin_type!(i128_registration, i128, 7);
register_builtin_type!(isize_registration, isize, 8);
register_builtin_type!(u8_registration, u8, 9);
register_builtin_type!(u16_registration, u16, 10);
register_builtin_type!(u32_registration, u32, 11);
register_builtin_type!(u64_registration, u64, 12);
register_builtin_type!(u128_registration, u128, 13);
register_builtin_type!(usize_registration, usize, 14);
register_builtin_type!(f32_registration, f32, 15);
register_builtin_type!(f64_registration, f64, 16);
register_builtin_type!(string_registration, String, 17);
register_builtin_type!(str_registration, str, 18);
register_builtin_type!(unit_tuple_registration, (), 19);
register_builtin_type!(debug_trait_object_registration, dyn std::fmt::Debug, 20);
