use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::runtime::context::handle::ReflectionContext;
use crate::runtime::context::handle::builder::ReflectionContextBuilder;
use crate::runtime::context::handle::global::GlobalContext;
use crate::runtime::context::handle::registration::Registration;
use crate::runtime::context::handle::registration::RegistrationFragment;
use crate::runtime::context::handle::registration::TypeDefinitionRegistration;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::generic::{
    GenericDefinitionMetadata, TypeDefinitionId, TypeDefinitionKind, TypeDefinitionMetadata,
};
use crate::runtime::descriptor::{Reflect, TypeRegistration};

#[derive(Clone)]
struct First;
struct Second;
struct LinkedRoot;
struct LinkedDependency;
struct LinkedOutside;
struct LinkedMissing;
struct MissingTarget;

impl Reflect for First {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<First> = TypeRegistration::new();
        &REGISTRATION
    }
}

impl Reflect for Second {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<Second> = TypeRegistration::new();
        &REGISTRATION
    }
}

macro_rules! reflect_fixture {
    ($type:ty) => {
        impl Reflect for $type {
            fn __reflect_registration() -> &'static TypeRegistration<Self> {
                static REGISTRATION: TypeRegistration<$type> = TypeRegistration::new();
                &REGISTRATION
            }
        }
    };
}
reflect_fixture!(LinkedRoot);
reflect_fixture!(LinkedDependency);
reflect_fixture!(LinkedOutside);
reflect_fixture!(LinkedMissing);

fn linked_root_dependencies() -> &'static [std::any::TypeId] {
    static DEPENDENCIES: [std::any::TypeId; 1] = [std::any::TypeId::of::<LinkedDependency>()];
    &DEPENDENCIES
}
fn no_linked_dependencies() -> &'static [std::any::TypeId] {
    &[]
}
fn missing_target_dependency() -> &'static [std::any::TypeId] {
    static DEPENDENCIES: [std::any::TypeId; 1] = [std::any::TypeId::of::<MissingTarget>()];
    &DEPENDENCIES
}
fn linked_root_registration() -> Registration {
    Registration::of::<LinkedRoot>()
}
fn linked_dependency_registration() -> Registration {
    Registration::of::<LinkedDependency>()
}
fn linked_outside_registration() -> Registration {
    Registration::of::<LinkedOutside>()
}
fn linked_missing_registration() -> Registration {
    Registration::of::<LinkedMissing>()
}

inventory::submit! {
    RegistrationFragment::for_type::<LinkedRoot>(
        ("tests", None, "linked_root", None, None, None, "LinkedRoot"),
        linked_root_dependencies,
        linked_root_registration,
    )
}
inventory::submit! {
    RegistrationFragment::for_type::<LinkedMissing>(
        ("tests", None, "linked_missing", None, None, None, "LinkedMissing"),
        missing_target_dependency,
        linked_missing_registration,
    )
}
inventory::submit! {
    RegistrationFragment::for_type::<LinkedDependency>(
        ("tests", None, "linked_dependency", None, None, None, "LinkedDependency"),
        no_linked_dependencies,
        linked_dependency_registration,
    )
}
inventory::submit! {
    RegistrationFragment::for_type::<LinkedOutside>(
        ("tests", None, "linked_outside", None, None, None, "LinkedOutside"),
        no_linked_dependencies,
        linked_outside_registration,
    )
}

struct EnvelopeMarker;
static EMPTY_GENERIC_DEFINITION: GenericDefinitionMetadata = GenericDefinitionMetadata {
    parameters: &[],
    predicates: &[],
};
static ENVELOPE_DEFINITION: TypeDefinitionMetadata = TypeDefinitionMetadata {
    id: TypeDefinitionId::of::<EnvelopeMarker>(),
    name: "Envelope",
    path: "tests::Envelope",
    kind: TypeDefinitionKind::Struct(crate::runtime::descriptor::generic::StructKind::Named),
    generics: &EMPTY_GENERIC_DEFINITION,
};

mod first {
    pub struct Thing;

    impl crate::runtime::descriptor::Reflect for Thing {
        fn __reflect_registration() -> &'static crate::runtime::descriptor::TypeRegistration<Self> {
            static REGISTRATION: crate::runtime::descriptor::TypeRegistration<Thing> =
                crate::runtime::descriptor::TypeRegistration::new();
            &REGISTRATION
        }
    }
}

mod second {
    pub struct Thing;

