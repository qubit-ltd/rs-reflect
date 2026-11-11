// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use std::time::Instant;

/// Runs a unit test child process with a fixed timeout.
///
/// # Parameters
///
/// * `test_name` — exact test name passed to the child process.
/// * `environment` — environment variable that activates the child path.
fn run_bounded_unit_test_child(test_name: &str, environment: &str) {
    let mut child = Command::new(std::env::current_exe().expect("test executable path"))
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .env(environment, "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start isolated child test");
    let deadline = Instant::now() + Duration::from_secs(5);
    let timed_out = loop {
        if let Some(status) = child.try_wait().expect("poll child") {
            assert!(status.success(), "child exited unsuccessfully: {status}");
            break false;
        }
        if Instant::now() >= deadline {
            child.kill().expect("kill timed-out child");
            child.wait().expect("reap timed-out child");
            break true;
        }
        thread::sleep(Duration::from_millis(10));
    };
    let output = child.wait_with_output().expect("collect child output");
    assert!(
        !timed_out,
        "child test {test_name} exceeded its five-second deadline: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

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

mod context_fixture;
use context_fixture::ContextFixture;

/// First reflected type used by context tests.
type First = ContextFixture<0>;
/// Second reflected type used by context tests.
type Second = ContextFixture<1>;
/// Root type for linked registration tests.
type LinkedRoot = ContextFixture<2>;
/// Dependency type for linked registration tests.
type LinkedDependency = ContextFixture<3>;
/// Unrelated type excluded from a linked context.
type LinkedOutside = ContextFixture<4>;
/// Type whose linked registration has a missing dependency.
type LinkedMissing = ContextFixture<5>;
/// Root type whose target fragment does not include its type.
type RootWithoutTypeFragment = ContextFixture<6>;
/// Dependency type included by the root target fragment.
type RootFragmentDependency = ContextFixture<7>;
/// Type identifier with no matching registration fragment.
type MissingTarget = ContextFixture<8>;
/// Root used to verify selection-time factory execution.
type FactoryRoot = ContextFixture<12>;
/// Unrelated target whose factory must stay uncalled.
type FactoryOutside = ContextFixture<13>;
/// Selected dependency used to verify materialization order.
type FactoryDependency = ContextFixture<14>;

impl Reflect for First {
    /// Returns the static registration used by this test fixture.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<First> = TypeRegistration::new();
        &REGISTRATION
    }
}

impl Reflect for Second {
    /// Returns the static registration used by this test fixture.
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
reflect_fixture!(RootWithoutTypeFragment);
reflect_fixture!(RootFragmentDependency);
reflect_fixture!(FactoryRoot);
reflect_fixture!(FactoryOutside);
reflect_fixture!(FactoryDependency);

static LINKED_FACTORY_ROOT_CALLS: AtomicUsize = AtomicUsize::new(0);
static LINKED_FACTORY_OUTSIDE_CALLS: AtomicUsize = AtomicUsize::new(0);
static LINKED_FACTORY_DEPENDENCY_CALLS: AtomicUsize = AtomicUsize::new(0);
static LINKED_FACTORY_CALL_ORDER: Mutex<Vec<&str>> = Mutex::new(Vec::new());

fn linked_factory_root_registration() -> Registration {
    LINKED_FACTORY_ROOT_CALLS.fetch_add(1, Ordering::SeqCst);
    LINKED_FACTORY_CALL_ORDER.lock().unwrap().push("root");
    Registration::metadata_only()
}

fn linked_factory_outside_registration() -> Registration {
    LINKED_FACTORY_OUTSIDE_CALLS.fetch_add(1, Ordering::SeqCst);
    Registration::metadata_only()
}

fn linked_factory_dependency_registration() -> Registration {
    LINKED_FACTORY_DEPENDENCY_CALLS.fetch_add(1, Ordering::SeqCst);
    LINKED_FACTORY_CALL_ORDER.lock().unwrap().push("dependency");
    Registration::of::<FactoryDependency>()
}

fn factory_root_dependencies() -> &'static [std::any::TypeId] {
    static DEPENDENCIES: [std::any::TypeId; 1] = [std::any::TypeId::of::<FactoryDependency>()];
    &DEPENDENCIES
}

/// Returns the linked dependency declared by the root fixture.
fn linked_root_dependencies() -> &'static [std::any::TypeId] {
    static DEPENDENCIES: [std::any::TypeId; 1] = [std::any::TypeId::of::<LinkedDependency>()];
    &DEPENDENCIES
}
/// Returns the dependency used to exercise root type fallback.
fn root_fragment_dependencies() -> &'static [std::any::TypeId] {
    static DEPENDENCIES: [std::any::TypeId; 1] = [std::any::TypeId::of::<RootFragmentDependency>()];
    &DEPENDENCIES
}
/// Returns an empty dependency list for an isolated fixture.
fn no_linked_dependencies() -> &'static [std::any::TypeId] {
    &[]
}
/// Returns a type identifier with no corresponding registration fragment.
fn missing_target_dependency() -> &'static [std::any::TypeId] {
    static DEPENDENCIES: [std::any::TypeId; 1] = [std::any::TypeId::of::<MissingTarget>()];
    &DEPENDENCIES
}
/// Creates the registration for the linked root fixture.
fn linked_root_registration() -> Registration {
    Registration::of::<LinkedRoot>()
}
/// Creates the registration for the linked dependency fixture.
fn linked_dependency_registration() -> Registration {
    Registration::of::<LinkedDependency>()
}
/// Creates the registration for an unrelated linked fixture.
fn linked_outside_registration() -> Registration {
    Registration::of::<LinkedOutside>()
}
/// Creates the registration for a fixture with a missing dependency.
fn linked_missing_registration() -> Registration {
    Registration::of::<LinkedMissing>()
}
/// Creates metadata without a type record for the root application fragment.
fn root_fragment_registration() -> Registration {
    Registration::metadata_only()
}
/// Creates the type registration for the root fragment's dependency.
fn root_fragment_dependency_registration() -> Registration {
    Registration::of::<RootFragmentDependency>()
}

