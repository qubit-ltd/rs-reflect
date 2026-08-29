// qubit-style: allow explicit-imports
//! Integration tests for the immutable distributed reflection registry.
use std::any::TypeId;
use std::sync::LazyLock;
use std::sync::OnceLock;

use qubit_reflect as reflect;
use reflect::__private::registration::CapabilityRegistration;
use reflect::__private::registration::FragmentKind;
use reflect::__private::registration::FragmentPayload;
use reflect::__private::registration::RegistrationFragment;
use reflect::__private::registration::RuntimeIdentity;
use reflect::__private::registration::StaticFragmentIdentity;
use reflect::__private::registration::build_registry;
use reflect::__private::registration::initialize_registry;
use reflect::Reflect;
use reflect::capability::CapabilityDescriptor;
use reflect::capability::CapabilityKey;
use reflect::descriptor::ImplDefinitionDescriptor;
use reflect::descriptor::ImplDescriptor;
use reflect::descriptor::ImplKind;
use reflect::descriptor::TraitCompleteness;
use reflect::descriptor::TraitDefinitionDescriptor;
use reflect::descriptor::TraitDescriptor;
use reflect::descriptor::TraitId;
use reflect::descriptor::TypeDescriptor;
use reflect::error::RegistryErrorKind;
use reflect::expression::DiagnosticText;
use reflect::expression::GenericDefinitionDescriptor;
use reflect::expression::GenericParameterDescriptor;
use reflect::expression::TypeExpression;
use reflect::identity::CapabilityId;
use reflect::identity::ExternalTraitId;
use reflect::identity::FragmentIdentity;
use reflect::registry::ReflectRegistry;

struct EarlyType;
struct LateType;
struct IndependentType;
struct CapabilityTarget;

static EARLY_DESCRIPTOR: TypeDescriptor = reflect::__private::descriptor::opaque_root::<EarlyType>("shared-query");
static LATE_DESCRIPTOR: TypeDescriptor = reflect::__private::descriptor::opaque_root::<LateType>("shared-query");
static INDEPENDENT_DESCRIPTOR: TypeDescriptor =
    reflect::__private::descriptor::opaque_root::<IndependentType>("independent");

impl Reflect for IndependentType {
    /// Returns a descriptor whose structural query is independent of the
    /// registry.
    fn type_descriptor() -> &'static TypeDescriptor {
        &INDEPENDENT_DESCRIPTOR
    }
}

/// Builds deterministic static identity facts for a test declaration.
const fn static_identity(
    module: &'static str,
    line: u32,
    member_kind: &'static str,
    fingerprint: u64,
) -> StaticFragmentIdentity {
    StaticFragmentIdentity::new("registry-fixture", module, line, 1, member_kind, fingerprint)
}

const EARLY_IDENTITY: StaticFragmentIdentity = static_identity("a_early", 10, "type", 10);
const LATE_IDENTITY: StaticFragmentIdentity = static_identity("z_late", 20, "type", 20);

/// Returns the runtime identity of the early reflected type.
fn early_runtime_identity() -> RuntimeIdentity {
    RuntimeIdentity::Type(TypeId::of::<EarlyType>())
}

/// Builds the early reflected type payload.
fn early_payload() -> FragmentPayload {
    FragmentPayload::Type(&EARLY_DESCRIPTOR)
}

/// Returns the runtime identity of the late reflected type.
fn late_runtime_identity() -> RuntimeIdentity {
    RuntimeIdentity::Type(TypeId::of::<LateType>())
}

/// Builds the late reflected type payload.
fn late_payload() -> FragmentPayload {
    FragmentPayload::Type(&LATE_DESCRIPTOR)
}

static EARLY_FRAGMENT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Type,
    EARLY_IDENTITY,
    early_runtime_identity,
    early_payload,
);
static LATE_FRAGMENT: RegistrationFragment =
    RegistrationFragment::new(FragmentKind::Type, LATE_IDENTITY, late_runtime_identity, late_payload);

