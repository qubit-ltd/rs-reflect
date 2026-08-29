//! Static records submitted by generated reflection code.

use std::any::TypeId;

use crate::capability::CapabilityDescriptor;
use crate::descriptor::{ImplDescriptor, TraitDefinitionDescriptor, TraitId, TypeDescriptor};
use crate::identity::{CapabilityId, FragmentIdentity};

/// Const-constructible stable source and content facts for one fragment.
///
/// Generated code stores this borrowed form directly in linker inventory. The
/// registry converts it to the owned public [`FragmentIdentity`] only while
/// building diagnostics and audit indexes.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StaticFragmentIdentity {
    declaring_crate: &'static str,
    module_path: &'static str,
    line: u32,
    column: u32,
    member_kind: &'static str,
    content_fingerprint: u64,
}

impl StaticFragmentIdentity {
    /// Creates static identity facts from macro-provided constants.
    #[doc(hidden)]
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
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FragmentKind {
    /// A concrete root type descriptor.
    Type,
    /// A reflected or external trait definition descriptor.
    Trait,
    /// A concrete inherent or trait implementation descriptor.
    Impl,
    /// One capability fact for an exact concrete type.
    Capability,
}

/// The process-local target claimed by a registration fragment.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeIdentity {
    /// An exact concrete Rust type.
    Type(TypeId),
    /// A reflected marker or stable external trait identity.
    Trait(TraitId),
    /// A concrete implementation target.
    Impl(TypeId),
    /// A capability ID attached to an exact concrete type.
    Capability {
        /// The exact concrete target type.
        target_type_id: TypeId,
        /// The stable capability identity.
        capability_id: CapabilityId,
    },
}

/// One capability payload contributed by generated registration code.
#[doc(hidden)]
#[derive(Debug)]
pub struct CapabilityRegistration {
    target_type_id: TypeId,
    descriptor: CapabilityDescriptor,
}

impl CapabilityRegistration {
    /// Creates a capability payload for `target_type_id`.
    #[doc(hidden)]
    pub const fn new(target_type_id: TypeId, descriptor: CapabilityDescriptor) -> Self {
        Self {
            target_type_id,
            descriptor,
        }
    }

    /// Returns the exact concrete target type.
    pub(crate) const fn target_type_id(&self) -> TypeId {
        self.target_type_id
    }

    /// Returns the immutable capability descriptor.
    pub(crate) const fn descriptor(&self) -> &CapabilityDescriptor {
        &self.descriptor
    }
}

/// Materialized data returned by a static registration fragment.
#[doc(hidden)]
#[derive(Debug)]
pub enum FragmentPayload {
    /// One concrete root type descriptor.
    Type(&'static TypeDescriptor),
    /// One reflected or external trait definition descriptor.
    Trait(&'static TraitDefinitionDescriptor),
    /// One concrete implementation descriptor.
    Impl(&'static ImplDescriptor),
    /// One capability fact for an exact concrete type.
    Capability(CapabilityRegistration),
}

impl FragmentPayload {
    /// Returns the payload category used to validate its static declaration.
    pub(crate) const fn kind(&self) -> FragmentKind {
        match self {
            Self::Type(_) => FragmentKind::Type,
            Self::Trait(_) => FragmentKind::Trait,
            Self::Impl(_) => FragmentKind::Impl,
            Self::Capability(_) => FragmentKind::Capability,
        }
    }

    /// Computes the process-local target represented by this payload.
    pub(crate) fn runtime_identity(&self) -> RuntimeIdentity {
        match self {
            Self::Type(descriptor) => RuntimeIdentity::Type(descriptor.type_id()),
            Self::Trait(descriptor) => RuntimeIdentity::Trait(descriptor.trait_id().clone()),
            Self::Impl(descriptor) => RuntimeIdentity::Impl(descriptor.target_type().type_id()),
            Self::Capability(registration) => RuntimeIdentity::Capability {
                target_type_id: registration.target_type_id(),
                capability_id: registration.descriptor().id().clone(),
            },
        }
    }
}

/// An immutable linker-discovered reflection registration record.
///
/// The record contains only static identity facts, function pointers, and an
/// enum tag. Calling the functions is deferred until registry initialization,
/// so linker discovery never executes generated or user code.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct RegistrationFragment {
    kind: FragmentKind,
    identity: StaticFragmentIdentity,
    target_identity: fn() -> RuntimeIdentity,
    build: fn() -> FragmentPayload,
}

impl RegistrationFragment {
    /// Creates a static fragment from generated factories.
    #[doc(hidden)]
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
    pub(crate) const fn kind(&self) -> FragmentKind {
        self.kind
    }

    /// Materializes the stable source and content identity.
    pub(crate) fn identity(&self) -> FragmentIdentity {
        self.identity.to_owned()
    }

    /// Materializes the process-local target identity.
    pub(crate) fn target_identity(&self) -> RuntimeIdentity {
        (self.target_identity)()
    }

    /// Builds the immutable payload during registry initialization.
    pub(crate) fn build(&self) -> FragmentPayload {
        (self.build)()
    }
}

inventory::collect!(RegistrationFragment);

macro_rules! register_builtin_type {
    ($module:ident, $type:ty, $fingerprint:expr) => {
        mod $module {
            use super::{
                FragmentKind, FragmentPayload, RegistrationFragment, RuntimeIdentity,
                StaticFragmentIdentity, TypeDescriptor, TypeId,
            };

            /// Returns the exact process-local built-in type identity.
            fn runtime_identity() -> RuntimeIdentity {
                RuntimeIdentity::Type(TypeId::of::<$type>())
            }

            /// Returns the existing unique built-in descriptor root.
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
