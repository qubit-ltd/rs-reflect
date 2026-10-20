// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// =============================================================================

//! Typed registrations for built-in runtime capabilities.

pub use crate::runtime::capability::builtin::CloneCapability;
pub use crate::runtime::capability::builtin::DefaultCapability;
pub use crate::runtime::capability::builtin::SendCapability;
pub use crate::runtime::capability::builtin::SyncCapability;
use crate::runtime::capability::capability_registration::CapabilityRegistration;
use crate::runtime::capability::capability_registration_origin::CapabilityRegistrationOrigin;
use crate::runtime::context::handle::registration::Registration;
use crate::runtime::context::handle::registration::RegistrationFragment;
use crate::runtime::identity::RegistrationOrigin;

fn linked_origin(
    crate_name: &'static str,
    crate_version: &'static str,
    module_path: &'static str,
    file: &'static str,
    line: u32,
) -> CapabilityRegistrationOrigin {
    CapabilityRegistrationOrigin::Linked(RegistrationOrigin {
        crate_name,
        crate_version: Some(crate_version),
        module_path,
        file: Some(file),
        line: Some(line),
        column: None,
    })
}

/// Creates a linked capability registration for `T: Send`.
#[doc(hidden)]
pub fn register_send<T: crate::runtime::descriptor::Reflect + Send + 'static>(
    crate_name: &'static str,
    crate_version: &'static str,
    module_path: &'static str,
    file: &'static str,
    line: u32,
) -> Registration {
    let capability =
        CapabilityRegistration::send::<T>(linked_origin(crate_name, crate_version, module_path, file, line))
            .expect("the built-in Send capability ID is valid");
    Registration::metadata_only().with_capability(capability)
}

/// Creates a linked capability registration for `T: Sync`.
#[doc(hidden)]
pub fn register_sync<T: crate::runtime::descriptor::Reflect + Sync + 'static>(
    crate_name: &'static str,
    crate_version: &'static str,
    module_path: &'static str,
    file: &'static str,
    line: u32,
) -> Registration {
    let capability =
        CapabilityRegistration::sync::<T>(linked_origin(crate_name, crate_version, module_path, file, line))
            .expect("the built-in Sync capability ID is valid");
    Registration::metadata_only().with_capability(capability)
}

/// Creates a linked local-mode Clone capability registration for `T`.
#[doc(hidden)]
pub fn register_clone<T: crate::runtime::descriptor::Reflect + std::clone::Clone + 'static>(
    crate_name: &'static str,
    crate_version: &'static str,
    module_path: &'static str,
    file: &'static str,
    line: u32,
) -> Registration {
    let capability =
        CapabilityRegistration::clone::<T>(linked_origin(crate_name, crate_version, module_path, file, line))
            .expect("the built-in Clone capability ID is valid");
    Registration::metadata_only().with_capability(capability)
}

/// Creates a linked Clone capability registration with Local and ThreadSafe
/// adapters.
#[doc(hidden)]
pub fn register_thread_safe_clone<
    T: crate::runtime::descriptor::Reflect
        + std::clone::Clone
        + crate::runtime::value::ValueFor<crate::runtime::value::ThreadSafe>
        + Send
        + Sync,
>(
    crate_name: &'static str,
    crate_version: &'static str,
    module_path: &'static str,
    file: &'static str,
    line: u32,
) -> Registration {
    let capability = CapabilityRegistration::clone_thread_safe::<T>(linked_origin(
        crate_name,
        crate_version,
        module_path,
        file,
        line,
    ))
    .expect("the built-in Clone capability ID is valid");
    Registration::metadata_only().with_capability(capability)
}

/// Creates a linked local-mode Default capability registration for `T`.
#[doc(hidden)]
pub fn register_default<T: crate::runtime::descriptor::Reflect + std::default::Default + 'static>(
    crate_name: &'static str,
    crate_version: &'static str,
    module_path: &'static str,
    file: &'static str,
    line: u32,
) -> Registration {
    let capability =
        CapabilityRegistration::default::<T>(linked_origin(crate_name, crate_version, module_path, file, line))
            .expect("the built-in Default capability ID is valid");
    Registration::metadata_only().with_capability(capability)
}

/// Creates a linked Default capability registration with Local and ThreadSafe
/// adapters.
#[doc(hidden)]
pub fn register_thread_safe_default<
    T: crate::runtime::descriptor::Reflect
        + std::default::Default
        + crate::runtime::value::ValueFor<crate::runtime::value::ThreadSafe>
        + Send
        + Sync,