/// Marker identity for the generic struct definition fixture.
type GenericStructDefinitionMarker = ContextFixture<9>;
/// Marker identity for the generic enum definition fixture.
type GenericEnumDefinitionMarker = ContextFixture<10>;
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

/// Creates the generic struct definition registration used by context tests.
fn generic_struct_definition_registration() -> Registration {
    Registration::type_definition(&GENERIC_STRUCT_DEFINITION)
}

/// Creates the generic enum definition registration used by context tests.
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
inventory::submit! {
    RegistrationFragment::for_target::<RootWithoutTypeFragment>(
        ("tests", None, "root_without_type", None, None, None, "RootWithoutTypeFragment"),
        root_fragment_dependencies,
        root_fragment_registration,
    )
}
inventory::submit! {
    RegistrationFragment::for_target::<FactoryRoot>(
        ("tests", None, "factory", None, None, None, "z-factory-root"),
        factory_root_dependencies,
        linked_factory_root_registration,
    )
}
inventory::submit! {
    RegistrationFragment::for_type::<FactoryDependency>(
        ("tests", None, "factory", None, None, None, "a-factory-dependency"),
        no_linked_dependencies,
        linked_factory_dependency_registration,
    )
}
inventory::submit! {
    RegistrationFragment::for_target::<FactoryOutside>(
        ("tests", None, "factory_outside", None, None, None, "FactoryOutside"),
        no_linked_dependencies,
        linked_factory_outside_registration,
    )
}
inventory::submit! {
    RegistrationFragment::for_type::<RootFragmentDependency>(
        ("tests", None, "root_fragment_dependency", None, None, None, "RootFragmentDependency"),
        no_linked_dependencies,
        root_fragment_dependency_registration,
    )
}

/// Marker identity for the envelope type definition fixture.
type EnvelopeMarker = ContextFixture<11>;
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

/// Marker identity for the first shared trait definition fixture.
type SharedTraitMarkerA = ContextFixture<12>;
/// Marker identity for the second shared trait definition fixture.
type SharedTraitMarkerB = ContextFixture<13>;
/// Marker identity for a conflicting type definition fixture.
type ConflictingTypeDefinitionMarker = ContextFixture<14>;
/// Marker identity for a duplicate path type definition fixture.
type DuplicatePathTypeDefinitionMarker = ContextFixture<15>;

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

/// Creates the first shared-trait registration used by lookup tests.
fn shared_trait_a_registration() -> Registration {
    Registration::metadata_only()
        .with_trait_definition(&SHARED_TRAIT_A)
        .with_trait_application(&SHARED_TRAIT_APPLICATION)
}