reflect::__private::inventory::submit! {
    LATE_FRAGMENT
}
reflect::__private::inventory::submit! {
    EARLY_FRAGMENT
}

const DUPLICATE_IDENTITY: StaticFragmentIdentity = static_identity("duplicate", 30, "type", 30);

static DUPLICATE_LEFT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Type,
    DUPLICATE_IDENTITY,
    early_runtime_identity,
    early_payload,
);
static DUPLICATE_RIGHT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Type,
    DUPLICATE_IDENTITY,
    late_runtime_identity,
    late_payload,
);

const FIRST_CONTENT_IDENTITY: StaticFragmentIdentity = static_identity("content", 40, "type", 1);
const SECOND_CONTENT_IDENTITY: StaticFragmentIdentity = static_identity("content", 40, "type", 2);

static CONTENT_LEFT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Type,
    FIRST_CONTENT_IDENTITY,
    early_runtime_identity,
    early_payload,
);
static CONTENT_RIGHT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Type,
    SECOND_CONTENT_IDENTITY,
    late_runtime_identity,
    late_payload,
);

const FIRST_CAPABILITY_IDENTITY: StaticFragmentIdentity = static_identity("capability_a", 50, "capability", 1);
const SECOND_CAPABILITY_IDENTITY: StaticFragmentIdentity = static_identity("capability_b", 51, "capability", 2);

/// Creates the shared capability ID used to provoke a cross-fragment conflict.
fn shared_capability_id() -> CapabilityId {
    CapabilityId::new("fixture.registry.shared").expect("the fixture capability ID must be valid")
}

/// Returns the audited runtime identity of the shared capability.
fn capability_runtime_identity() -> RuntimeIdentity {
    RuntimeIdentity::Capability {
        target_type_id: TypeId::of::<CapabilityTarget>(),
        capability_id: shared_capability_id(),
    }
}

/// Builds the first capability payload.
fn first_capability_payload() -> FragmentPayload {
    let key = CapabilityKey::<u8>::new(shared_capability_id());
    FragmentPayload::Capability(CapabilityRegistration::new(
        TypeId::of::<CapabilityTarget>(),
        CapabilityDescriptor::without_adapter(key),
    ))
}

/// Builds a payload that assigns a different contract to the same capability
/// ID.
fn second_capability_payload() -> FragmentPayload {
    let key = CapabilityKey::<u16>::new(shared_capability_id());
    FragmentPayload::Capability(CapabilityRegistration::new(
        TypeId::of::<CapabilityTarget>(),
        CapabilityDescriptor::without_adapter(key),
    ))
}

static CAPABILITY_LEFT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Capability,
    FIRST_CAPABILITY_IDENTITY,
    capability_runtime_identity,
    first_capability_payload,
);
static CAPABILITY_RIGHT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Capability,
    SECOND_CAPABILITY_IDENTITY,
    capability_runtime_identity,
    second_capability_payload,
);

static EMPTY_GENERIC_DEFINITION: LazyLock<GenericDefinitionDescriptor> =
    LazyLock::new(|| GenericDefinitionDescriptor {
        parameters: Box::default(),
        predicates: Box::default(),
        diagnostic: DiagnosticText::default(),
    });
static ONE_TYPE_PARAMETER: LazyLock<GenericDefinitionDescriptor> = LazyLock::new(|| GenericDefinitionDescriptor {
    parameters: vec![GenericParameterDescriptor::Type {
        name: "T".into(),
        bounds: Box::default(),
        default: None,
        diagnostic: DiagnosticText::default(),
    }]
    .into_boxed_slice(),
    predicates: Box::default(),
    diagnostic: DiagnosticText::default(),
});

