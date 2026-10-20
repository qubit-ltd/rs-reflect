// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::runtime::context::handle::ReflectionContext;
use crate::runtime::context::handle::builder::ReflectionContextBuilder;
use crate::runtime::context::handle::global::GlobalContext;
use crate::runtime::context::handle::registration::Registration;
use crate::runtime::context::handle::registration::RegistrationFragment;
use crate::runtime::context::handle::registration::TypeDefinitionRegistration;
use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::TypeRegistration;
use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
use crate::runtime::descriptor::generic::TypeDefinitionId;
use crate::runtime::descriptor::generic::TypeDefinitionKind;
use crate::runtime::descriptor::generic::TypeDefinitionMetadata;
use crate::runtime::descriptor::trait_descriptor::SupertraitCompleteness;
use crate::runtime::descriptor::trait_descriptor::TraitApplicationMetadata;
use crate::runtime::descriptor::trait_descriptor::TraitCompleteness;
use crate::runtime::descriptor::trait_descriptor::TraitDefinitionId;
use crate::runtime::descriptor::trait_descriptor::TraitDefinitionMetadata;
use crate::runtime::descriptor::trait_descriptor::Visibility;
use crate::runtime::expression::TraitExpressionData;

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

struct GenericStructDefinitionMarker;
struct GenericEnumDefinitionMarker;
static GENERIC_TYPE_PARAMETERS: [crate::runtime::descriptor::generic::GenericParameterMetadata; 1] =
    [crate::runtime::descriptor::generic::GenericParameterMetadata::type_parameter("T", 0, &[], None)];
static GENERIC_DEFINITION_METADATA: GenericDefinitionMetadata =
    GenericDefinitionMetadata::new(&GENERIC_TYPE_PARAMETERS, &[]);
static GENERIC_STRUCT_DEFINITION: TypeDefinitionRegistration<GenericStructDefinitionMarker> =
    TypeDefinitionRegistration::new(
        "GenericStruct",
        "tests::GenericStruct",
        TypeDefinitionKind::Struct(crate::runtime::descriptor::generic::StructKind::Named),
        &GENERIC_DEFINITION_METADATA,
    );
static GENERIC_ENUM_DEFINITION: TypeDefinitionRegistration<GenericEnumDefinitionMarker> =
    TypeDefinitionRegistration::new(
        "GenericEnum",
        "tests::GenericEnum",
        TypeDefinitionKind::Enum,
        &GENERIC_DEFINITION_METADATA,
    );

fn generic_struct_definition_registration() -> Registration {
    Registration::type_definition(&GENERIC_STRUCT_DEFINITION)
}

fn generic_enum_definition_registration() -> Registration {
    Registration::type_definition(&GENERIC_ENUM_DEFINITION)
}

