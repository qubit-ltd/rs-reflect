// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Public macros for explicit concrete reflection registrations.

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
/// Third-party typed operations use the bracket form, where each key expression
/// must match its adapter expression's Rust type:
///
/// ```
/// use qubit_reflect::capability::{CapabilityKey, TypeCapabilities};
/// use qubit_reflect::identity::CapabilityId;
/// use qubit_reflect::register_type_capabilities;
///
/// #[derive(Clone, Copy)]
/// struct Adapter;
///
/// fn key() -> CapabilityKey<Adapter> {
///     CapabilityKey::new(CapabilityId::new("example.adapter").expect("valid test ID"))
/// }
///
/// register_type_capabilities!(String: [key() => Adapter]);
///
/// fn main() {
///     let _ = TypeCapabilities::default();
/// }
/// ```
///
/// ```compile_fail
/// use std::rc::Rc;
/// use qubit_reflect::register_type_capabilities;
///
/// struct LocalOnly(Rc<()>);
/// register_type_capabilities!(LocalOnly: Send);
/// register_type_capabilities!(LocalOnly: Sync);
/// register_type_capabilities!(LocalOnly: Clone);
/// struct NoDefault;
/// register_type_capabilities!(NoDefault: Default);
/// ```
#[macro_export]
macro_rules! register_type_capabilities {
    ($target:ty: [$($key:expr => $adapter:expr),+ $(,)?]) => {
        const _: () = {
            fn __qubit_reflect_target_descriptor() -> &'static $crate::descriptor::TypeDescriptor {
                $crate::descriptor::TypeDescriptor::of::<$target>()
            }

            fn __qubit_reflect_descriptors(
            ) -> ::std::vec::Vec<$crate::capability::CapabilityDescriptor> {
                ::std::vec![
                    $(
                        $crate::capability::CapabilityDescriptor::with_adapter(
                            $key,
                            $adapter,
                        )
                    ),+
                ]
            }

            fn __qubit_reflect_runtime_identity(
            ) -> $crate::__private::codegen_v2::registration::RuntimeIdentity {
                $crate::__private::codegen_v2::registration::RuntimeIdentity::Capabilities(
                    $crate::__private::codegen_v2::registration::CapabilityTarget::Type(
                        __qubit_reflect_target_descriptor().type_id(),
                    ),
                )
            }

            fn __qubit_reflect_payload(
            ) -> $crate::__private::codegen_v2::registration::FragmentPayload {
                $crate::__private::codegen_v2::registration::FragmentPayload::Capability(
                    $crate::__private::codegen_v2::registration::CapabilityRegistration::for_type(
                        __qubit_reflect_target_descriptor(),
                        __qubit_reflect_descriptors(),
                    ),
                )
            }

            $crate::__private::codegen_v2::inventory::submit! {
                $crate::__private::codegen_v2::registration::RegistrationFragment::new(
                    $crate::__private::codegen_v2::registration::FragmentKind::Capability,
                    $crate::__private::codegen_v2::registration::StaticFragmentIdentity::new(
                        env!("CARGO_PKG_NAME"), module_path!(), line!(), column!(), "capability", 0,
                    ),
                    __qubit_reflect_runtime_identity,
                    __qubit_reflect_payload,
                )
            }
        };
    };
    ($target:ty: $($capability:ident),+ $(,)?) => {
        const _: () = {
            fn __qubit_reflect_target_descriptor() -> &'static $crate::descriptor::TypeDescriptor {
                $crate::descriptor::TypeDescriptor::of::<$target>()
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

            fn __qubit_reflect_runtime_identity(
            ) -> $crate::__private::codegen_v2::registration::RuntimeIdentity {
                $crate::__private::codegen_v2::registration::RuntimeIdentity::Capabilities(
                    $crate::__private::codegen_v2::registration::CapabilityTarget::Type(
                        __qubit_reflect_target_descriptor().type_id(),
                    ),
                )
            }

            fn __qubit_reflect_payload(
            ) -> $crate::__private::codegen_v2::registration::FragmentPayload {
                $crate::__private::codegen_v2::registration::FragmentPayload::Capability(
                    $crate::__private::codegen_v2::registration::CapabilityRegistration::for_type(
                        __qubit_reflect_target_descriptor(),
                        __qubit_reflect_descriptors(),
                    ),
                )
            }

            $crate::__private::codegen_v2::inventory::submit! {
                $crate::__private::codegen_v2::registration::RegistrationFragment::new(
                    $crate::__private::codegen_v2::registration::FragmentKind::Capability,
                    $crate::__private::codegen_v2::registration::StaticFragmentIdentity::new(
                        env!("CARGO_PKG_NAME"), module_path!(), line!(), column!(), "capability", 0,
                    ),
                    __qubit_reflect_runtime_identity,
                    __qubit_reflect_payload,
                )
            }
        };
    };
}

/// Registers an existing [`Reflect`](crate::descriptor::Reflect) descriptor
/// root.
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

            fn __qubit_reflect_runtime_identity() -> $crate::__private::codegen_v2::registration::RuntimeIdentity {
                $crate::__private::codegen_v2::registration::RuntimeIdentity::Type(__qubit_reflect_target_type_id())
            }

            fn __qubit_reflect_payload() -> $crate::__private::codegen_v2::registration::FragmentPayload {
                $crate::__private::codegen_v2::registration::FragmentPayload::Type(__qubit_reflect_descriptor())
            }

            $crate::__private::codegen_v2::inventory::submit! {
                $crate::__private::codegen_v2::registration::RegistrationFragment::new(
                    $crate::__private::codegen_v2::registration::FragmentKind::Type,
                    $crate::__private::codegen_v2::registration::StaticFragmentIdentity::new(
                        env!("CARGO_PKG_NAME"), module_path!(), line!(), column!(), "type", 0,
                    ),
                    __qubit_reflect_runtime_identity,
                    __qubit_reflect_payload,
                )
            }
        };
    };
}