static EXTERNAL_DEFINITION_LEFT: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
    TraitDefinitionDescriptor::new(
        TraitId::External(shared_external_trait_id()),
        "Shared",
        "fixture::left::Shared",
        "shared",
        TraitCompleteness::ExternalIncomplete,
        &EMPTY_GENERIC_DEFINITION,
    )
});
static EXTERNAL_DEFINITION_RIGHT: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
    TraitDefinitionDescriptor::new(
        TraitId::External(shared_external_trait_id()),
        "Shared",
        "fixture::right::Shared",
        "shared",
        TraitCompleteness::ExternalIncomplete,
        &EMPTY_GENERIC_DEFINITION,
    )
});
static EXTERNAL_DEFINITION_CONFLICT: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
    TraitDefinitionDescriptor::new(
        TraitId::External(shared_external_trait_id()),
        "Shared",
        "fixture::left::Shared",
        "shared",
        TraitCompleteness::ExternalIncomplete,
        &ONE_TYPE_PARAMETER,
    )
});
/// Creates the shared external-trait ID used to provoke a path conflict.
fn shared_external_trait_id() -> ExternalTraitId {
    ExternalTraitId::new("fixture.registry.external").expect("the fixture external trait ID must be valid")
}

const FIRST_EXTERNAL_IDENTITY: StaticFragmentIdentity = static_identity("external_a", 60, "trait", 1);
const SECOND_EXTERNAL_IDENTITY: StaticFragmentIdentity = static_identity("external_b", 61, "trait", 2);
const CONFLICTING_EXTERNAL_IDENTITY: StaticFragmentIdentity = static_identity("external_c", 62, "trait", 3);

/// Returns the shared runtime external-trait identity.
fn external_runtime_identity() -> RuntimeIdentity {
    RuntimeIdentity::Trait(TraitId::External(shared_external_trait_id()))
}

/// Builds the first external-trait payload.
fn first_external_payload() -> FragmentPayload {
    FragmentPayload::Trait(&EXTERNAL_DEFINITION_LEFT)
}

/// Builds the second external-trait payload.
fn second_external_payload() -> FragmentPayload {
    FragmentPayload::Trait(&EXTERNAL_DEFINITION_RIGHT)
}

/// Builds incompatible generic declaration facts for the shared external ID.
fn conflicting_external_payload() -> FragmentPayload {
    FragmentPayload::Trait(&EXTERNAL_DEFINITION_CONFLICT)
}

static EXTERNAL_LEFT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Trait,
    FIRST_EXTERNAL_IDENTITY,
    external_runtime_identity,
    first_external_payload,
);
static EXTERNAL_RIGHT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Trait,
    SECOND_EXTERNAL_IDENTITY,
    external_runtime_identity,
    second_external_payload,
);
static EXTERNAL_CONFLICT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Trait,
    CONFLICTING_EXTERNAL_IDENTITY,
    external_runtime_identity,
    conflicting_external_payload,
);

/// Resolves the descriptor used by the impl registration fixtures.
fn independent_descriptor() -> &'static TypeDescriptor {
    &INDEPENDENT_DESCRIPTOR
}

static IMPL_DEFINITION: LazyLock<ImplDefinitionDescriptor> = LazyLock::new(|| {
    ImplDefinitionDescriptor::new(
        FragmentIdentity::new("registry-fixture", "impl", 70, 1, "impl", 1),
        TypeExpression::Parameter("Self".into()),
        ImplKind::Inherent,
        None,
        &EMPTY_GENERIC_DEFINITION,
    )
    .expect("the inherent impl fixture definition must be valid")
});
static IMPL_DESCRIPTOR: LazyLock<ImplDescriptor> = LazyLock::new(|| {
    ImplDescriptor::builder(&IMPL_DEFINITION, independent_descriptor)
        .build()
        .expect("the inherent impl fixture must be valid")
});

