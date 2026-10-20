// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use crate::runtime::capability::builtin::CloneCapability;
use crate::runtime::capability::builtin::SendCapability;
use crate::runtime::capability::capabilities::Capabilities;
use crate::runtime::capability::capabilities_view::CapabilitiesView;
use crate::runtime::capability::capability::CapabilitySpec;
use crate::runtime::capability::capability_error::CapabilityContractError;
use crate::runtime::capability::capability_error::CapabilityError;
use crate::runtime::capability::capability_registration::CapabilityRegistration;
use crate::runtime::capability::capability_registration_origin::CapabilityRegistrationOrigin;
use crate::runtime::capability::capability_target::CapabilityTarget;
use crate::runtime::capability::capability_target_id::CapabilityTargetId;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::identity::TypeIdentity;
use crate::runtime::value::DynamicRef;
use crate::runtime::value::Local;
use crate::runtime::value::ThreadSafe;

struct CountAdapter(u8);
struct CountCapability;
struct WrongCountCapability;
#[derive(Clone, Default)]
struct Cloneable(u32);

impl crate::runtime::descriptor::Reflect for Cloneable {
    fn __reflect_registration() -> &'static crate::runtime::descriptor::TypeRegistration<Self> {
        static REGISTRATION: crate::runtime::descriptor::TypeRegistration<Cloneable> =
            crate::runtime::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}

impl CapabilitySpec for CountCapability {
    type Adapter = CountAdapter;
    const ID: &'static str = "example.count";
}

impl CapabilitySpec for WrongCountCapability {
    type Adapter = u16;
    const ID: &'static str = CountCapability::ID;
}

fn target<T: 'static>() -> CapabilityTargetId {
    CapabilityTargetId::Type(TypeIdentity {
        type_id: TypeId::of::<T>(),
        rust_name: std::any::type_name::<T>(),
    })
}

fn registration<T: 'static>(adapter: u8, origin: CapabilityRegistrationOrigin) -> CapabilityRegistration {
    CapabilityRegistration::new::<CountCapability>(target::<T>(), CountAdapter(adapter), origin)
        .expect("fixture capability ID is valid")
}

fn view_for_cloneable(registrations: impl IntoIterator<Item = CapabilityRegistration>) -> CapabilitiesView<'static> {
    let target = TypeDescriptor::of::<Cloneable>();
    let target_view = CapabilityTarget::Type(target);
    let capabilities = Capabilities::try_new(target_view.id(), registrations).expect("unique capability set builds");
    CapabilitiesView::new(std::sync::Arc::new(capabilities), target_view, target.context())
}

#[test]
fn typed_lookup_binds_the_id_and_adapter_type() {
    let capabilities = view_for_cloneable([registration::<Cloneable>(
        9,
        CapabilityRegistrationOrigin::Explicit(origin("typed")),
    )]);

    let capability = capabilities
        .capability::<CountCapability>()
        .expect("typed lookup succeeds");
    assert_eq!(capability.adapter().0, 9);
    assert!(
        matches!(capability.target(), CapabilityTarget::Type(target) if target.type_id() == TypeId::of::<Cloneable>())
    );
    assert!(matches!(
        capability.origin(),
        crate::runtime::capability::capability_origin::CapabilityOrigin::Explicit { .. }
    ));
}

#[test]
fn descriptor_metadata_and_clones_preserve_linked_capability_state() {
    use crate::runtime::capability::capability_origin::CapabilityOrigin;

    let capabilities = view_for_cloneable([registration::<Cloneable>(
        13,
        CapabilityRegistrationOrigin::Linked(origin("linked")),
    )]);
    let descriptor = capabilities
        .get(CountCapability::ID)
        .expect("registered capability exists");

    assert_eq!(descriptor.id(), CountCapability::ID);
    assert_eq!(descriptor.adapter_type_id(), TypeId::of::<CountAdapter>());
    assert!(matches!(descriptor.target(), CapabilityTarget::Type(target)
        if target.type_id() == TypeId::of::<Cloneable>()));
    assert!(matches!(descriptor.origin(), CapabilityOrigin::Linked { registration }
        if registration.registration().module_path == "linked"));

    let capability = descriptor
        .downcast::<CountCapability>()
        .expect("the descriptor satisfies the typed capability contract");
    let cloned = capability.clone();
    assert_eq!(capability.adapter().0, 13);
    assert!(std::ptr::eq(capability.adapter(), cloned.adapter()));
    assert!(matches!(cloned.target(), CapabilityTarget::Type(target)
        if target.type_id() == TypeId::of::<Cloneable>()));
    assert!(matches!(cloned.origin(), CapabilityOrigin::Linked { registration }
        if registration.registration().module_path == "linked"));
}

