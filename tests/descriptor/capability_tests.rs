//! Integration tests for typed reflection capabilities.

use std::any::TypeId;
use std::rc::Rc;

use qubit_reflect::capability::{
    CapabilityConflictKind, CapabilityDescriptor, CapabilityKey, TypeCapabilities,
    clone_descriptor, clone_key, default_descriptor, default_key, registered_reflected_type,
    registered_type_capabilities, send_key, sync_key,
};
use qubit_reflect::descriptor::{Reflect, TypeDescriptor};
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::value::ReflectedOwned;
use qubit_reflect::{register_reflected_type, register_type_capabilities};

#[derive(Debug, Eq, PartialEq)]
struct TextAdapter(&'static str);

#[derive(Debug)]
struct NumberAdapter;

#[derive(Clone, Default, Debug, Eq, PartialEq)]
struct LocalOnly {
    values: Rc<Vec<u32>>,
}

struct ReflectedRoot;

static REFLECTED_ROOT_DESCRIPTOR: TypeDescriptor =
    TypeDescriptor::new_opaque::<ReflectedRoot>("capability_tests::ReflectedRoot", "ReflectedRoot");

impl Reflect for ReflectedRoot {
    /// Returns the test type's unique descriptor.
    fn type_descriptor() -> &'static TypeDescriptor {
        &REFLECTED_ROOT_DESCRIPTOR
    }
}

register_reflected_type!(ReflectedRoot);
register_type_capabilities!(LocalOnly: Clone, Default);

/// Creates a valid third-party capability ID for a test.
fn external_id(value: &str) -> CapabilityId {
    CapabilityId::new(value).expect("the test capability ID must be valid")
}

/// Confirms descriptors retain unknown contracts and iterate by stable ID order.
#[test]
fn test_type_capabilities_preserve_unknown_capabilities_in_stable_order() {
    let zeta_key = CapabilityKey::<TextAdapter>::new(external_id("example.capability.zeta"));
    let alpha_key = CapabilityKey::<TextAdapter>::new(external_id("example.capability.alpha"));
    let descriptors = vec![
        CapabilityDescriptor::with_adapter(zeta_key.clone(), TextAdapter("zeta")),
        CapabilityDescriptor::without_adapter(alpha_key.clone()),
    ];

    let capabilities =
        TypeCapabilities::try_new(descriptors).expect("distinct capability IDs must be accepted");

    let ids: Vec<_> = capabilities
        .descriptors()
        .iter()
        .map(|descriptor| descriptor.id().as_str())
        .collect();
    assert_eq!(ids, ["example.capability.alpha", "example.capability.zeta"]);
    assert!(!capabilities.descriptors()[0].has_adapter());
    assert_eq!(capabilities.get(zeta_key), Some(&TextAdapter("zeta")));
    assert_eq!(capabilities.get(alpha_key), None);
}

/// Confirms one stable ID cannot silently acquire a different adapter contract.
#[test]
fn test_type_capabilities_reject_same_id_with_different_adapter_types() {
    let text_key = CapabilityKey::<TextAdapter>::new(external_id("example.capability.shared"));
    let number_key = CapabilityKey::<NumberAdapter>::new(external_id("example.capability.shared"));

    let error = TypeCapabilities::try_new(vec![
        CapabilityDescriptor::without_adapter(text_key),
        CapabilityDescriptor::with_adapter(number_key, NumberAdapter),
    ])
    .expect_err("one capability ID cannot identify two adapter contracts");

    assert_eq!(error.kind(), CapabilityConflictKind::AdapterTypeMismatch);
    assert_eq!(error.id().as_str(), "example.capability.shared");
    assert_ne!(error.first_adapter_type(), error.second_adapter_type());
}

/// Confirms duplicate descriptors are rejected even when their contracts match.
#[test]
fn test_type_capabilities_reject_duplicate_ids() {
    let first_key = CapabilityKey::<TextAdapter>::new(external_id("example.capability.duplicate"));
    let second_key = CapabilityKey::<TextAdapter>::new(external_id("example.capability.duplicate"));

    let error = TypeCapabilities::try_new(vec![
        CapabilityDescriptor::without_adapter(first_key),
        CapabilityDescriptor::without_adapter(second_key),
    ])
    .expect_err("duplicate capability IDs must not be silently coalesced");

    assert_eq!(error.kind(), CapabilityConflictKind::DuplicateId);
}

/// Confirms built-in clone and default adapters retain exact dynamic type checks.
#[test]
fn test_clone_and_default_capabilities_use_safe_local_dynamic_values() {
    let capabilities = TypeCapabilities::try_new(vec![
        clone_descriptor::<String>(),
        default_descriptor::<String>(),
    ])
    .expect("the built-in capability IDs are distinct");

    let clone_adapter = capabilities
        .get(clone_key())
        .expect("the clone adapter must be present");
    let cloned = clone_adapter
        .clone_owned(&ReflectedOwned::new(String::from("clone me")))
        .expect("the dynamic value has the registered type");
    assert_eq!(
        cloned.downcast_ref::<String>().map(String::as_str),
        Some("clone me")
    );

    let mismatch = match clone_adapter.clone_owned(&ReflectedOwned::new(17_u32)) {
        Ok(_) => panic!("an adapter must reject a different concrete type"),
        Err(error) => error,
    };
    assert_eq!(mismatch.expected(), TypeId::of::<String>());
    assert_eq!(mismatch.actual(), TypeId::of::<u32>());

    let defaulted = capabilities
        .get(default_key())
        .expect("the default adapter must be present")
        .create();
    assert_eq!(
        defaulted.downcast_ref::<String>().map(String::as_str),
        Some("")
    );
}

/// Confirms explicit registration is exact and does not infer thread safety.
#[test]
fn test_concrete_capability_registration_keeps_send_and_sync_as_facts_only() {
    let capabilities = registered_type_capabilities::<LocalOnly>()
        .expect("the concrete type has one compatible registration fragment");

    assert!(capabilities.get(clone_key()).is_some());
    assert!(capabilities.get(default_key()).is_some());
    assert!(!capabilities.contains(send_key()));
    assert!(!capabilities.contains(sync_key()));

    let source = LocalOnly {
        values: Rc::new(vec![1, 2, 3]),
    };
    let cloned = capabilities
        .get(clone_key())
        .expect("the clone registration carries its operation adapter")
        .clone_owned(&ReflectedOwned::new(source.clone()))
        .expect("the registered local concrete type must clone dynamically");
    assert_eq!(cloned.downcast_ref::<LocalOnly>(), Some(&source));
}

/// Confirms reflected-type registration returns the existing descriptor root.
#[test]
fn test_reflected_type_registration_preserves_descriptor_root_identity() {
    let registered = registered_reflected_type::<ReflectedRoot>()
        .expect("the exact concrete reflected type must be registered");

    assert!(std::ptr::eq(
        registered,
        TypeDescriptor::of::<ReflectedRoot>()
    ));
}
