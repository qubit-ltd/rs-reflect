// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
//! Deterministic concurrency, registry-model, and ownership stress coverage.

use std::any::TypeId;
use std::sync::Arc;
use std::sync::Barrier;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_reflect as reflect;
use qubit_reflect::__private::registration::CapabilityRegistration;
use qubit_reflect::__private::registration::FragmentKind;
use qubit_reflect::__private::registration::FragmentPayload;
use qubit_reflect::__private::registration::RegistrationFragment;
use qubit_reflect::__private::registration::RuntimeIdentity;
use qubit_reflect::__private::registration::StaticFragmentIdentity;
use qubit_reflect::__private::registration::build_registry;
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::capability::CapabilityDescriptor;
use qubit_reflect::capability::CapabilityKey;
use qubit_reflect::error::RegistryErrorKind;
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::registry::ReflectRegistry;
use qubit_reflect::value::ReflectedMut;
use qubit_reflect::value::ReflectedOwned;

#[derive(Reflect)]
struct StressRecord<T> {
    values: Vec<T>,
}

#[derive(Reflect)]
struct RecursiveStressNode {
    children: Vec<Option<Box<RecursiveStressNode>>>,
}

#[derive(Reflect)]
struct RecoveryRecord {
    value: u64,
}

struct DropProbe(Arc<AtomicUsize>);

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

struct NameCandidateLeft;
struct NameCandidateRight;
struct CapabilityTarget;

static NAME_LEFT_DESCRIPTOR: TypeDescriptor =
    reflect::__private::descriptor::opaque_root::<NameCandidateLeft>("stress-name");
static NAME_RIGHT_DESCRIPTOR: TypeDescriptor =
    reflect::__private::descriptor::opaque_root::<NameCandidateRight>("stress-name");

const fn identity(module: &'static str, line: u32, fingerprint: u64) -> StaticFragmentIdentity {
    StaticFragmentIdentity::new("stress-fixture", module, line, 1, "type", fingerprint)
}

fn left_runtime_identity() -> RuntimeIdentity {
    RuntimeIdentity::Type(TypeId::of::<NameCandidateLeft>())
}

fn left_payload() -> FragmentPayload {
    FragmentPayload::Type(&NAME_LEFT_DESCRIPTOR)
}

fn right_runtime_identity() -> RuntimeIdentity {
    RuntimeIdentity::Type(TypeId::of::<NameCandidateRight>())
}

fn right_payload() -> FragmentPayload {
    FragmentPayload::Type(&NAME_RIGHT_DESCRIPTOR)
}

static NAME_LEFT_FRAGMENT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Type,
    identity("a_name", 10, 10),
    left_runtime_identity,
    left_payload,
);
static NAME_RIGHT_FRAGMENT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Type,
    identity("z_name", 20, 20),
    right_runtime_identity,
    right_payload,
);
fn stress_capability_id() -> CapabilityId {
    CapabilityId::new("stress.registry.capability").expect("fixture capability ID must be valid")
}

fn capability_runtime_identity() -> RuntimeIdentity {
    RuntimeIdentity::Capability {
        target_type_id: TypeId::of::<CapabilityTarget>(),
        capability_id: stress_capability_id(),
    }
}

fn capability_u8_payload() -> FragmentPayload {
    FragmentPayload::Capability(CapabilityRegistration::new(
        TypeId::of::<CapabilityTarget>(),
        CapabilityDescriptor::without_adapter(CapabilityKey::<u8>::new(stress_capability_id())),
    ))
}

fn capability_u16_payload() -> FragmentPayload {
    FragmentPayload::Capability(CapabilityRegistration::new(
        TypeId::of::<CapabilityTarget>(),
        CapabilityDescriptor::without_adapter(CapabilityKey::<u16>::new(stress_capability_id())),
    ))
}

static CAPABILITY_LEFT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Capability,
    StaticFragmentIdentity::new("stress-fixture", "capability_a", 40, 1, "capability", 40),
    capability_runtime_identity,
    capability_u8_payload,
);
static CAPABILITY_RIGHT: RegistrationFragment = RegistrationFragment::new(
    FragmentKind::Capability,
    StaticFragmentIdentity::new("stress-fixture", "capability_b", 50, 1, "capability", 50),
    capability_runtime_identity,
    capability_u16_payload,
);

/// Produces deterministic state-machine input from a repeatable xorshift seed.
fn next_random(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

/// Returns the recursive node reached through `Vec<Option<Box<Node>>>`.
fn recursive_sequence_target() -> &'static TypeDescriptor {
    let root = TypeDescriptor::of::<RecursiveStressNode>();
    root.field("children")
        .expect("recursive field must exist")
        .field_type()
        .as_resolved()
        .expect("recursive field must resolve to Vec")
        .as_sequence()
        .expect("recursive field must expose its sequence view")
        .element_type()
        .as_resolved()
        .expect("sequence element must resolve to Option")
        .as_optional()
        .expect("sequence element must expose its optional view")
        .element_type()
        .as_resolved()
        .expect("optional element must resolve to Box")
        .as_smart_pointer()
        .expect("optional element must expose its smart-pointer view")
        .pointee_type()
        .as_resolved()
        .expect("Box pointee must resolve to the recursive root")
}