>(
    crate_name: &'static str,
    crate_version: &'static str,
    module_path: &'static str,
    file: &'static str,
    line: u32,
) -> Registration {
    let capability = CapabilityRegistration::default_thread_safe::<T>(linked_origin(
        crate_name,
        crate_version,
        module_path,
        file,
        line,
    ))
    .expect("the built-in Default capability ID is valid");
    Registration::metadata_only().with_capability(capability)
}

fn no_dependencies() -> &'static [std::any::TypeId] {
    &[]
}

#[doc(hidden)]
pub const fn capability_fragment<T: 'static>(
    crate_name: &'static str,
    crate_version: &'static str,
    module_path: &'static str,
    file: &'static str,
    line: u32,
    label: &'static str,
    registration: fn() -> Registration,
) -> RegistrationFragment {
    RegistrationFragment::for_target::<T>(
        (
            crate_name,
            Some(crate_version),
            module_path,
            Some(file),
            Some(line),
            None,
            label,
        ),
        no_dependencies,
        registration,
    )
}

/// Submits a typed built-in capability registration for `T`.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_register_capability {
    ($ty:ty: SendCapability) => {
        $crate::__qubit_reflect_register_capability!($ty: Send);
    };
    ($ty:ty: SyncCapability) => {
        $crate::__qubit_reflect_register_capability!($ty: Sync);
    };
    ($ty:ty: CloneCapability) => {
        $crate::__qubit_reflect_register_capability!($ty: Clone);
    };
    ($ty:ty: DefaultCapability) => {
        $crate::__qubit_reflect_register_capability!($ty: Default);
    };
    ($ty:ty: Send) => {
        const _: () = {
            fn registration() -> $crate::__private::codegen_v4::registration::Registration {
                $crate::__private::codegen_v4::capability::register_send::<$ty>(
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                    module_path!(),
                    file!(),
                    line!(),
                )
            }
            $crate::__private::__inventory::submit! {
                $crate::__private::codegen_v4::capability::capability_fragment::<$ty>(
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                    module_path!(),
                    file!(),
                    line!(),
                    concat!(stringify!($ty), "::capability::Send"),
                    registration,
                )
            }
        };
    };
    ($ty:ty: Sync) => {
        const _: () = {
            fn registration() -> $crate::__private::codegen_v4::registration::Registration {
                $crate::__private::codegen_v4::capability::register_sync::<$ty>(
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                    module_path!(),
                    file!(),
                    line!(),
                )
            }
            $crate::__private::__inventory::submit! {
                $crate::__private::codegen_v4::capability::capability_fragment::<$ty>(
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                    module_path!(),
                    file!(),
                    line!(),
                    concat!(stringify!($ty), "::capability::Sync"),
                    registration,
                )
            }
        };
    };
    ($ty:ty: Clone) => {
        const _: () = {
            fn registration() -> $crate::__private::codegen_v4::registration::Registration {
                $crate::__private::codegen_v4::capability::register_clone::<$ty>(
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                    module_path!(),
                    file!(),
                    line!(),
                )
            }
            $crate::__private::__inventory::submit! {
                $crate::__private::codegen_v4::capability::capability_fragment::<$ty>(
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                    module_path!(),
                    file!(),
                    line!(),
                    concat!(stringify!($ty), "::capability::Clone"),
                    registration,
                )
            }
        };
    };
    ($ty:ty: Clone(ThreadSafe)) => {
        const _: () = {
            fn registration() -> $crate::__private::codegen_v4::registration::Registration {
                $crate::__private::codegen_v4::capability::register_thread_safe_clone::<$ty>(
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                    module_path!(),
                    file!(),
                    line!(),
                )
            }
            $crate::__private::__inventory::submit! {
                $crate::__private::codegen_v4::capability::capability_fragment::<$ty>(
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                    module_path!(),
                    file!(),
                    line!(),
                    concat!(stringify!($ty), "::capability::Clone(ThreadSafe)"),
                    registration,
                )
            }
        };
    };
    ($ty:ty: Default) => {
        const _: () = {
            fn registration() -> $crate::__private::codegen_v4::registration::Registration {
                $crate::__private::codegen_v4::capability::register_default::<$ty>(
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                    module_path!(),
                    file!(),
                    line!(),
                )
            }
            $crate::__private::__inventory::submit! {
                $crate::__private::codegen_v4::capability::capability_fragment::<$ty>(
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                    module_path!(),
                    file!(),
                    line!(),
                    concat!(stringify!($ty), "::capability::Default"),
                    registration,
                )
            }
        };
    };
    ($ty:ty: Default(ThreadSafe)) => {
        const _: () = {
            fn registration() -> $crate::__private::codegen_v4::registration::Registration {
                $crate::__private::codegen_v4::capability::register_thread_safe_default::<$ty>(
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                    module_path!(),
                    file!(),
                    line!(),
                )
            }
            $crate::__private::__inventory::submit! {
                $crate::__private::codegen_v4::capability::capability_fragment::<$ty>(
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                    module_path!(),
                    file!(),
                    line!(),
                    concat!(stringify!($ty), "::capability::Default(ThreadSafe)"),
                    registration,
                )
            }
        };
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_register_capability as register_capability;

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use crate::runtime::capability::CapabilityError;
    use crate::runtime::capability::CapabilityInvocationError;
    use crate::runtime::capability::CapabilityOrigin;
    use crate::runtime::capability::CloneCapability;
    use crate::runtime::capability::DefaultCapability;
    use crate::runtime::capability::SendCapability;
    use crate::runtime::capability::SyncCapability;
    use crate::runtime::capability::capability_error::CapabilityInvocationMode;
    use crate::runtime::context::ReflectionContext;
    use crate::runtime::context::handle::registration::Registration;
    use crate::runtime::descriptor::Reflect;
    use crate::runtime::descriptor::TypeRegistration;
    use crate::runtime::value::DynamicRef;
    use crate::runtime::value::Local;
    use crate::runtime::value::ThreadSafe;

    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    struct CapabilityFixture(u32);

    impl Reflect for CapabilityFixture {
        fn __reflect_registration() -> &'static TypeRegistration<Self> {
            static REGISTRATION: TypeRegistration<CapabilityFixture> = TypeRegistration::new();
            &REGISTRATION
        }
    }

    fn assert_linked_origin<C: crate::runtime::capability::CapabilitySpec>(context: &ReflectionContext) {
        let capability = context
            .type_of::<CapabilityFixture>()
            .capability::<C>()
            .expect("the capability registration is present");
        let CapabilityOrigin::Linked { registration } = capability.origin() else {
            panic!("generated capability registrations have linked provenance");
        };
        let origin = registration.registration();
        assert_eq!(origin.crate_name, "capability-tests");
        assert_eq!(origin.crate_version, Some("1.2.3"));
        assert_eq!(origin.module_path, "codegen_v4::capability_tests");
        assert_eq!(origin.file, Some("capability.rs"));
        assert_eq!(origin.line, Some(41));
        assert_eq!(origin.column, None);
    }

    fn linked_context(registrations: impl IntoIterator<Item = Registration>) -> ReflectionContext {
        let builder = ReflectionContext::builder().include_type::<CapabilityFixture>();
        registrations
            .into_iter()
            .fold(builder, |builder, registration| builder.register(registration))
            .build()
            .expect("linked capability metadata builds a context")
    }

    fn source() -> Registration {
        super::register_send::<CapabilityFixture>(
            "capability-tests",
            "1.2.3",
            "codegen_v4::capability_tests",
            "capability.rs",
            41,
        )
    }

    #[test]
    fn linked_origin_and_fragment_bind_registration_to_the_typed_target() {
        let fragment = super::capability_fragment::<CapabilityFixture>(
            "capability-tests",
            "1.2.3",
            "codegen_v4::capability_tests",
            "capability.rs",
            41,
            "CapabilityFixture::capability::Send",
            source,
        );

        assert_eq!(fragment.target_type_id(), Some(TypeId::of::<CapabilityFixture>()));
        assert!(fragment.dependencies().is_empty());
        assert!(!fragment.includes_type());
        assert!(!fragment.include_in_linked_context());
        assert_eq!(fragment.sort_key().6, "CapabilityFixture::capability::Send");

        let context = linked_context([fragment.materialize().expect("fragment target is valid")]);
        assert_linked_origin::<SendCapability>(&context);
    }

    #[test]
    fn registration_functions_cover_marker_and_local_adapter_capabilities() {
        let context = linked_context([
            super::register_send::<CapabilityFixture>("capability-tests", "1.2.3", "tests", "caps.rs", 1),
            super::register_sync::<CapabilityFixture>("capability-tests", "1.2.3", "tests", "caps.rs", 2),
            super::register_clone::<CapabilityFixture>("capability-tests", "1.2.3", "tests", "caps.rs", 3),
            super::register_default::<CapabilityFixture>("capability-tests", "1.2.3", "tests", "caps.rs", 4),
        ]);
        let descriptor = context.type_of::<CapabilityFixture>();

        assert!(descriptor.has_capability::<SendCapability>().unwrap());
        assert!(descriptor.has_capability::<SyncCapability>().unwrap());
        assert!(descriptor.has_capability::<CloneCapability>().unwrap());
        assert!(descriptor.has_capability::<DefaultCapability>().unwrap());

        let clone = descriptor.capability::<CloneCapability>().unwrap();
        let cloned = clone
            .adapter()
            .clone_dynamic::<Local>(DynamicRef::new(&CapabilityFixture(7)))
            .unwrap()
            .downcast::<CapabilityFixture>()
            .unwrap();
        assert_eq!(cloned, CapabilityFixture(7));
        assert!(matches!(
            clone
                .adapter()
                .clone_dynamic::<ThreadSafe>(DynamicRef::new(&CapabilityFixture(7))),
            Err(CapabilityInvocationError::UnsupportedMode {
                expected_type,
                mode: CapabilityInvocationMode::ThreadSafe,
            }) if expected_type == TypeId::of::<CapabilityFixture>()
        ));
        assert!(matches!(
            clone.adapter().clone_dynamic::<Local>(DynamicRef::new(&19_u32)),
            Err(CapabilityInvocationError::TypeMismatch { expected, actual })
                if expected == TypeId::of::<CapabilityFixture>() && actual == TypeId::of::<u32>()
        ));

        let default = descriptor.capability::<DefaultCapability>().unwrap();
        assert_eq!(
            default
                .adapter()
                .create_dynamic::<Local>()
                .unwrap()
                .downcast::<CapabilityFixture>()
                .unwrap(),
            CapabilityFixture::default()
        );
        assert!(matches!(
            default.adapter().create_dynamic::<ThreadSafe>(),
            Err(CapabilityInvocationError::UnsupportedMode {
                expected_type,
                mode: CapabilityInvocationMode::ThreadSafe,
            }) if expected_type == TypeId::of::<CapabilityFixture>()
        ));

        let missing_context = linked_context(std::iter::empty::<Registration>());
        let missing = match missing_context
            .type_of::<CapabilityFixture>()
            .capability::<CloneCapability>()
        {
            Ok(_) => panic!("an unregistered capability must be reported as missing"),
            Err(error) => error,
        };
        assert!(matches!(
            missing,
            CapabilityError::Missing {
                capability_id: "qubit.reflect.clone",
                ..
            }
        ));
    }

    #[test]
    fn thread_safe_registrations_support_local_and_thread_safe_adapter_modes() {
        let context = linked_context([
            super::register_thread_safe_clone::<CapabilityFixture>("capability-tests", "1.2.3", "tests", "caps.rs", 11),
            super::register_thread_safe_default::<CapabilityFixture>(
                "capability-tests",
                "1.2.3",
                "tests",
                "caps.rs",
                12,
            ),
        ]);
        let descriptor = context.type_of::<CapabilityFixture>();
        let clone = descriptor.capability::<CloneCapability>().unwrap();

        for clone_value in [
            clone
                .adapter()
                .clone_dynamic::<Local>(DynamicRef::new(&CapabilityFixture(11)))
                .unwrap()
                .downcast::<CapabilityFixture>()
                .unwrap(),
            clone
                .adapter()
                .clone_dynamic::<ThreadSafe>(DynamicRef::new(&CapabilityFixture(12)))
                .unwrap()
                .downcast::<CapabilityFixture>()
                .unwrap(),
        ] {
            assert!(clone_value == CapabilityFixture(11) || clone_value == CapabilityFixture(12));
        }

        let wrong = 23_u32;
        assert!(matches!(
            clone
                .adapter()
                .clone_dynamic::<ThreadSafe>(DynamicRef::new(&wrong)),
            Err(CapabilityInvocationError::TypeMismatch { expected, actual })
                if expected == TypeId::of::<CapabilityFixture>() && actual == TypeId::of::<u32>()
        ));

        let default = descriptor.capability::<DefaultCapability>().unwrap();
        for value in [
            default
                .adapter()
                .create_dynamic::<Local>()
                .unwrap()
                .downcast::<CapabilityFixture>()
                .unwrap(),
            default
                .adapter()
                .create_dynamic::<ThreadSafe>()
                .unwrap()
                .downcast::<CapabilityFixture>()
                .unwrap(),
        ] {
            assert_eq!(value, CapabilityFixture::default());
        }
    }
}
