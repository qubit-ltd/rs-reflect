// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Typed registrations for built-in runtime capabilities.

use crate::runtime::capability::capability_origin::CapabilityOrigin;
use crate::runtime::capability::capability_registration::CapabilityRegistration;
use crate::runtime::context::handle::registration::{Registration, RegistrationFragment};
use crate::runtime::identity::RegistrationOrigin;

pub use crate::runtime::capability::builtin::{
    CloneCapability, DefaultCapability, SendCapability, SyncCapability,
};

fn linked_origin(
    crate_name: &'static str,
    crate_version: &'static str,
    module_path: &'static str,
    file: &'static str,
    line: u32,
) -> CapabilityOrigin {
    CapabilityOrigin::Linked(RegistrationOrigin {
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
    let capability = CapabilityRegistration::send::<T>(linked_origin(
        crate_name,
        crate_version,
        module_path,
        file,
        line,
    ))
    .expect("the built-in Send capability ID is valid");
    Registration::of::<T>().with_capability(capability)
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
    let capability = CapabilityRegistration::sync::<T>(linked_origin(
        crate_name,
        crate_version,
        module_path,
        file,
        line,
    ))
    .expect("the built-in Sync capability ID is valid");
    Registration::of::<T>().with_capability(capability)
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
    let capability = CapabilityRegistration::clone::<T>(linked_origin(
        crate_name,
        crate_version,
        module_path,
        file,
        line,
    ))
    .expect("the built-in Clone capability ID is valid");
    Registration::of::<T>().with_capability(capability)
}

/// Creates a linked Clone capability registration with Local and ThreadSafe adapters.
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
    Registration::of::<T>().with_capability(capability)
}

/// Creates a linked local-mode Default capability registration for `T`.
#[doc(hidden)]
pub fn register_default<
    T: crate::runtime::descriptor::Reflect + std::default::Default + 'static,
>(
    crate_name: &'static str,
    crate_version: &'static str,
    module_path: &'static str,
    file: &'static str,
    line: u32,
) -> Registration {
    let capability = CapabilityRegistration::default::<T>(linked_origin(
        crate_name,
        crate_version,
        module_path,
        file,
        line,
    ))
    .expect("the built-in Default capability ID is valid");
    Registration::of::<T>().with_capability(capability)
}

/// Creates a linked Default capability registration with Local and ThreadSafe adapters.
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
    Registration::of::<T>().with_capability(capability)
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