const IMPL_IDENTITY: StaticFragmentIdentity = static_identity("impl", 70, "impl", 1);
const MISMATCHED_IMPL_IDENTITY: StaticFragmentIdentity = static_identity("impl_duplicate", 71, "impl", 2);

/// Returns the concrete target identity of the impl fixture.
fn impl_runtime_identity() -> RuntimeIdentity {
    RuntimeIdentity::Impl(TypeId::of::<IndependentType>())
}

/// Builds the shared impl payload used to test registration identity auditing.
fn impl_payload() -> FragmentPayload {
    FragmentPayload::Impl(&IMPL_DESCRIPTOR)
}

static IMPL_FRAGMENT: RegistrationFragment =
    RegistrationFragment::new(FragmentKind::Impl, IMPL_IDENTITY, impl_runtime_identity, impl_payload);
static MISMATCHED_IMPL_FRAGMENT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Impl,
    MISMATCHED_IMPL_IDENTITY,
    impl_runtime_identity,
    impl_payload,
);

static APPLIED_EXTERNAL_TRAIT: LazyLock<TraitDescriptor> = LazyLock::new(|| {
    TraitDescriptor::builder(&EXTERNAL_DEFINITION_LEFT)
        .build()
        .expect("the applied external trait fixture must be valid")
});
static TRAIT_IMPL_DEFINITION_LEFT: LazyLock<ImplDefinitionDescriptor> = LazyLock::new(|| {
    ImplDefinitionDescriptor::new(
        FragmentIdentity::new("registry-fixture", "trait_impl_a", 80, 1, "impl", 1),
        TypeExpression::Parameter("Self".into()),
        ImplKind::Trait,
        Some(&EXTERNAL_DEFINITION_LEFT),
        &EMPTY_GENERIC_DEFINITION,
    )
    .expect("the first trait impl definition must be valid")
});
static TRAIT_IMPL_DEFINITION_RIGHT: LazyLock<ImplDefinitionDescriptor> = LazyLock::new(|| {
    ImplDefinitionDescriptor::new(
        FragmentIdentity::new("registry-fixture", "trait_impl_b", 81, 1, "impl", 2),
        TypeExpression::Parameter("Self".into()),
        ImplKind::Trait,
        Some(&EXTERNAL_DEFINITION_RIGHT),
        &EMPTY_GENERIC_DEFINITION,
    )
    .expect("the second trait impl definition must be valid")
});
static TRAIT_IMPL_LEFT: LazyLock<ImplDescriptor> = LazyLock::new(|| {
    ImplDescriptor::builder(&TRAIT_IMPL_DEFINITION_LEFT, independent_descriptor)
        .implemented_trait(&APPLIED_EXTERNAL_TRAIT)
        .build()
        .expect("the first trait impl fixture must be valid")
});
static TRAIT_IMPL_RIGHT: LazyLock<ImplDescriptor> = LazyLock::new(|| {
    ImplDescriptor::builder(&TRAIT_IMPL_DEFINITION_RIGHT, independent_descriptor)
        .implemented_trait(&APPLIED_EXTERNAL_TRAIT)
        .build()
        .expect("the second trait impl fixture must be valid")
});

const TRAIT_IMPL_IDENTITY_LEFT: StaticFragmentIdentity = static_identity("trait_impl_a", 80, "impl", 1);
const TRAIT_IMPL_IDENTITY_RIGHT: StaticFragmentIdentity = static_identity("trait_impl_b", 81, "impl", 2);

/// Builds the first impl of the shared external trait and target.
fn trait_impl_payload_left() -> FragmentPayload {
    FragmentPayload::Impl(&TRAIT_IMPL_LEFT)
}

/// Builds the duplicate impl of the same external trait and target.
fn trait_impl_payload_right() -> FragmentPayload {
    FragmentPayload::Impl(&TRAIT_IMPL_RIGHT)
}

