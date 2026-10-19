use std::any::TypeId;

use super::builtin::{CloneCapability, SendCapability};
use super::capabilities::Capabilities;
use super::capability::CapabilitySpec;
use super::capability_error::{CapabilityContractError, CapabilityError};
use super::capability_origin::CapabilityOrigin;
use super::capability_registration::CapabilityRegistration;
use super::capability_target::CapabilityTargetId;
use crate::runtime::identity::TypeIdentity;
use crate::runtime::value::{DynamicRef, ThreadSafe};

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

fn registration<T: 'static>(adapter: u8, origin: CapabilityOrigin) -> CapabilityRegistration {
    CapabilityRegistration::new::<CountCapability>(target::<T>(), CountAdapter(adapter), origin)
        .expect("fixture capability ID is valid")
}

#[test]
fn typed_lookup_binds_the_id_and_adapter_type() {
    let target = target::<u32>();
    let capabilities = Capabilities::try_new(
        target.clone(),
        [registration::<u32>(
            9,
            CapabilityOrigin::Explicit(origin("typed")),
        )],
    )
    .expect("unique capability set builds");

    let capability = capabilities
        .capability::<CountCapability>()
        .expect("typed lookup succeeds");
    assert_eq!(capability.adapter().0, 9);
    assert_eq!(capability.target(), &target);
    assert!(matches!(capability.origin(), CapabilityOrigin::Explicit(_)));
}

#[test]
fn missing_capability_is_distinct_from_a_contract_mismatch() {
    let capabilities = Capabilities::try_new(target::<u32>(), []).expect("empty set builds");
    assert!(matches!(
        capabilities.capability::<CountCapability>(),
        Err(CapabilityError::Missing {
            capability_id: "example.count",
            ..
        })
    ));

    let wrong_contract = Capabilities::try_new(
        target::<u32>(),
        [CapabilityRegistration::new::<WrongCountCapability>(
            target::<u32>(),
            7_u16,
            CapabilityOrigin::Intrinsic,
        )
        .expect("fixture capability ID is valid")],
    )
    .expect("one declared provider builds");
    assert!(matches!(
        wrong_contract.capability::<CountCapability>(),
        Err(CapabilityError::ContractMismatch { expected_adapter, actual_adapter, .. })
            if expected_adapter == TypeId::of::<CountAdapter>() && actual_adapter == TypeId::of::<u16>()
    ));
}

#[test]
fn erased_descriptor_reports_contract_mismatch_without_calling_it_missing() {
    let capabilities = Capabilities::try_new(
        target::<u32>(),
        [CapabilityRegistration::new::<WrongCountCapability>(
            target::<u32>(),
            7_u16,
            CapabilityOrigin::Intrinsic,
        )
        .expect("fixture capability ID is valid")],
    )
    .expect("one declared provider builds");
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
            registration::<u32>(1, CapabilityOrigin::Explicit(origin("first"))),
            registration::<u32>(2, CapabilityOrigin::Linked(origin("second"))),
            registration::<u32>(3, CapabilityOrigin::Intrinsic),
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
        [registration::<u64>(1, CapabilityOrigin::Intrinsic)],
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
    let send = CapabilityRegistration::send::<String>(CapabilityOrigin::Intrinsic)
        .expect("String is Send");
    assert_eq!(send.id.as_str(), SendCapability::ID);

    let clone = CapabilityRegistration::clone::<Cloneable>(CapabilityOrigin::Intrinsic)
        .expect("Cloneable implements Clone");
    assert_eq!(clone.id.as_str(), CloneCapability::ID);

    let sync = CapabilityRegistration::sync::<String>(CapabilityOrigin::Intrinsic)
        .expect("String is Sync");
    assert_eq!(sync.id.as_str(), super::builtin::SyncCapability::ID);

    let default = CapabilityRegistration::default::<Cloneable>(CapabilityOrigin::Intrinsic)
        .expect("Cloneable implements Default");
    assert_eq!(default.id.as_str(), super::builtin::DefaultCapability::ID);
}

#[test]
fn builtin_clone_and_default_adapters_operate_on_the_proven_concrete_type() {
    let clone_registration =
        CapabilityRegistration::clone::<Cloneable>(CapabilityOrigin::Intrinsic)
            .expect("Cloneable implements Clone");
    let clone_adapter = clone_registration
        .adapter
        .downcast_ref::<super::builtin::CloneAdapter>()
        .expect("registration carries a clone adapter");
    let source = Cloneable(47);
    let cloned = clone_adapter
        .clone_local(DynamicRef::new(&source))
        .expect("adapter accepts the registered concrete type");
    assert_eq!(
        cloned
            .downcast_ref::<Cloneable>()
            .expect("cloned value type")
            .0,
        47
    );
    struct Other;
    impl crate::runtime::descriptor::Reflect for Other {
        fn __reflect_registration() -> &'static crate::runtime::descriptor::TypeRegistration<Self> {
            static REGISTRATION: crate::runtime::descriptor::TypeRegistration<Other> =
                crate::runtime::descriptor::TypeRegistration::new();
            &REGISTRATION
        }
    }
    assert!(clone_adapter.clone_local(DynamicRef::new(&Other)).is_none());

    let thread_safe_registration =
        CapabilityRegistration::clone_thread_safe::<Cloneable>(CapabilityOrigin::Intrinsic)
            .expect("Cloneable is Clone + Send + Sync");
    let thread_safe_adapter = thread_safe_registration
        .adapter
        .downcast_ref::<super::builtin::CloneAdapter>()
        .expect("registration carries a clone adapter");
    let thread_safe_clone = thread_safe_adapter
        .clone_thread_safe(DynamicRef::<ThreadSafe>::new(&source))
        .expect("adapter supports the thread-safe value mode");
    assert_eq!(
        thread_safe_clone
            .downcast_ref::<Cloneable>()
            .expect("cloned value type")
            .0,
        47
    );

    let default_registration =
        CapabilityRegistration::default::<Cloneable>(CapabilityOrigin::Intrinsic)
            .expect("Cloneable implements Default");
    let default_adapter = default_registration
        .adapter
        .downcast_ref::<super::builtin::DefaultAdapter>()
        .expect("registration carries a default adapter");
    assert_eq!(
        default_adapter
            .create_local()
            .downcast_ref::<Cloneable>()
            .expect("default value type")
            .0,
        0,
    );

    let thread_safe_default_registration =
        CapabilityRegistration::default_thread_safe::<Cloneable>(CapabilityOrigin::Intrinsic)
            .expect("Cloneable is Default + Send + Sync");
    let thread_safe_default_adapter = thread_safe_default_registration
        .adapter
        .downcast_ref::<super::builtin::DefaultAdapter>()
        .expect("registration carries a default adapter");
    assert_eq!(
        thread_safe_default_adapter
            .create_thread_safe()
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
        CapabilityOrigin::Intrinsic,
    )
    .err()
    .expect("malformed namespaced ID is rejected");
    assert!(matches!(
        error,
        super::capability_error::CapabilityRegistrationError::InvalidId(_)
    ));

    struct ReservedCapability;
    impl CapabilitySpec for ReservedCapability {
        type Adapter = ();
        const ID: &'static str = "qubit.reflect.forged";
    }
    let error = CapabilityRegistration::new::<ReservedCapability>(
        target::<u32>(),
        (),
        CapabilityOrigin::Explicit(origin("reserved")),
    )
    .err()
    .expect("external specs cannot use the built-in namespace");
    assert!(matches!(
        error,
        super::capability_error::CapabilityRegistrationError::InvalidId(_)
    ));
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
