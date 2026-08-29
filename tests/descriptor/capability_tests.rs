//! Integration tests for typed reflection capabilities.

use std::any::TypeId;
use std::rc::Rc;
use std::sync::OnceLock;

use qubit_reflect::capability::CapabilityConflictKind;
use qubit_reflect::capability::CapabilityDescriptor;
use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::capability::TypeCapabilities;
use qubit_reflect::capability::clone_descriptor;
use qubit_reflect::capability::clone_key;
use qubit_reflect::capability::default_descriptor;
use qubit_reflect::capability::default_key;
use qubit_reflect::capability::registered_reflected_type;
use qubit_reflect::capability::registered_type_capabilities;
use qubit_reflect::capability::send_key;
use qubit_reflect::capability::sync_key;
use qubit_reflect::descriptor::Reflect;
use qubit_reflect::descriptor::TypeDescriptor;
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::register_reflected_type;
use qubit_reflect::register_type_capabilities;
use qubit_reflect::value::ReflectedOwned;

#[derive(Debug, Eq, PartialEq)]
struct TextAdapter(&'static str);

#[derive(Debug)]
struct NumberAdapter;

#[derive(Debug, Eq, PartialEq)]
struct ExtensionAdapter(&'static str);

#[derive(Clone, Default, Debug, Eq, PartialEq)]
struct LocalOnly {
    values: Rc<Vec<u32>>,
}

#[derive(Clone, Default)]
struct SendSync;

struct ExtensionRegistration;

#[derive(Clone)]
struct DuplicateRegistration;

struct ReflectedRoot;

/// Builds the test root's capability set once for its static descriptor.
fn reflected_root_capabilities() -> &'static TypeCapabilities {
    static CAPABILITIES: OnceLock<TypeCapabilities> = OnceLock::new();
    CAPABILITIES.get_or_init(|| {
        TypeCapabilities::try_new(vec![CapabilityDescriptor::with_adapter(
            extension_key(),
            ExtensionAdapter("descriptor"),
        )])
        .expect("the test extension capability must be valid")
    })
}

static REFLECTED_ROOT_DESCRIPTOR: TypeDescriptor = qubit_reflect::__private::descriptor::opaque_root_with_capabilities::<
    ReflectedRoot,
>("ReflectedRoot", reflected_root_capabilities);

impl Reflect for ReflectedRoot {
    /// Returns the test type's unique descriptor.
    fn type_descriptor() -> &'static TypeDescriptor {
        &REFLECTED_ROOT_DESCRIPTOR
    }
}

register_reflected_type!(ReflectedRoot);
register_type_capabilities!(LocalOnly: Clone, Default);
register_type_capabilities!(SendSync: Clone, Default, Send, Sync);
register_type_capabilities!(ExtensionRegistration: [extension_key() => ExtensionAdapter("registered")]);
register_type_capabilities!(DuplicateRegistration: Clone);
register_type_capabilities!(DuplicateRegistration: Clone);

/// Creates a valid third-party capability ID for a test.
fn external_id(value: &str) -> CapabilityId {
    CapabilityId::new(value).expect("the test capability ID must be valid")
}

/// Returns the typed third-party extension key used by static registrations.
fn extension_key() -> CapabilityKey<ExtensionAdapter> {
    CapabilityKey::new(external_id("example.capability.extension"))
}