static TRAIT_IMPL_FRAGMENT_LEFT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Impl,
    TRAIT_IMPL_IDENTITY_LEFT,
    impl_runtime_identity,
    trait_impl_payload_left,
);
static TRAIT_IMPL_FRAGMENT_RIGHT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Impl,
    TRAIT_IMPL_IDENTITY_RIGHT,
    impl_runtime_identity,
    trait_impl_payload_right,
);

/// Verifies inventory discovery, stable fragment ordering, and all public type
/// indexes.
#[test]
fn test_registry_runtime_discovers_and_indexes_types_in_stable_order() {
    let registry = ReflectRegistry::initialize().expect("valid linked fragments must initialize");
    assert!(std::ptr::eq(
        registry
            .get(TypeId::of::<EarlyType>())
            .expect("the early type must be indexed"),
        &EARLY_DESCRIPTOR,
    ));
    assert!(std::ptr::eq(
        registry
            .get(TypeId::of::<LateType>())
            .expect("the late type must be indexed"),
        &LATE_DESCRIPTOR,
    ));

    let early_position = registry
        .types()
        .iter()
        .position(|descriptor| descriptor.type_id() == TypeId::of::<EarlyType>())
        .expect("the early descriptor must be enumerated");
    let late_position = registry
        .types()
        .iter()
        .position(|descriptor| descriptor.type_id() == TypeId::of::<LateType>())
        .expect("the late descriptor must be enumerated");
    assert!(early_position < late_position);

    let type_name_matches: Vec<_> = registry
        .find_by_type_name(std::any::type_name::<EarlyType>())
        .iter()
        .map(TypeDescriptor::type_id)
        .collect();
    assert_eq!(type_name_matches, [TypeId::of::<EarlyType>()]);

    let query_matches: Vec<_> = registry
        .find_by_query_name("shared-query")
        .iter()
        .map(TypeDescriptor::type_id)
        .collect();
    assert_eq!(query_matches, [TypeId::of::<EarlyType>(), TypeId::of::<LateType>()]);
    assert!(registry.find_by_query_name("missing").is_empty());
}

/// Verifies simultaneous first access publishes exactly one immutable snapshot.
#[test]
fn test_registry_runtime_initializes_once_across_threads() {
    let handles: Vec<_> = (0..32)
        .map(|_| {
            std::thread::spawn(|| {
                ReflectRegistry::initialize().expect("valid linked fragments must initialize") as *const ReflectRegistry
                    as usize
            })
        })
        .collect();
    let addresses: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("registry initialization thread must finish"))
        .collect();
    assert!(addresses.windows(2).all(|pair| pair[0] == pair[1]));
}

/// Verifies exact static identity duplicates retain both conflicting
/// identities.
#[test]
fn test_registry_runtime_rejects_duplicate_fragment_identity() {
    let error = build_registry(&[&DUPLICATE_RIGHT, &DUPLICATE_LEFT])
        .expect_err("an exact fragment identity duplicate must fail");
    assert_eq!(error.kind(), RegistryErrorKind::DuplicateFragment);
    let (left, right) = error
        .conflicting_fragments()
        .expect("a conflict must retain both fragment identities");
    assert_eq!(left, right);
}

/// Verifies one source identity cannot silently change its normalized content.
#[test]
fn test_registry_runtime_rejects_content_fingerprint_conflict() {
    let error = build_registry(&[&CONTENT_RIGHT, &CONTENT_LEFT])
        .expect_err("one source identity with different fingerprints must fail");
    assert_eq!(error.kind(), RegistryErrorKind::IdentityConflict);
    let (left, right) = error
        .conflicting_fragments()
        .expect("a conflict must retain both fragment identities");
    assert_ne!(left.content_fingerprint(), right.content_fingerprint());
}

/// Verifies capability IDs are unique for one exact concrete target.
#[test]
fn test_registry_runtime_rejects_capability_contract_conflict() {
    let error = build_registry(&[&CAPABILITY_RIGHT, &CAPABILITY_LEFT])
        .expect_err("one target capability ID cannot have two contracts");
    assert_eq!(error.kind(), RegistryErrorKind::CapabilityConflict);
}