#[test]
fn capability_views_resolve_targets_and_explicit_origins_in_the_query_context() {
    use crate::runtime::capability::capability_origin::CapabilityOrigin;
    use crate::runtime::context::ReflectionContext;
    use crate::runtime::context::handle::registration::Registration;

    let context = ReflectionContext::builder()
        .register(
            Registration::of::<Cloneable>().with_capability(
                CapabilityRegistration::clone::<Cloneable>(CapabilityRegistrationOrigin::Explicit(origin(
                    "explicit_clone",
                )))
                .expect("Clone registration is valid"),
            ),
        )
        .build()
        .expect("the context contains its explicitly registered capability target");
    let target = context.type_of::<Cloneable>();
    let capabilities = target.capabilities().expect("capability view resolves");
    let descriptor = capabilities
        .get("qubit.reflect.clone")
        .expect("Clone capability exists");
    assert!(matches!(descriptor.target(), CapabilityTarget::Type(resolved)
        if resolved.type_id() == target.type_id() && std::ptr::eq(resolved.context(), &context)));
    assert!(
        matches!(descriptor.origin(), CapabilityOrigin::Explicit { registration }
        if std::ptr::eq(registration.context(), &context)
            && registration.registration().module_path == "explicit_clone")
    );
    assert!(matches!(descriptor.downcast::<CloneCapability>(), Ok(capability)
        if matches!(capability.target(), CapabilityTarget::Type(resolved)
            if std::ptr::eq(resolved.context(), &context))));
}

#[test]
fn missing_capability_is_distinct_from_a_contract_mismatch() {
    let capabilities = view_for_cloneable([]);
    assert!(matches!(
        capabilities.capability::<CountCapability>(),
        Err(CapabilityError::Missing {
            capability_id: "example.count",
            ..
        })
    ));

    let wrong_contract = view_for_cloneable([CapabilityRegistration::new::<WrongCountCapability>(
        target::<Cloneable>(),
        7_u16,
        CapabilityRegistrationOrigin::Explicit(origin("wrong_contract")),
    )
    .expect("fixture capability ID is valid")]);
    assert!(matches!(
        wrong_contract.capability::<CountCapability>(),
        Err(CapabilityError::ContractMismatch { expected_adapter, actual_adapter, .. })
            if expected_adapter == TypeId::of::<CountAdapter>() && actual_adapter == TypeId::of::<u16>()
    ));
}

#[test]
fn erased_descriptor_reports_contract_mismatch_without_calling_it_missing() {
    let capabilities = view_for_cloneable([CapabilityRegistration::new::<WrongCountCapability>(
        target::<Cloneable>(),
        7_u16,
        CapabilityRegistrationOrigin::Explicit(origin("wrong_contract")),
    )
    .expect("fixture capability ID is valid")]);
    let error = capabilities
        .get("example.count")
        .expect("descriptor exists")
        .downcast::<CountCapability>()
        .err()
        .expect("incorrect adapter contract is rejected");
    assert!(matches!(
        error,
        CapabilityContractError::AdapterTypeMismatch { actual, .. }
            if actual == TypeId::of::<u16>()
    ));
}

#[test]
fn conflicting_ids_retain_all_registration_origins() {
    let error = Capabilities::try_new(
        target::<u32>(),
        [
            registration::<u32>(1, CapabilityRegistrationOrigin::Explicit(origin("first"))),
            registration::<u32>(2, CapabilityRegistrationOrigin::Linked(origin("second"))),
            registration::<u32>(3, CapabilityRegistrationOrigin::Explicit(origin("third"))),
        ],
    )
    .err()
    .expect("duplicate capability IDs conflict");
    assert!(matches!(
        error,
        CapabilityError::Conflict { registrations, .. } if registrations.len() == 3
    ));
}

#[test]
fn capability_sets_reject_registrations_for_another_target() {
    let error = Capabilities::try_new(
        target::<u32>(),
        [registration::<u64>(
            1,
            CapabilityRegistrationOrigin::Explicit(origin("wrong_target")),
        )],
    )
    .err()
    .expect("a registration cannot leak across targets");
    assert!(matches!(
        error,
        CapabilityError::TargetMismatch { expected, actual }
            if expected == target::<u32>() && actual == target::<u64>()
    ));
}

