// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

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
/// # Examples
///
/// ```
/// use qubit_reflect::register_type_capabilities;
///
/// register_type_capabilities!(String: Clone, Send, Sync);
/// ```
///
/// Third-party typed operations use the bracket form, where each key expression
/// must match its adapter expression's Rust type:
///
/// ```
/// use qubit_reflect::capability::CapabilityKey;
/// use qubit_reflect::capability::TypeCapabilities;
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
            ) -> $crate::__private::codegen_v3::registration::RuntimeIdentity {
                $crate::__private::codegen_v3::registration::RuntimeIdentity::Capabilities(
                    $crate::__private::codegen_v3::registration::CapabilityTarget::Type(
                        __qubit_reflect_target_descriptor().type_id(),
                    ),
                )
            }

            fn __qubit_reflect_payload(
            ) -> $crate::__private::codegen_v3::registration::FragmentPayload {
                $crate::__private::codegen_v3::registration::FragmentPayload::Capability(
                    $crate::__private::codegen_v3::registration::CapabilityRegistration::for_type(
                        __qubit_reflect_target_descriptor(),
                        __qubit_reflect_descriptors(),
                    ),
                )
            }

            $crate::__private::codegen_v3::inventory::submit! {
                $crate::__private::codegen_v3::registration::RegistrationFragment::new(
                    $crate::__private::codegen_v3::registration::FragmentKind::Capability,
                    $crate::__private::codegen_v3::registration::StaticFragmentIdentity::new(
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
            ) -> $crate::__private::codegen_v3::registration::RuntimeIdentity {
                $crate::__private::codegen_v3::registration::RuntimeIdentity::Capabilities(
                    $crate::__private::codegen_v3::registration::CapabilityTarget::Type(
                        __qubit_reflect_target_descriptor().type_id(),
                    ),
                )
            }

            fn __qubit_reflect_payload(
            ) -> $crate::__private::codegen_v3::registration::FragmentPayload {
                $crate::__private::codegen_v3::registration::FragmentPayload::Capability(
                    $crate::__private::codegen_v3::registration::CapabilityRegistration::for_type(
                        __qubit_reflect_target_descriptor(),
                        __qubit_reflect_descriptors(),
                    ),
                )
            }

            $crate::__private::codegen_v3::inventory::submit! {
                $crate::__private::codegen_v3::registration::RegistrationFragment::new(
                    $crate::__private::codegen_v3::registration::FragmentKind::Capability,
                    $crate::__private::codegen_v3::registration::StaticFragmentIdentity::new(
                        env!("CARGO_PKG_NAME"), module_path!(), line!(), column!(), "capability", 0,
                    ),
                    __qubit_reflect_runtime_identity,
                    __qubit_reflect_payload,
                )
            }
        };
    };
}