    impl crate::runtime::descriptor::Reflect for Thing {
        fn __reflect_registration() -> &'static crate::runtime::descriptor::TypeRegistration<Self> {
            static REGISTRATION: crate::runtime::descriptor::TypeRegistration<Thing> =
                crate::runtime::descriptor::TypeRegistration::new();
            &REGISTRATION
        }
    }
}

#[test]
fn metadata_only_fragments_register_definitions_without_a_type_record() {
    struct DefinitionMarker;
    static GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&[], &[]);
    static DEFINITION: TypeDefinitionRegistration<DefinitionMarker> =
        TypeDefinitionRegistration::new(
            "Envelope",
            "example::Envelope",
            TypeDefinitionKind::Struct(crate::runtime::descriptor::generic::StructKind::Named),
            &GENERICS,
        );

    fn registration() -> Registration {
        Registration::metadata_only().with_type_definition(&DEFINITION)
    }

    let fragment =
        RegistrationFragment::for_global_registration("envelope-definition", registration);
    let registration = fragment.materialize().expect("fragment is well-formed");
    assert!(registration.type_record().is_none());

    let context = ReflectionContextBuilder::new()
        .register(registration)
        .build()
        .expect("metadata-only registration builds a context");
    assert_eq!(context.registered_type_count().unwrap(), 0);
    assert!(
        context
            .type_definition(TypeDefinitionId::of::<DefinitionMarker>())
            .unwrap()
            .is_some()
    );
}

#[test]
fn explicit_contexts_are_isolated_and_frozen() {
    let first = ReflectionContext::builder()
        .include_type::<First>()
        .build()
        .expect("first context builds");
    let second = ReflectionContext::builder()
        .include_type::<Second>()
        .build()
        .expect("second context builds");

    assert_ne!(first.id(), second.id());
    assert!(first.is_initialized());
    assert!(
        first
            .find_type_by_id(std::any::TypeId::of::<First>())
            .expect("first lookup succeeds")
            .is_some()
    );
    assert!(
        first
            .find_type_by_id(std::any::TypeId::of::<Second>())
            .expect("second lookup succeeds")
            .is_none()
    );
    let known_type = first.type_of::<Second>();
    assert!(std::ptr::eq(known_type.context(), &first));
    assert!(
        first
            .find_type_by_id(std::any::TypeId::of::<Second>())
            .expect("second lookup succeeds")
            .is_none()
    );
    assert!(
        second
            .find_type_by_id(std::any::TypeId::of::<Second>())
            .expect("second lookup succeeds")
            .is_some()
    );
    assert!(
        second
            .find_type_by_id(std::any::TypeId::of::<First>())
            .expect("first lookup succeeds")
            .is_none()
    );
}

#[test]
fn generic_definitions_and_instances_are_context_bound() {
    let included = ReflectionContext::builder()
        .register(
            Registration::of::<First>()
                .with_type_definition_metadata(&ENVELOPE_DEFINITION)
                .with_type_instance_id(TypeDefinitionId::of::<EnvelopeMarker>()),
        )
        .build()
        .expect("context builds");
    let isolated = ReflectionContext::builder()
        .include_type::<First>()
        .build()
        .expect("isolated context builds");

    let definition = included
        .type_definition(TypeDefinitionId::of::<EnvelopeMarker>())
        .expect("context index reads")
        .expect("definition is registered");
    let instances = definition.instances().expect("instance callback succeeds");
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].type_id(), std::any::TypeId::of::<First>());
    assert!(
        isolated
            .type_definition(TypeDefinitionId::of::<EnvelopeMarker>())
            .expect("context index reads")
            .is_none()
    );
}

#[test]
fn capability_registrations_are_frozen_per_context() {
    use crate::runtime::capability::capability_origin::CapabilityOrigin;
    use crate::runtime::capability::capability_registration::CapabilityRegistration;

    let with_capability = ReflectionContext::builder()
        .register(
            Registration::of::<First>().with_capability(
                CapabilityRegistration::clone::<First>(CapabilityOrigin::Intrinsic)
                    .expect("core clone registration is valid"),
            ),
        )
        .build()
        .expect("context builds");
    let without_capability = ReflectionContext::builder()
        .include_type::<First>()
        .build()
        .expect("isolated context builds");

    let capabilities = with_capability
        .type_of::<First>()
        .capabilities()
        .expect("capability index reads")
        .expect("registered capability set exists");
    assert!(capabilities.get("qubit.reflect.clone").is_some());
    assert!(
        without_capability
            .type_of::<First>()
            .capabilities()
            .expect("capability index reads")
            .is_none()
    );
}