#[test]
fn builtin_registrations_require_the_corresponding_rust_trait_bound() {
    let send = CapabilityRegistration::send::<String>(CapabilityRegistrationOrigin::Explicit(origin("send")))
        .expect("String is Send");
    assert_eq!(send.id.as_str(), SendCapability::ID);

    let clone = CapabilityRegistration::clone::<Cloneable>(CapabilityRegistrationOrigin::Explicit(origin("clone")))
        .expect("Cloneable implements Clone");
    assert_eq!(clone.id.as_str(), CloneCapability::ID);

    let sync = CapabilityRegistration::sync::<String>(CapabilityRegistrationOrigin::Explicit(origin("sync")))
        .expect("String is Sync");
    assert_eq!(
        sync.id.as_str(),
        crate::runtime::capability::builtin::SyncCapability::ID
    );

    let default =
        CapabilityRegistration::default::<Cloneable>(CapabilityRegistrationOrigin::Explicit(origin("default")))
            .expect("Cloneable implements Default");
    assert_eq!(
        default.id.as_str(),
        crate::runtime::capability::builtin::DefaultCapability::ID
    );
}

#[test]
fn builtin_clone_and_default_adapters_operate_on_the_proven_concrete_type() {
    let clone_registration =
        CapabilityRegistration::clone::<Cloneable>(CapabilityRegistrationOrigin::Explicit(origin("clone")))
            .expect("Cloneable implements Clone");
    let clone_adapter = clone_registration
        .adapter
        .downcast_ref::<crate::runtime::capability::builtin::CloneAdapter>()
        .expect("registration carries a clone adapter");
    let source = Cloneable(47);
    let cloned = clone_adapter
        .clone_dynamic::<Local>(DynamicRef::new(&source))
        .expect("adapter accepts the registered concrete type");
    assert_eq!(cloned.downcast_ref::<Cloneable>().expect("cloned value type").0, 47);
    struct Other;
    impl crate::runtime::descriptor::Reflect for Other {
        fn __reflect_registration() -> &'static crate::runtime::descriptor::TypeRegistration<Self> {
            static REGISTRATION: crate::runtime::descriptor::TypeRegistration<Other> =
                crate::runtime::descriptor::TypeRegistration::new();
            &REGISTRATION
        }
    }
    let mismatch = clone_adapter
        .clone_dynamic::<Local>(DynamicRef::new(&Other))
        .err()
        .expect("the exact value TypeId is checked");
    assert!(matches!(
        mismatch,
        crate::runtime::capability::capability_error::CapabilityInvocationError::TypeMismatch {
            expected,
            actual,
        } if expected == TypeId::of::<Cloneable>() && actual == TypeId::of::<Other>()
    ));

    let thread_safe_registration = CapabilityRegistration::clone_thread_safe::<Cloneable>(
        CapabilityRegistrationOrigin::Explicit(origin("thread_safe_clone")),
    )
    .expect("Cloneable is Clone + Send + Sync");
    let thread_safe_adapter = thread_safe_registration
        .adapter
        .downcast_ref::<crate::runtime::capability::builtin::CloneAdapter>()
        .expect("registration carries a clone adapter");
    let thread_safe_clone = thread_safe_adapter
        .clone_dynamic::<ThreadSafe>(DynamicRef::<ThreadSafe>::new(&source))
        .expect("adapter supports the thread-safe value mode");
    assert_eq!(
        thread_safe_clone
            .downcast_ref::<Cloneable>()
            .expect("cloned value type")
            .0,
        47
    );
    let thread_safe_mismatch = thread_safe_adapter
        .clone_dynamic::<ThreadSafe>(DynamicRef::<ThreadSafe>::new(&Other))
        .err()
        .expect("thread-safe mode retains the same type mismatch category");
    assert!(matches!(
        thread_safe_mismatch,
        crate::runtime::capability::capability_error::CapabilityInvocationError::TypeMismatch {
            expected,
            actual,
        } if expected == TypeId::of::<Cloneable>() && actual == TypeId::of::<Other>()
    ));

    let default_registration =
        CapabilityRegistration::default::<Cloneable>(CapabilityRegistrationOrigin::Explicit(origin("default")))
            .expect("Cloneable implements Default");
    let default_adapter = default_registration
        .adapter
        .downcast_ref::<crate::runtime::capability::builtin::DefaultAdapter>()
        .expect("registration carries a default adapter");
    assert_eq!(
        default_adapter
            .create_dynamic::<Local>()
            .expect("local mode is available")
            .downcast_ref::<Cloneable>()
            .expect("default value type")
            .0,
        0,
    );
    let unsupported_mode = default_adapter
        .create_dynamic::<ThreadSafe>()
        .err()
        .expect("local-only registration rejects thread-safe creation");
    assert!(matches!(
        unsupported_mode,
        crate::runtime::capability::capability_error::CapabilityInvocationError::UnsupportedMode {
            expected_type,
            mode: crate::runtime::capability::capability_error::CapabilityInvocationMode::ThreadSafe,
        } if expected_type == TypeId::of::<Cloneable>()
    ));

    let thread_safe_default_registration = CapabilityRegistration::default_thread_safe::<Cloneable>(
        CapabilityRegistrationOrigin::Explicit(origin("thread_safe_default")),
    )
    .expect("Cloneable is Default + Send + Sync");
    let thread_safe_default_adapter = thread_safe_default_registration
        .adapter
        .downcast_ref::<crate::runtime::capability::builtin::DefaultAdapter>()
        .expect("registration carries a default adapter");
    assert_eq!(
        thread_safe_default_adapter
            .create_dynamic::<ThreadSafe>()
            .expect("adapter supports the thread-safe value mode")
            .downcast_ref::<Cloneable>()
            .expect("default value type")
            .0,
        0,
    );
}