/// Registers typed capabilities for a generic definition without adding it as
/// a registry definition member.
///
/// # Examples
///
/// ```
/// #[cfg(feature = "derive")]
/// fn main() {
/// use std::sync::OnceLock;
/// use qubit_reflect::capability::{CapabilityKey, TypeCapabilities};
/// use qubit_reflect::descriptor::{TypeDefinitionDescriptor, TypeDefinitionId};
/// use qubit_reflect::expression::DiagnosticText;
/// use qubit_reflect::expression::ExpressionName;
/// use qubit_reflect::expression::GenericDefinitionDescriptor;
/// use qubit_reflect::expression::GenericParameterDescriptor;
/// use qubit_reflect::identity::CapabilityId;
/// use qubit_reflect::register_definition_capabilities;
///
/// struct ExampleAdapter;
/// struct ExampleDefinition;
/// fn example_type_definition() -> &'static TypeDefinitionDescriptor {
///     static GENERICS: OnceLock<GenericDefinitionDescriptor> = OnceLock::new();
///     static DEFINITION: OnceLock<TypeDefinitionDescriptor> = OnceLock::new();
///     DEFINITION.get_or_init(|| {
///         let parameter = GenericParameterDescriptor::Type {
///             name: ExpressionName::new("T").expect("valid generic parameter"),
///             bounds: Box::default(),
///             default: None,
///             diagnostic: DiagnosticText::default(),
///         };
///         let generics = GENERICS.get_or_init(|| GenericDefinitionDescriptor::new([parameter], []));
///         TypeDefinitionDescriptor::opaque(
///             TypeDefinitionId::of::<ExampleDefinition>(),
///             "example::Generic<T>",
///             "Generic",
///             generics,
///         )
///     })
/// }
///
/// register_definition_capabilities! {
///     definition = example_type_definition,
///     capabilities = [
///         CapabilityKey::<ExampleAdapter>::new(
///             CapabilityId::new("example.generic").expect("valid ID"),
///         ) => ExampleAdapter,
///     ],
/// }
///
/// let _ = TypeCapabilities::default();
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
#[macro_export]
macro_rules! register_definition_capabilities {
    (
        definition = $definition:path,
        capabilities = [$($key:expr => $adapter:expr),+ $(,)?],
        source = (
            $declaring_crate:expr,
            $module_path:expr,
            $line:expr,
            $column:expr,
            $member_kind:expr,
            $fingerprint:expr $(,)?
        ),
    ) => {
        const _: () = {
            fn __qubit_reflect_definition() -> &'static $crate::descriptor::TypeDefinitionDescriptor { $definition() }
            fn __qubit_reflect_runtime_identity() -> $crate::__private::codegen_v3::registration::RuntimeIdentity {
                $crate::__private::codegen_v3::registration::RuntimeIdentity::Capabilities(
                    $crate::__private::codegen_v3::registration::CapabilityTarget::TypeDefinition(
                        __qubit_reflect_definition().id(),
                    ),
                )
            }
            fn __qubit_reflect_payload() -> $crate::__private::codegen_v3::registration::FragmentPayload {
                $crate::__private::codegen_v3::registration::FragmentPayload::Capability(
                    $crate::__private::codegen_v3::registration::CapabilityRegistration::for_definition(
                        __qubit_reflect_definition(),
                        ::std::vec![$($crate::capability::CapabilityDescriptor::with_adapter($key, $adapter)),+]))
            }
            $crate::__private::codegen_v3::inventory::submit! {
                $crate::__private::codegen_v3::registration::RegistrationFragment::new(
                    $crate::__private::codegen_v3::registration::FragmentKind::Capability,
                    $crate::__private::codegen_v3::registration::StaticFragmentIdentity::new(
                        $declaring_crate, $module_path, $line, $column, $member_kind, $fingerprint),
                    __qubit_reflect_runtime_identity, __qubit_reflect_payload)
            }
        };
    };
    (
        definition = $definition:path,
        capabilities = [$($key:expr => $adapter:expr),+ $(,)?],
    ) => {
        $crate::register_definition_capabilities! {
            definition = $definition,
            capabilities = [$($key => $adapter),+],
            source = (env!("CARGO_PKG_NAME"), module_path!(), line!(), column!(), "definition-capability", 0_u64),
        }
    };
}

/// Registers an existing [`Reflect`](crate::descriptor::Reflect) descriptor
/// root.
///
/// The emitted fragment calls `TypeDescriptor::of` and therefore both verifies
/// the trait bound and preserves the interner's existing root identity.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// #[cfg(feature = "derive")]
/// fn main() {
/// use qubit_reflect::Reflect;
/// use qubit_reflect::register_reflected_type;
///
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// struct Example;
///
/// register_reflected_type!(Example);
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
#[macro_export]
macro_rules! register_reflected_type {
    ($target:ty $(,)?) => {
        const _: () = {
            fn __qubit_reflect_target_type_id() -> ::std::any::TypeId {
                ::std::any::TypeId::of::<$target>()
            }

            fn __qubit_reflect_descriptor() -> &'static $crate::descriptor::TypeDescriptor {
                <$target as $crate::descriptor::Reflect>::type_descriptor()
            }

            fn __qubit_reflect_runtime_identity() -> $crate::__private::codegen_v3::registration::RuntimeIdentity {
                $crate::__private::codegen_v3::registration::RuntimeIdentity::Type(__qubit_reflect_target_type_id())
            }

            fn __qubit_reflect_payload() -> $crate::__private::codegen_v3::registration::FragmentPayload {
                $crate::__private::codegen_v3::registration::FragmentPayload::Type(__qubit_reflect_descriptor())
            }

            $crate::__private::codegen_v3::inventory::submit! {
                $crate::__private::codegen_v3::registration::RegistrationFragment::new(
                    $crate::__private::codegen_v3::registration::FragmentKind::Type,
                    $crate::__private::codegen_v3::registration::StaticFragmentIdentity::new(
                        env!("CARGO_PKG_NAME"), module_path!(), line!(), column!(), "type", 0,
                    ),
                    __qubit_reflect_runtime_identity,
                    __qubit_reflect_payload,
                )
            }
        };
    };
}
