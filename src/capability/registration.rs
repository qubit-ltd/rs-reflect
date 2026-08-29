//! Link-time records for explicit concrete reflection registrations.

use std::any::TypeId;

use crate::capability::{CapabilityConflict, CapabilityDescriptor, TypeCapabilities};
use crate::descriptor::TypeDescriptor;

/// A link-time fragment declaring capabilities for one exact concrete type.
#[doc(hidden)]
pub struct TypeCapabilityRegistration {
    target_type_id: fn() -> TypeId,
    descriptors: fn() -> Vec<CapabilityDescriptor>,
}

impl TypeCapabilityRegistration {
    /// Creates a macro-generated concrete capability registration fragment.
    #[doc(hidden)]
    pub const fn new(
        target_type_id: fn() -> TypeId,
        descriptors: fn() -> Vec<CapabilityDescriptor>,
    ) -> Self {
        Self {
            target_type_id,
            descriptors,
        }
    }
}

inventory::collect!(TypeCapabilityRegistration);

/// A link-time fragment declaring one reflected concrete descriptor root.
#[doc(hidden)]
pub struct ReflectedTypeRegistration {
    target_type_id: fn() -> TypeId,
    descriptor: fn() -> &'static TypeDescriptor,
}

impl ReflectedTypeRegistration {
    /// Creates a macro-generated reflected concrete type registration fragment.
    #[doc(hidden)]
    pub const fn new(
        target_type_id: fn() -> TypeId,
        descriptor: fn() -> &'static TypeDescriptor,
    ) -> Self {
        Self {
            target_type_id,
            descriptor,
        }
    }
}

inventory::collect!(ReflectedTypeRegistration);

/// Collects all capability fragments registered for exact concrete type `T`.
///
/// Returns [`CapabilityConflict`] when matching fragments claim the same ID.
/// An unregistered type produces an empty set rather than inferred capabilities.
#[doc(hidden)]
pub fn registered_type_capabilities<T: 'static>() -> Result<TypeCapabilities, CapabilityConflict> {
    let target = TypeId::of::<T>();
    let mut descriptors = Vec::new();
    for registration in inventory::iter::<TypeCapabilityRegistration> {
        if (registration.target_type_id)() == target {
            descriptors.extend((registration.descriptors)());
        }
    }
    TypeCapabilities::try_new(descriptors)
}

/// Returns the explicitly registered descriptor for exact concrete type `T`.
///
/// `None` means no linked registration fragment names `T`. Registration only
/// returns the existing [`TypeDescriptor`](crate::descriptor::TypeDescriptor)
/// root and never creates a second root.
#[doc(hidden)]
pub fn registered_reflected_type<T: 'static>() -> Option<&'static TypeDescriptor> {
    let target = TypeId::of::<T>();
    inventory::iter::<ReflectedTypeRegistration>
        .into_iter()
        .find(|registration| (registration.target_type_id)() == target)
        .map(|registration| (registration.descriptor)())
}

/// Converts one supported trait token into a bound-checked descriptor.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_capability_descriptor {
    (Clone, $target:ty) => {
        $crate::capability::clone_descriptor::<$target>()
    };
    (Default, $target:ty) => {
        $crate::capability::default_descriptor::<$target>()
    };
    (Send, $target:ty) => {
        $crate::capability::send_descriptor::<$target>()
    };
    (Sync, $target:ty) => {
        $crate::capability::sync_descriptor::<$target>()
    };
}

/// Registers verified capability facts for one exact concrete Rust type.
///
/// Each listed trait is instantiated through a bound-constrained constructor,
/// so a false declaration is rejected while compiling the macro invocation.
/// The registration records facts only: `Send` and `Sync` never change the
/// erased mode of a dynamic value.
///
/// ```compile_fail
/// use std::rc::Rc;
/// use qubit_reflect::register_type_capabilities;
///
/// struct LocalOnly(Rc<()>);
/// register_type_capabilities!(LocalOnly: Send);
/// ```
#[macro_export]
macro_rules! register_type_capabilities {
    ($target:ty: $($capability:ident),+ $(,)?) => {
        const _: () = {
            fn __qubit_reflect_target_type_id() -> ::std::any::TypeId {
                ::std::any::TypeId::of::<$target>()
            }

            fn __qubit_reflect_descriptors(
            ) -> ::std::vec::Vec<$crate::capability::CapabilityDescriptor> {
                ::std::vec![
                    $(
                        $crate::__qubit_reflect_capability_descriptor!(
                            $capability,
                            $target
                        )
                    ),+
                ]
            }

            $crate::capability::__inventory::submit! {
                $crate::capability::TypeCapabilityRegistration::new(
                    __qubit_reflect_target_type_id,
                    __qubit_reflect_descriptors,
                )
            }
        };
    };
}

/// Registers an existing [`Reflect`](crate::descriptor::Reflect) descriptor root.
///
/// The emitted fragment calls `TypeDescriptor::of` and therefore both verifies
/// the trait bound and preserves the interner's existing root identity.
#[macro_export]
macro_rules! register_reflected_type {
    ($target:ty $(,)?) => {
        const _: () = {
            fn __qubit_reflect_target_type_id() -> ::std::any::TypeId {
                ::std::any::TypeId::of::<$target>()
            }

            fn __qubit_reflect_descriptor() -> &'static $crate::descriptor::TypeDescriptor {
                $crate::descriptor::TypeDescriptor::of::<$target>()
            }

            $crate::capability::__inventory::submit! {
                $crate::capability::ReflectedTypeRegistration::new(
                    __qubit_reflect_target_type_id,
                    __qubit_reflect_descriptor,
                )
            }
        };
    };
}