#[test]
fn invalid_external_capability_id_is_rejected_during_registration() {
    struct InvalidCapability;
    impl CapabilitySpec for InvalidCapability {
        type Adapter = ();
        const ID: &'static str = "example..invalid";
    }

    let error = CapabilityRegistration::new::<InvalidCapability>(
        target::<u32>(),
        (),
        CapabilityRegistrationOrigin::Explicit(origin("invalid_id")),
    )
    .err()
    .expect("malformed namespaced ID is rejected");
    assert!(matches!(
        error,
        crate::runtime::capability::capability_error::CapabilityRegistrationError::InvalidId(_)
    ));

    struct ReservedCapability;
    impl CapabilitySpec for ReservedCapability {
        type Adapter = ();
        const ID: &'static str = "qubit.reflect.forged";
    }
    let error = CapabilityRegistration::new::<ReservedCapability>(
        target::<u32>(),
        (),
        CapabilityRegistrationOrigin::Explicit(origin("reserved")),
    )
    .err()
    .expect("external specs cannot use the built-in namespace");
    assert!(matches!(
        error,
        crate::runtime::capability::capability_error::CapabilityRegistrationError::InvalidId(_)
    ));
}

#[test]
fn capability_contract_errors_report_id_and_adapter_identity_boundaries() {
    struct SameAdapterDifferentId;
    impl CapabilitySpec for SameAdapterDifferentId {
        type Adapter = CountAdapter;
        const ID: &'static str = "example.other";
    }

    let capabilities = view_for_cloneable([registration::<Cloneable>(
        5,
        CapabilityRegistrationOrigin::Explicit(origin("contract_boundaries")),
    )]);
    let error = capabilities
        .get(CountCapability::ID)
        .expect("registered capability exists")
        .downcast::<SameAdapterDifferentId>()
        .err()
        .expect("matching adapter type cannot override the stable ID");
    assert!(matches!(
        &error,
        CapabilityContractError::IdMismatch { expected: "example.other", actual }
            if actual.as_str() == CountCapability::ID
    ));
    assert_eq!(
        error.to_string(),
        "capability ID mismatch: expected example.other, got example.count"
    );
    assert!(std::error::Error::source(&error).is_none());

    let error = capabilities
        .get(CountCapability::ID)
        .expect("registered capability exists")
        .downcast::<WrongCountCapability>()
        .err()
        .expect("matching stable ID cannot override the adapter type");
    assert!(matches!(
        &error,
        CapabilityContractError::AdapterTypeMismatch { capability_id, expected, actual }
            if capability_id.as_str() == CountCapability::ID
                && *expected == TypeId::of::<u16>()
                && *actual == TypeId::of::<CountAdapter>()
    ));
    assert_eq!(
        error.to_string(),
        "adapter contract mismatch for capability example.count"
    );
}

#[test]
fn capability_builder_keeps_type_and_definition_targets_distinct() {
    use crate::runtime::descriptor::generic::TypeDefinitionId;

    let definition = TypeDefinitionId::of::<Cloneable>();
    let definition_target = CapabilityTargetId::Definition(definition);
    let registration = CapabilityRegistration::for_definition::<CountCapability>(definition, CountAdapter(3));
    assert!(Capabilities::try_new(definition_target.clone(), [registration]).is_ok());

    let type_target = target::<Cloneable>();
    let error = Capabilities::try_new(
        type_target.clone(),
        [CapabilityRegistration::for_definition::<CountCapability>(
            definition,
            CountAdapter(4),
        )],
    )
    .err()
    .expect("a definition registration cannot be consumed by a concrete type target");
    assert!(matches!(
        error,
        CapabilityError::TargetMismatch { expected, actual }
            if expected == type_target && actual == definition_target
    ));
}