/// Verifies diagnostic source aliases do not split one stable external trait
/// ID.
#[test]
fn test_registry_runtime_merges_external_trait_path_aliases() {
    build_registry(&[&EXTERNAL_RIGHT, &EXTERNAL_LEFT])
        .expect("different diagnostic paths with one external ID must merge");
}

/// Verifies one external-trait ID cannot identify incompatible declaration
/// facts.
#[test]
fn test_registry_runtime_rejects_external_trait_id_conflict() {
    let error = build_registry(&[&EXTERNAL_CONFLICT, &EXTERNAL_LEFT])
        .expect_err("one external trait ID cannot name incompatible generic declarations");
    assert_eq!(error.kind(), RegistryErrorKind::ExternalTraitIdConflict);
}

/// Verifies an impl payload cannot claim a different registration identity.
#[test]
fn test_registry_runtime_rejects_impl_registration_identity_mismatch() {
    let error =
        build_registry(&[&MISMATCHED_IMPL_FRAGMENT]).expect_err("the outer and descriptor impl identities must match");
    assert_eq!(error.kind(), RegistryErrorKind::IdentityConflict);
    let (definition, registration) = error
        .conflicting_fragments()
        .expect("the mismatch must retain both identities");
    assert_eq!(definition.module_path(), "impl");
    assert_eq!(registration.module_path(), "impl_duplicate");

    build_registry(&[&IMPL_FRAGMENT]).expect("a matching impl identity must register");
}

/// Verifies one target cannot register the same applied trait impl twice.
#[test]
fn test_registry_runtime_rejects_duplicate_target_trait_impl() {
    let error = build_registry(&[&TRAIT_IMPL_FRAGMENT_RIGHT, &TRAIT_IMPL_FRAGMENT_LEFT])
        .expect_err("duplicate target trait impls must fail deterministically");
    assert_eq!(error.kind(), RegistryErrorKind::IdentityConflict);
    let (left, right) = error
        .conflicting_fragments()
        .expect("the duplicate impl error must retain both fragments");
    assert_eq!(left.module_path(), "trait_impl_a");
    assert_eq!(right.module_path(), "trait_impl_b");
}

static CONFLICTING_FRAGMENTS: [&RegistrationFragment; 2] = [&CONTENT_RIGHT, &CONTENT_LEFT];
static VALID_FRAGMENTS: [&RegistrationFragment; 1] = [&EARLY_FRAGMENT];
static ERROR_CACHE: OnceLock<Result<ReflectRegistry, reflect::error::RegistryError>> = OnceLock::new();

/// Verifies failed initialization is cached and returned by cheap error clones.
#[test]
fn test_registry_runtime_caches_initialization_error() {
    let first =
        initialize_registry(&ERROR_CACHE, &CONFLICTING_FRAGMENTS).expect_err("the conflicting registry must fail");
    let second =
        initialize_registry(&ERROR_CACHE, &VALID_FRAGMENTS).expect_err("the cached error must win over later input");
    assert_eq!(first.kind(), RegistryErrorKind::IdentityConflict);
    assert_eq!(second.kind(), first.kind());
    assert_eq!(second.conflicting_fragments(), first.conflicting_fragments());
}

/// Verifies registry aggregation failure cannot poison pure descriptor
/// structure queries.
#[test]
fn test_registry_runtime_keeps_descriptor_queries_independent() {
    let _ = build_registry(&[&CONTENT_LEFT, &CONTENT_RIGHT]).expect_err("the synthetic registry must fail");
    let descriptor = TypeDescriptor::of::<IndependentType>();
    assert_eq!(descriptor.type_id(), TypeId::of::<IndependentType>());
    assert_eq!(descriptor.query_name(), "independent");
}
