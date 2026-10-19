use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::runtime::context::handle::ReflectionContext;
use crate::runtime::context::handle::global::GlobalContext;
use crate::runtime::descriptor::{Reflect, TypeRegistration};

struct First;
struct Second;

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