#[test]
fn capability_errors_preserve_sources_and_render_each_public_error_category() {
    let target_id = target::<Cloneable>();
    let context_error = crate::runtime::error::ContextError::new("invalid context fixture");
    let context = CapabilityError::Context(context_error.clone());
    assert_eq!(
        context.to_string(),
        "reflection context failed: invalid context fixture"
    );
    assert_eq!(
        std::error::Error::source(&context)
            .expect("context error is the source")
            .to_string(),
        "invalid context fixture"
    );

    let missing = CapabilityError::Missing {
        target: target_id.clone(),
        capability_id: CountCapability::ID,
    };
    assert_eq!(missing.to_string(), "missing capability example.count");
    assert!(std::error::Error::source(&missing).is_none());

    let mismatch = CapabilityError::TargetMismatch {
        expected: target_id.clone(),
        actual: target::<u64>(),
    };
    assert_eq!(
        mismatch.to_string(),
        "capability registration targets a different reflection entity"
    );

    let contract = CapabilityError::ContractMismatch {
        target: target_id.clone(),
        capability_id: crate::identity::CapabilityId::new(CountCapability::ID).expect("fixture capability ID is valid"),
        expected_adapter: TypeId::of::<CountAdapter>(),
        actual_adapter: TypeId::of::<u16>(),
        origin: CapabilityRegistrationOrigin::Explicit(origin("render_contract")),
    };
    assert_eq!(
        contract.to_string(),
        "adapter contract mismatch for capability example.count"
    );

    let conflict = CapabilityError::Conflict {
        target: target_id,
        capability_id: crate::identity::CapabilityId::new(CountCapability::ID).expect("fixture capability ID is valid"),
        registrations: Box::new([CapabilityRegistrationOrigin::Explicit(origin("first"))]),
    };
    assert_eq!(
        conflict.to_string(),
        "conflicting registrations for capability example.count"
    );
    assert!(std::error::Error::source(&conflict).is_none());

    let nested = CapabilityError::Contract(CapabilityContractError::IdMismatch {
        expected: "example.other",
        actual: crate::identity::CapabilityId::new(CountCapability::ID).expect("fixture capability ID is valid"),
    });
    assert_eq!(
        nested.to_string(),
        "capability ID mismatch: expected example.other, got example.count"
    );
    assert!(std::error::Error::source(&nested).is_some());
}

#[test]
fn invocation_errors_distinguish_wrong_value_types_from_unsupported_modes() {
    use crate::runtime::capability::capability_error::CapabilityInvocationError;
    use crate::runtime::capability::capability_error::CapabilityInvocationMode;

    let mismatch = CapabilityInvocationError::TypeMismatch {
        expected: TypeId::of::<Cloneable>(),
        actual: TypeId::of::<u32>(),
    };
    assert_eq!(
        mismatch.to_string(),
        format!(
            "capability adapter expected value type {:?}, got {:?}",
            TypeId::of::<Cloneable>(),
            TypeId::of::<u32>()
        )
    );
    assert!(std::error::Error::source(&mismatch).is_none());

    let unsupported = CapabilityInvocationError::UnsupportedMode {
        expected_type: TypeId::of::<Cloneable>(),
        mode: CapabilityInvocationMode::ThreadSafe,
    };
    assert_eq!(
        unsupported.to_string(),
        format!(
            "capability adapter for {:?} does not support ThreadSafe values",
            TypeId::of::<Cloneable>()
        )
    );
}

#[test]
fn invalid_registration_id_error_keeps_the_validation_cause() {
    use crate::runtime::capability::capability_error::CapabilityRegistrationError;

    let id_error = crate::identity::CapabilityId::new("invalid..id").expect_err("malformed IDs fail validation");
    let registration_error = CapabilityRegistrationError::InvalidId(id_error);
    assert!(registration_error.to_string().starts_with("invalid capability ID:"));
    assert!(std::error::Error::source(&registration_error).is_some());
}

fn origin(module_path: &'static str) -> crate::runtime::identity::RegistrationOrigin {
    crate::runtime::identity::RegistrationOrigin {
        crate_name: "capability-tests",
        crate_version: None,
        module_path,
        file: None,
        line: None,
        column: None,
    }
}