/// Verifies multiple concrete generic/composite roots converge to one pointer
/// per `TypeId` without conflating distinct applications.
#[test]
fn test_stress_concurrent_generic_descriptor_and_registry_initialization() {
    const WORKERS: usize = 16;
    let barrier = Arc::new(Barrier::new(WORKERS));
    let workers: Vec<_> = (0..WORKERS)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                let descriptors = [
                    TypeDescriptor::of::<StressRecord<u64>>(),
                    TypeDescriptor::of::<StressRecord<String>>(),
                    TypeDescriptor::of::<Vec<Option<u64>>>(),
                    TypeDescriptor::of::<Option<Vec<String>>>(),
                ];
                let recursive_root = TypeDescriptor::of::<RecursiveStressNode>();
                let recursive_target = recursive_sequence_target();
                assert!(std::ptr::eq(recursive_root, recursive_target));
                let registry = ReflectRegistry::initialize().expect("registry must initialize once");
                (
                    descriptors.map(|descriptor| descriptor as *const TypeDescriptor as usize),
                    descriptors.map(TypeDescriptor::type_id),
                    registry.types().len(),
                )
            })
        })
        .collect();
    let results: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().expect("stress worker must not panic"))
        .collect();

    for result in &results[1..] {
        assert_eq!(result.0, results[0].0, "one TypeId must intern to one pointer");
        assert_eq!(result.1, results[0].1, "concrete arguments must remain stable");
        assert_eq!(result.2, results[0].2, "registry snapshot must be immutable");
    }
    for left in 0..results[0].1.len() {
        for right in (left + 1)..results[0].1.len() {
            assert_ne!(results[0].1[left], results[0].1[right]);
            assert_ne!(results[0].0[left], results[0].0[right]);
        }
    }
}

/// Independently predicts the result of one generated registry operation
/// sequence.
fn reference_model(operations: &[u8]) -> Result<Vec<TypeId>, RegistryErrorKind> {
    let mut counts = [0_usize; 4];
    for operation in operations {
        counts[usize::from(*operation)] += 1;
    }
    if counts.iter().any(|count| *count > 1) {
        return Err(RegistryErrorKind::DuplicateFragment);
    }
    if counts[2] == 1 && counts[3] == 1 {
        return Err(RegistryErrorKind::CapabilityConflict);
    }

    let mut names = Vec::new();
    if counts[0] == 1 {
        names.push(TypeId::of::<NameCandidateLeft>());
    }
    if counts[1] == 1 {
        names.push(TypeId::of::<NameCandidateRight>());
    }
    Ok(names)
}

/// Compares generated registry operation sequences with an independent
/// pure-Rust state model.
#[test]
fn test_stress_registry_matches_reference_model_for_order_and_conflicts() {
    let mut seed = 0x4d59_5df4_d0f3_3173;
    let fragments = [
        &NAME_LEFT_FRAGMENT,
        &NAME_RIGHT_FRAGMENT,
        &CAPABILITY_LEFT,
        &CAPABILITY_RIGHT,
    ];
    let mut observed_success = false;
    let mut observed_duplicate = false;
    let mut observed_capability_conflict = false;

    for _ in 0..512 {
        let operation_count = (next_random(&mut seed) as usize % 8) + 1;
        let operations = (0..operation_count)
            .map(|_| (next_random(&mut seed) % 4) as u8)
            .collect::<Vec<_>>();
        let actual_fragments = operations
            .iter()
            .map(|operation| fragments[usize::from(*operation)])
            .collect::<Vec<_>>();

        match (reference_model(&operations), build_registry(&actual_fragments)) {
            (Ok(expected_names), Ok(registry)) => {
                observed_success = true;
                let actual_names = registry
                    .find_by_query_name("stress-name")
                    .into_iter()
                    .map(TypeDescriptor::type_id)
                    .collect::<Vec<_>>();
                assert_eq!(actual_names, expected_names);
            }
            (Err(expected), Err(actual)) => {
                observed_duplicate |= expected == RegistryErrorKind::DuplicateFragment;
                observed_capability_conflict |= expected == RegistryErrorKind::CapabilityConflict;
                assert_eq!(actual.kind(), expected);
            }
            (expected, actual) => {
                panic!("registry/model mismatch: expected {expected:?}, actual {actual:?}")
            }
        }
    }

    assert!(observed_success);
    assert!(observed_duplicate);
    assert!(observed_capability_conflict);
}

/// Proves repeated pre-execution failures retain ownership until the caller
/// explicitly extracts and destroys each recovered replacement.
#[test]
fn test_stress_field_failure_recovery_drops_each_input_exactly_once() {
    const ATTEMPTS: usize = 1_024;
    let field = TypeDescriptor::of::<RecoveryRecord>()
        .field("value")
        .expect("derived field must be discoverable");
    let drops = Arc::new(AtomicUsize::new(0));
    let mut target = RecoveryRecord { value: 7 };

    for attempt in 0..ATTEMPTS {
        let replacement = ReflectedOwned::new(DropProbe(Arc::clone(&drops)));
        let failure = field
            .set(ReflectedMut::new(&mut target), replacement)
            .expect_err("wrong replacement type must fail before execution");
        assert_eq!(drops.load(Ordering::SeqCst), attempt);
        let recovered = failure
            .into_recovery()
            .unwrap_or_else(|_| panic!("pre-execution failure must retain recovery"))
            .into_value_at(0)
            .unwrap_or_else(|_| panic!("replacement must remain at its source index"))
            .downcast::<DropProbe>()
            .unwrap_or_else(|_| panic!("recovery must preserve the concrete replacement"));
        assert_eq!(drops.load(Ordering::SeqCst), attempt);
        drop(recovered);
    }

    assert_eq!(target.value, 7, "failed replacement must not mutate the target");
    assert_eq!(drops.load(Ordering::SeqCst), ATTEMPTS);
}