#[test]
fn linked_target_closure_includes_dependencies_and_excludes_unrelated_fragments() {
    let context = ReflectionContext::builder()
        .include_linked_for::<LinkedRoot>()
        .build()
        .expect("linked closure builds");
    let ids: Vec<_> = context
        .types()
        .expect("types are frozen")
        .into_vec()
        .into_iter()
        .map(TypeDescriptor::type_id)
        .collect();
    assert!(ids.contains(&std::any::TypeId::of::<LinkedRoot>()));
    assert!(ids.contains(&std::any::TypeId::of::<LinkedDependency>()));
    assert!(!ids.contains(&std::any::TypeId::of::<LinkedOutside>()));
}

#[test]
fn linked_target_closure_reports_missing_dependency_before_publishing_context() {
    let error = match ReflectionContext::builder()
        .include_linked_for::<LinkedMissing>()
        .build()
    {
        Ok(_) => panic!("missing linked dependency rejects context build"),
        Err(error) => error,
    };
    assert!(matches!(
        error.primary_issue(),
        crate::runtime::error::ContextIssue::MissingDependency { .. }
    ));
}

#[test]
fn duplicate_type_registration_fails_without_publishing_a_context() {
    let error = ReflectionContext::builder()
        .include_type::<First>()
        .include_type::<First>()
        .build()
        .err()
        .expect("duplicate type registration fails");

    assert!(matches!(
        error.primary_issue(),
        crate::runtime::error::ContextIssue::DuplicateTypeIdentity {
            type_id,
            registrations,
        } if *type_id == std::any::TypeId::of::<First>() && registrations.len() == 2
    ));
}

#[test]
fn lookup_candidates_have_stable_registration_order() {
    let context = ReflectionContext::builder()
        .include_type::<second::Thing>()
        .include_type::<first::Thing>()
        .build()
        .expect("context builds");

    assert!(matches!(
        context
            .lookup_type_by_short_name("Missing")
            .expect("lookup succeeds"),
        crate::runtime::lookup::Lookup::Missing
    ));
    let crate::runtime::lookup::Lookup::Ambiguous(candidates) = context
        .lookup_type_by_short_name("Thing")
        .expect("lookup succeeds")
    else {
        panic!("both same short names should be returned as ambiguous");
    };
    let candidate_ids: Vec<_> = candidates
        .iter()
        .map(|candidate| candidate.type_id())
        .collect();
    assert_eq!(
        candidate_ids,
        [
            std::any::TypeId::of::<first::Thing>(),
            std::any::TypeId::of::<second::Thing>(),
        ]
    );
    let error = context
        .type_by_short_name("Thing")
        .err()
        .expect("strict lookup rejects ambiguity");
    assert!(matches!(
        error,
        crate::runtime::error::LookupError::Ambiguous { candidates, .. } if candidates.len() == 2
    ));
}

#[test]
fn global_initialization_caches_success_and_failure_without_holding_its_lock() {
    let successful = GlobalContext::new();
    assert!(!successful.is_initialized());
    let success_calls = AtomicUsize::new(0);
    let success = successful
        .get_or_initialize(|| {
            assert!(
                successful.state_lock_is_available(),
                "initializer must run outside the state lock"
            );
            success_calls.fetch_add(1, Ordering::SeqCst);
            Ok(crate::runtime::context::handle::data::ContextData::empty())
        })
        .expect("initialization succeeds");
    let cached_success = successful
        .get_or_initialize(|| panic!("cached success must not invoke the initializer"))
        .expect("success is cached");
    assert!(Arc::ptr_eq(&success, &cached_success));
    assert!(successful.is_initialized());
    assert_eq!(success_calls.load(Ordering::SeqCst), 1);

    let failed = GlobalContext::new();
    assert!(!failed.is_initialized());
    let failure_calls = AtomicUsize::new(0);
    let first_error = failed
        .get_or_initialize(|| {
            failure_calls.fetch_add(1, Ordering::SeqCst);
            Err(crate::runtime::error::ContextError::new(
                "fixture validation failed",
            ))
        })
        .err()
        .expect("initialization fails");
    let second_error = failed
        .get_or_initialize(|| panic!("cached failure must not invoke the initializer"))
        .err()
        .expect("failure is cached");
    assert_eq!(first_error.message(), second_error.message());
    assert!(failed.is_initialized());
    assert_eq!(failure_calls.load(Ordering::SeqCst), 1);
}