/// Creates the second shared-trait registration used by lookup tests.
fn shared_trait_b_registration() -> Registration {
    Registration::metadata_only().with_trait_definition(&SHARED_TRAIT_B)
}

mod first;
mod second;

#[test]
fn test_metadata_only_fragments_register_definitions_without_a_type_record() {
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
fn test_explicit_contexts_are_isolated_and_frozen() {
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
fn test_descriptors_distinguish_registered_types_from_unindexed_metadata() {
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
fn test_definition_path_ambiguity_and_iteration_preserve_distinct_identities() {
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
fn test_same_trait_path_keeps_typed_links_and_reports_lookup_ambiguity() {
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
fn test_unsized_reflect_types_can_be_included_in_contexts() {
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
fn test_generic_definitions_and_instances_are_context_bound() {
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
fn test_linked_contexts_discover_generic_struct_and_enum_definitions_without_type_records() {
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
fn test_capability_registrations_are_frozen_per_context() {
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
fn test_linked_target_closure_includes_dependencies_and_excludes_unrelated_fragments() {
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
fn test_linked_target_closure_registers_root_without_type_fragment() {
    let context = ReflectionContext::builder()
        .include_linked_for::<RootWithoutTypeFragment>()
        .build()
        .expect("root and its dependency are registered");

    assert!(
        context
            .find_type_by_id(std::any::TypeId::of::<RootWithoutTypeFragment>())
            .expect("root lookup succeeds")
            .is_some()
    );
    assert!(
        context
            .find_type_by_id(std::any::TypeId::of::<RootFragmentDependency>())
            .expect("dependency lookup succeeds")
            .is_some()
    );
}

#[test]
fn test_linked_target_closure_reports_missing_dependency_before_publishing_context() {
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
fn test_linked_target_selection_runs_only_selected_factories_each_time() {
    LINKED_FACTORY_ROOT_CALLS.store(0, Ordering::SeqCst);
    LINKED_FACTORY_OUTSIDE_CALLS.store(0, Ordering::SeqCst);
    LINKED_FACTORY_DEPENDENCY_CALLS.store(0, Ordering::SeqCst);

    for expected_calls in [1, 2] {
        LINKED_FACTORY_CALL_ORDER.lock().unwrap().clear();
        ReflectionContext::builder()
            .include_linked_for::<FactoryRoot>()
            .build()
            .expect("factory fixture context builds");
        assert_eq!(LINKED_FACTORY_ROOT_CALLS.load(Ordering::SeqCst), expected_calls);
        assert_eq!(LINKED_FACTORY_DEPENDENCY_CALLS.load(Ordering::SeqCst), expected_calls);
        assert_eq!(LINKED_FACTORY_OUTSIDE_CALLS.load(Ordering::SeqCst), 0);
        assert_eq!(*LINKED_FACTORY_CALL_ORDER.lock().unwrap(), ["dependency", "root"],);
    }
}

#[test]
fn test_duplicate_type_registration_fails_without_publishing_a_context() {
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
fn test_lookup_candidates_have_stable_registration_order() {
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
fn test_global_initialization_caches_success_and_failure_without_holding_its_lock() {
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

#[test]
fn test_global_initialization_recursive_reentry_child() {
    if std::env::var_os("QUBIT_REFLECT_INTERNAL_REENTRY_CHILD").is_none() {
        return;
    }
    let context = GlobalContext::new();
    let panic = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        context.get_or_initialize(|| {
            let _ = context.get_or_initialize(|| Ok(crate::runtime::context::handle::data::ContextData::empty()));
            Ok(crate::runtime::context::handle::data::ContextData::empty())
        })
    })) {
        Ok(_) => panic!("recursive initialization must panic"),
        Err(panic) => panic,
    };
    assert!(
        panic
            .downcast_ref::<String>()
            .is_some_and(|message| { message == "recursive global reflection context initialization" })
            || panic
                .downcast_ref::<&str>()
                .is_some_and(|message| { *message == "recursive global reflection context initialization" }),
        "recursive initialization used an unexpected panic payload"
    );
    let _retried = context
        .get_or_initialize(|| Ok(crate::runtime::context::handle::data::ContextData::empty()))
        .expect("a recursive panic must reset the initializer for retry");
}

#[test]
fn test_global_initialization_recursive_reentry_is_bounded() {
    let mut child = Command::new(std::env::current_exe().expect("test executable path"))
        .arg("--exact")
        .arg("runtime::tests::context::test_global_initialization_recursive_reentry_child")
        .arg("--nocapture")
        .env("QUBIT_REFLECT_INTERNAL_REENTRY_CHILD", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start isolated child test");
    let deadline = Instant::now() + Duration::from_secs(5);
    let timed_out = loop {
        if let Some(status) = child.try_wait().expect("poll child") {
            assert!(status.success(), "child exited unsuccessfully: {status}");
            break false;
        }
        if Instant::now() >= deadline {
            child.kill().expect("kill timed-out child");
            child.wait().expect("reap timed-out child");
            break true;
        }
        thread::sleep(Duration::from_millis(10));
    };
    let output = child.wait_with_output().expect("collect child output");
    assert!(
        !timed_out
            && String::from_utf8_lossy(&output.stderr).contains("recursive global reflection context initialization"),
        "expected recursive initialization to panic with the fixed diagnostic before the deadline; timed_out={timed_out}, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_global_initialization_concurrent_callers_share_one_arc() {
    if std::env::var_os("QUBIT_REFLECT_CONCURRENT_INITIALIZATION_CHILD").is_none() {
        run_bounded_unit_test_child(
            "runtime::tests::context::test_global_initialization_concurrent_callers_share_one_arc",
            "QUBIT_REFLECT_CONCURRENT_INITIALIZATION_CHILD",
        );
        return;
    }
    let context = Arc::new(GlobalContext::new());
    let barrier = Arc::new(std::sync::Barrier::new(5));
    let calls = Arc::new(AtomicUsize::new(0));
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let release_rx = Arc::new(std::sync::Mutex::new(release_rx));
    let mut callers = Vec::new();
    for _ in 0..4 {
        let context = Arc::clone(&context);
        let barrier = Arc::clone(&barrier);
        let calls = Arc::clone(&calls);
        let entered_tx = entered_tx.clone();
        let release_rx = Arc::clone(&release_rx);
        callers.push(std::thread::spawn(move || {
            barrier.wait();
            entered_tx.send(()).expect("report caller ready");
            context
                .get_or_initialize(|| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    release_rx
                        .lock()
                        .expect("release receiver lock")
                        .recv()
                        .expect("release initializer");
                    Ok(crate::runtime::context::handle::data::ContextData::empty())
                })
                .expect("initialization succeeds")
        }));
    }
    barrier.wait();
    for _ in 0..4 {
        entered_rx.recv().expect("all callers reached initialization");
    }
    release_tx.send(()).expect("release the winning initializer");
    let results: Vec<_> = callers
        .into_iter()
        .map(|caller| caller.join().expect("caller thread completes"))
        .collect();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(results.iter().all(|result| Arc::ptr_eq(&results[0], result)));
}

#[test]
fn test_global_initialization_waiter_restarts_after_initializer_panic() {
    if std::env::var_os("QUBIT_REFLECT_PANIC_WAITER_CHILD").is_none() {
        run_bounded_unit_test_child(
            "runtime::tests::context::test_global_initialization_waiter_restarts_after_initializer_panic",
            "QUBIT_REFLECT_PANIC_WAITER_CHILD",
        );
        return;
    }
    let context = Arc::new(GlobalContext::new());
    let (initializer_entered_tx, initializer_entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let initializer_context = Arc::clone(&context);
    let initializer = std::thread::spawn(move || {
        initializer_context.get_or_initialize(|| {
            initializer_entered_tx.send(()).expect("report initializer started");
            release_rx.recv().expect("release panicking initializer");
            panic!("fixture initializer panic");
        })
    });
    initializer_entered_rx.recv().expect("initializer started");

    let waiter_context = Arc::clone(&context);
    let waiter = std::thread::spawn(move || {
        waiter_context
            .get_or_initialize(|| Ok(crate::runtime::context::handle::data::ContextData::empty()))
            .expect("waiter retries initialization after panic")
    });
    context.wait_for_waiters(1);
    release_tx.send(()).expect("release initializer");
    assert!(initializer.join().is_err(), "initializer panic propagates");
    let waiter_result = waiter.join().expect("waiter completes after notification");
    assert!(Arc::ptr_eq(
        &waiter_result,
        &context
            .get_or_initialize(|| panic!("waiter should cache successful retry"))
            .expect("retry result is cached")
    ));
}