inventory::submit! {
    RegistrationFragment::for_type_definition(
        "GenericStruct-definition",
        generic_struct_definition_registration,
    )
}
inventory::submit! {
    RegistrationFragment::for_type_definition(
        "GenericEnum-definition",
        generic_enum_definition_registration,
    )
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

struct SharedTraitMarkerA;
struct SharedTraitMarkerB;
struct ConflictingTypeDefinitionMarker;
struct DuplicatePathTypeDefinitionMarker;

static SHARED_TRAIT_A: TraitDefinitionMetadata = TraitDefinitionMetadata::new::<SharedTraitMarkerA>(
    "Shared",
    "tests::shared::Shared",
    Visibility::Public,
    TraitCompleteness::Complete,
    &EMPTY_GENERIC_DEFINITION,
    &[],
    &[],
    &[],
    &[],
);
static SHARED_TRAIT_B: TraitDefinitionMetadata = TraitDefinitionMetadata::new::<SharedTraitMarkerB>(
    "Shared",
    "tests::shared::Shared",
    Visibility::Public,
    TraitCompleteness::Complete,
    &EMPTY_GENERIC_DEFINITION,
    &[],
    &[],
    &[],
    &[],
);
static SHARED_TRAIT_EXPRESSION: TraitExpressionData = TraitExpressionData::new("tests::shared::Shared", &[], &[]);
static SHARED_TRAIT_APPLICATION: TraitApplicationMetadata = TraitApplicationMetadata::linked(
    &SHARED_TRAIT_A,
    &SHARED_TRAIT_EXPRESSION,
    &[],
    &[],
    &[],
    &[],
    SupertraitCompleteness::Complete,
);
static CONFLICTING_TYPE_DEFINITION_A: TypeDefinitionMetadata = TypeDefinitionMetadata {
    id: TypeDefinitionId::of::<ConflictingTypeDefinitionMarker>(),
    name: "ConflictA",
    path: "tests::ConflictA",
    kind: TypeDefinitionKind::Struct(crate::runtime::descriptor::generic::StructKind::Named),
    generics: &EMPTY_GENERIC_DEFINITION,
};
static CONFLICTING_TYPE_DEFINITION_B: TypeDefinitionMetadata = TypeDefinitionMetadata {
    id: TypeDefinitionId::of::<ConflictingTypeDefinitionMarker>(),
    name: "ConflictB",
    path: "tests::ConflictB",
    kind: TypeDefinitionKind::Struct(crate::runtime::descriptor::generic::StructKind::Named),
    generics: &EMPTY_GENERIC_DEFINITION,
};
static DUPLICATE_PATH_TYPE_DEFINITION: TypeDefinitionMetadata = TypeDefinitionMetadata {
    id: TypeDefinitionId::of::<DuplicatePathTypeDefinitionMarker>(),
    name: "GenericStructAlias",
    path: "tests::GenericStruct",
    kind: TypeDefinitionKind::Struct(crate::runtime::descriptor::generic::StructKind::Named),
    generics: &EMPTY_GENERIC_DEFINITION,
};

fn shared_trait_a_registration() -> Registration {
    Registration::metadata_only()
        .with_trait_definition(&SHARED_TRAIT_A)
        .with_trait_application(&SHARED_TRAIT_APPLICATION)
}

fn shared_trait_b_registration() -> Registration {
    Registration::metadata_only().with_trait_definition(&SHARED_TRAIT_B)
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
fn metadata_only_fragments_register_definitions_without_a_type_record() {
    struct DefinitionMarker;
    static GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&[], &[]);
    static DEFINITION: TypeDefinitionRegistration<DefinitionMarker> = TypeDefinitionRegistration::new(
        "Envelope",
        "example::Envelope",
        TypeDefinitionKind::Struct(crate::runtime::descriptor::generic::StructKind::Named),
        &GENERICS,
    );

    fn registration() -> Registration {
        Registration::metadata_only().with_type_definition(&DEFINITION)
    }

    let fragment = RegistrationFragment::for_global_registration("envelope-definition", registration);
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
fn descriptors_distinguish_registered_types_from_unindexed_metadata() {
    let context = ReflectionContext::builder()
        .include_type::<First>()
        .build()
        .expect("context builds");
    let indexed = context
        .find_type_by_id(std::any::TypeId::of::<First>())
        .expect("registered lookup succeeds")
        .expect("First is indexed");
    let unindexed = TypeDescriptor::of_in::<Second>(&context);

    assert!(indexed.is_registered().expect("registry read succeeds"));
    assert!(!unindexed.is_registered().expect("registry read succeeds"));
    assert!(context.find_type_by_id(unindexed.type_id()).unwrap().is_none());
    assert!(std::ptr::eq(unindexed.context(), &context));

    let types = context.types().expect("type collection is available");
    assert_eq!(types.len(), 1);
    assert!(!types.is_empty());
    assert_eq!(types.get(0).unwrap().type_id(), indexed.type_id());
    assert!(types.get(1).is_none());
    let mut iter = types.iter();
    assert_eq!(iter.len(), 1);
    assert_eq!(iter.next().unwrap().type_id(), indexed.type_id());
    assert_eq!(iter.len(), 0);
    assert!(iter.next().is_none());
}

#[test]
fn definition_path_ambiguity_and_iteration_preserve_distinct_identities() {
    let context = ReflectionContext::builder()
        .register(Registration::type_definition(&GENERIC_STRUCT_DEFINITION))
        .register(Registration::type_definition(&GENERIC_ENUM_DEFINITION))
        .register(Registration::metadata_only().with_type_definition_metadata(&CONFLICTING_TYPE_DEFINITION_A))
        .register(Registration::metadata_only().with_type_definition_metadata(&CONFLICTING_TYPE_DEFINITION_B));
    // The two definitions sharing an identity are rejected before a context
    // can expose partially built indexes.
    let conflict = context
        .build()
        .err()
        .expect("conflicting definition identity is rejected");
    assert!(matches!(
        conflict.primary_issue(),
        crate::runtime::error::ContextIssue::ConflictingTypeDefinition { registrations, .. }
            if registrations.len() == 2
    ));

    let context = ReflectionContext::builder()
        .register(Registration::type_definition(&GENERIC_STRUCT_DEFINITION))
        .register(Registration::type_definition(&GENERIC_ENUM_DEFINITION))
        .build()
        .expect("distinct definitions build");
    let definitions = context.type_definitions().expect("definitions are indexed");
    assert_eq!(definitions.len(), 2);
    assert_eq!(
        definitions
            .iter()
            .map(|definition| definition.path())
            .collect::<Vec<_>>(),
        ["tests::GenericEnum", "tests::GenericStruct"]
    );
    assert_eq!(definitions.get(0).unwrap().path(), "tests::GenericEnum");
    assert!(definitions.get(2).is_none());
    assert!(matches!(
        context.lookup_type_definition_by_name("missing::Definition").unwrap(),
        crate::runtime::lookup::Lookup::Missing
    ));

    let ambiguous = ReflectionContext::builder()
        .register(Registration::type_definition(&GENERIC_STRUCT_DEFINITION))
        .register(Registration::metadata_only().with_type_definition_metadata(&DUPLICATE_PATH_TYPE_DEFINITION))
        .build()
        .expect("different definition identities can share a source path");
    assert!(matches!(
        ambiguous
            .lookup_type_definition_by_name("tests::GenericStruct")
            .unwrap(),
        crate::runtime::lookup::Lookup::Ambiguous(candidates) if candidates.len() == 2
    ));
    assert!(matches!(
        ambiguous.type_definition_by_name("tests::GenericStruct"),
        Err(crate::runtime::error::LookupError::Ambiguous { candidates, .. })
            if candidates.len() == 2
    ));
}

#[test]
fn same_trait_path_keeps_typed_links_and_reports_lookup_ambiguity() {
    let context = ReflectionContext::builder()
        .register(shared_trait_a_registration())
        .register(shared_trait_b_registration())
        .build()
        .expect("same source path may belong to distinct trait identities");

    let lookup = context
        .lookup_trait_definition_by_name("tests::shared::Shared")
        .expect("trait path lookup succeeds");
    let crate::runtime::lookup::Lookup::Ambiguous(candidates) = lookup else {
        panic!("distinct trait identities with one source path stay ambiguous");
    };
    assert_eq!(candidates.len(), 2);
    let error = match context.trait_definition_by_name("tests::shared::Shared") {
        Err(error) => error,
        Ok(_) => panic!("strict lookup rejects a shared path"),
    };
    assert!(matches!(
        error,
        crate::runtime::error::LookupError::Ambiguous {
            subject: crate::runtime::error::LookupSubject::TraitDefinition,
            candidates,
            ..
        } if candidates.len() == 2
    ));

    let trait_a = context
        .trait_definition_for::<SharedTraitMarkerA>()
        .expect("typed identity lookup succeeds")
        .expect("trait A is retained");
    let trait_b = context
        .trait_definition_for::<SharedTraitMarkerB>()
        .expect("typed identity lookup succeeds")
        .expect("trait B is retained");
    assert_eq!(trait_a.path(), trait_b.path());
    assert_eq!(trait_a.completeness(), TraitCompleteness::Complete);

    let applications = context.trait_applications().expect("applications are iterable");
    assert_eq!(applications.len(), 1);
    let linked_definition = applications.get(0).unwrap().definition().unwrap();
    assert_eq!(linked_definition.path(), trait_a.path());
    assert_eq!(linked_definition.id(), TraitDefinitionId::of::<SharedTraitMarkerA>());
    assert_eq!(
        context
            .trait_applications_by_path("tests::shared::Shared")
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn unsized_reflect_types_can_be_included_in_contexts() {
    let explicitly_included = ReflectionContext::builder()
        .include_type::<str>()
        .build()
        .expect("context with an explicitly included unsized type builds");
    assert!(
        explicitly_included
            .find_type_by_id(std::any::TypeId::of::<str>())
            .expect("unsized type lookup succeeds")
            .is_some()
    );
    assert!(std::ptr::eq(
        explicitly_included.type_of::<str>().context(),
        &explicitly_included
    ));

    let linked_for_unsized = ReflectionContext::builder()
        .include_linked_for::<str>()
        .build()
        .expect("context rooted at an unsized type builds");
    assert!(
        linked_for_unsized
            .find_type_by_id(std::any::TypeId::of::<str>())
            .expect("linked unsized type lookup succeeds")
            .is_some()
    );
    assert!(std::ptr::eq(
        linked_for_unsized.type_of::<str>().context(),
        &linked_for_unsized
    ));
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
fn linked_contexts_discover_generic_struct_and_enum_definitions_without_type_records() {
    let context = ReflectionContext::builder()
        .include_linked_for::<LinkedRoot>()
        .build()
        .expect("global generic definition metadata is valid in linked contexts");

    let generic_struct = context
        .type_definition_by_name("tests::GenericStruct")
        .expect("generic struct definition lookup succeeds");
    let generic_enum = context
        .type_definition_by_name("tests::GenericEnum")
        .expect("generic enum definition lookup succeeds");

    assert_eq!(context.registered_type_count().unwrap(), 2);
    assert_eq!(generic_struct.name(), "GenericStruct");
    assert_eq!(
        generic_struct.kind(),
        TypeDefinitionKind::Struct(crate::runtime::descriptor::generic::StructKind::Named,)
    );
    assert_eq!(generic_struct.generics().parameters().len(), 1);
    assert_eq!(
        generic_struct
            .generics()
            .parameter("T")
            .expect("type parameter is retained")
            .name(),
        "T"
    );
    assert_eq!(generic_enum.name(), "GenericEnum");
    assert_eq!(generic_enum.kind(), TypeDefinitionKind::Enum);
    assert_eq!(generic_enum.generics().parameters().len(), 1);
}

#[test]
fn capability_registrations_are_frozen_per_context() {
    use crate::runtime::capability::capability_registration::CapabilityRegistration;
    use crate::runtime::capability::capability_registration_origin::CapabilityRegistrationOrigin;

    let with_capability = ReflectionContext::builder()
        .register(
            Registration::of::<First>().with_capability(
                CapabilityRegistration::clone::<First>(CapabilityRegistrationOrigin::Explicit(
                    crate::runtime::identity::RegistrationOrigin {
                        crate_name: "context-tests",
                        crate_version: None,
                        module_path: "capability_registrations_are_frozen_per_context",
                        file: None,
                        line: None,
                        column: None,
                    },
                ))
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
        .expect("capability index reads");
    assert!(capabilities.get("qubit.reflect.clone").is_some());
    assert!(
        without_capability
            .type_of::<First>()
            .capabilities()
            .expect("capability index reads")
            .is_empty()
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
        .iter()
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
    assert_eq!(error.issues().len(), 1);
    assert_eq!(error.message(), "reflection context validation failed");
    assert!(error.to_string().contains("requires missing"));
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
    assert_eq!(error.issues().len(), 1);
    assert_eq!(error.message(), "reflection context validation failed");
    assert_eq!(error, error.clone());
    assert!(error.to_string().contains("duplicate type identity"));
}

#[test]
fn lookup_candidates_have_stable_registration_order() {
    let context = ReflectionContext::builder()
        .include_type::<second::Thing>()
        .include_type::<first::Thing>()
        .build()
        .expect("context builds");

    assert!(matches!(
        context.lookup_type_by_short_name("Missing").expect("lookup succeeds"),
        crate::runtime::lookup::Lookup::Missing
    ));
    let crate::runtime::lookup::Lookup::Ambiguous(candidates) =
        context.lookup_type_by_short_name("Thing").expect("lookup succeeds")
    else {
        panic!("both same short names should be returned as ambiguous");
    };
    let candidate_ids: Vec<_> = candidates.iter().map(|candidate| candidate.type_id()).collect();
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
            Err(crate::runtime::error::ContextError::new("fixture validation failed"))
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