/// Confirms descriptors retain unknown contracts and iterate by stable ID
/// order.
#[test]
fn test_type_capabilities_preserve_unknown_capabilities_in_stable_order() {
    let zeta_key = CapabilityKey::<TextAdapter>::new(external_id("example.capability.zeta"));
    let alpha_key = CapabilityKey::<TextAdapter>::new(external_id("example.capability.alpha"));
    let descriptors = vec![
        CapabilityDescriptor::with_adapter(zeta_key.clone(), TextAdapter("zeta")),
        CapabilityDescriptor::without_adapter(alpha_key.clone()),
    ];

    let capabilities = TypeCapabilities::try_new(descriptors).expect("distinct capability IDs must be accepted");

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

/// Confirms built-in clone and default adapters retain exact dynamic type
/// checks.
#[test]
fn test_clone_and_default_capabilities_use_safe_local_dynamic_values() {
    let capabilities = TypeCapabilities::try_new(vec![clone_descriptor::<String>(), default_descriptor::<String>()])
        .expect("the built-in capability IDs are distinct");

    let clone_adapter = capabilities
        .get(clone_key())
        .expect("the clone adapter must be present");
    let cloned = clone_adapter
        .clone_owned(&ReflectedOwned::new(String::from("clone me")))
        .expect("the dynamic value has the registered type");
    assert_eq!(cloned.downcast_ref::<String>().map(String::as_str), Some("clone me"));

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
    assert_eq!(defaulted.downcast_ref::<String>().map(String::as_str), Some(""));
}

/// Confirms descriptors expose their own typed capability set without a
/// registry lookup.
#[test]
fn test_type_descriptor_navigates_its_owned_typed_capabilities() {
    let descriptor = TypeDescriptor::of::<ReflectedRoot>();

    assert_eq!(
        descriptor.get_capability(extension_key()),
        Some(&ExtensionAdapter("descriptor"))
    );
    assert_eq!(descriptor.capabilities().descriptors().len(), 1);
}

/// Confirms built-in descriptors register core facts and executable operations.
#[test]
fn test_builtin_primitive_and_text_descriptors_register_core_capabilities() {
    let primitive = TypeDescriptor::of::<u32>();
    let text = TypeDescriptor::of::<String>();

    assert!(primitive.capabilities().contains(send_key()));
    assert!(primitive.capabilities().contains(sync_key()));
    assert!(primitive.get_capability(clone_key()).is_some());
    assert!(primitive.get_capability(default_key()).is_some());
    assert_eq!(
        text.get_capability(clone_key())
            .expect("String must have an actual clone adapter")
            .clone_owned(&ReflectedOwned::new(String::from("text")))
            .expect("the registered String adapter must accept String")
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("text")
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

/// Confirms explicitly declared thread-safety facts have no operation adapter.
#[test]
fn test_concrete_registration_exposes_send_and_sync_facts_without_value_promotion() {
    let capabilities =
        registered_type_capabilities::<SendSync>().expect("the concrete thread-safe registration must be compatible");

    assert!(capabilities.contains(send_key()));
    assert!(capabilities.contains(sync_key()));
    assert_eq!(capabilities.get(send_key()), None);
    assert_eq!(capabilities.get(sync_key()), None);
}

/// Confirms macro registration accepts a third-party typed key and adapter.
#[test]
fn test_concrete_registration_accepts_third_party_typed_adapter() {
    let capabilities =
        registered_type_capabilities::<ExtensionRegistration>().expect("the extension registration must be compatible");

    assert_eq!(capabilities.get(extension_key()), Some(&ExtensionAdapter("registered")));
}

/// Confirms matching IDs emitted by independent fragments remain an explicit
/// conflict.
#[test]
fn test_concrete_registration_rejects_cross_fragment_duplicate_capability() {
    let error = registered_type_capabilities::<DuplicateRegistration>()
        .expect_err("two linked fragments cannot silently claim one capability ID");

    assert_eq!(error.kind(), CapabilityConflictKind::DuplicateId);
    assert_eq!(error.id().as_str(), "qubit.reflect.clone");
}

/// Confirms reflected-type registration returns the existing descriptor root.
#[test]
fn test_reflected_type_registration_preserves_descriptor_root_identity() {
    let registered =
        registered_reflected_type::<ReflectedRoot>().expect("the exact concrete reflected type must be registered");

    assert!(std::ptr::eq(registered, TypeDescriptor::of::<ReflectedRoot>()));
}
