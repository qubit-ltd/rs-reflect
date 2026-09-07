// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Descriptor diagnostics must never initialize intrinsic capabilities.

use qubit_reflect::TypeDescriptor;

#[test]
fn test_debug_reports_structure_without_a_synthetic_capability_count() {
    let output = format!("{:?}", TypeDescriptor::of::<u32>());
    assert!(
        output.contains("type_name"),
        "the diagnostic must retain type facts"
    );
    assert!(
        !output.contains("intrinsic_capability_count"),
        "Debug must not report an unqueried capability count"
    );
}

#[cfg(feature = "derive")]
mod derived {
    use std::process::Command;
    use std::process::Stdio;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    use std::time::Instant;

    use qubit_reflect::Reflect;
    use qubit_reflect::TypeDescriptor;
    use qubit_reflect::capability::CapabilityDescriptor;
    use qubit_reflect::capability::CapabilityKey;
    use qubit_reflect::identity::CapabilityId;
    use qubit_reflect::registry::RegistrySnapshotBuilder;

    static CALLS: AtomicUsize = AtomicUsize::new(0);

    /// Creates the fixed capability used by the diagnostic fixtures.
    fn capability() -> CapabilityDescriptor {
        let key = CapabilityKey::new(
            CapabilityId::new("example.debug").expect("valid fixture ID"),
        );
        CapabilityDescriptor::with_adapter(key, 7_u32)
    }

    /// Counts execution independently from descriptor construction.
    fn counted<T: 'static>() -> CapabilityDescriptor {
        let _ = std::marker::PhantomData::<T>;
        CALLS.fetch_add(1, Ordering::SeqCst);
        capability()
    }

    #[derive(Reflect)]
    #[reflect(capabilities(counted))]
    struct Counted<T> {
        value: T,
    }

    #[test]
    fn test_concurrent_debug_does_not_execute_a_provider() {
        let descriptor = TypeDescriptor::of::<Counted<u64>>();
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    assert!(format!("{descriptor:?}").contains("Counted"));
                });
            }
        });
        assert_eq!(
            CALLS.load(Ordering::SeqCst),
            0,
            "formatting must not initialize capabilities"
        );
    }

    /// Logs the descriptor whose capability is currently being initialized.
    fn logging<T: Reflect>() -> CapabilityDescriptor {
        eprintln!("{:?}", TypeDescriptor::of::<T>());
        CALLS.fetch_add(1, Ordering::SeqCst);
        capability()
    }

    #[derive(Reflect)]
    #[reflect(capabilities(logging))]
    struct Logging<T> {
        value: T,
    }

    #[test]
    fn test_provider_can_format_its_own_descriptor() {
        const CHILD: &str = "QUBIT_REFLECT_DEBUG_REENTRY_CHILD";
        if std::env::var_os(CHILD).is_some() {
            let snapshot = RegistrySnapshotBuilder::new()
                .build()
                .expect("empty snapshot");
            let descriptor = TypeDescriptor::of::<Logging<u32>>();
            assert!(snapshot.capabilities(descriptor).is_ok());
            assert!(snapshot.capabilities(descriptor).is_ok());
            assert_eq!(
                CALLS.load(Ordering::SeqCst),
                1,
                "provider must initialize exactly once"
            );
            return;
        }
        let mut child =
            Command::new(std::env::current_exe().expect("test executable"))
                .args([
                    "--exact",
                    "derived::test_provider_can_format_its_own_descriptor",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("start isolated reentry fixture");
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if child.try_wait().expect("inspect child state").is_some() {
                let output =
                    child.wait_with_output().expect("collect fixture output");
                assert!(
                    output.status.success(),
                    "fixture failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                break;
            }
            if Instant::now() >= deadline {
                child.kill().expect("terminate stuck fixture");
                let output =
                    child.wait_with_output().expect("reap stuck fixture");
                panic!(
                    "descriptor Debug reentered capability initialization: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Panics only when the caller explicitly requests capabilities.
    fn panicking<T: 'static>() -> CapabilityDescriptor {
        let _ = std::marker::PhantomData::<T>;
        panic!("explicit capability provider panic");
    }

    #[derive(Reflect)]
    #[reflect(capabilities(panicking))]
    struct Panicking<T> {
        value: T,
    }

    #[test]
    fn test_debug_does_not_trigger_a_panicking_provider() {
        let descriptor = TypeDescriptor::of::<Panicking<u32>>();
        assert!(format!("{descriptor:?}").contains("Panicking"));
        let snapshot = RegistrySnapshotBuilder::new()
            .build()
            .expect("empty snapshot");
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || snapshot.capabilities(descriptor)
            ))
            .is_err()
        );
    }

    /// Declares the same capability twice to exercise structured conflict
    /// handling.
    fn duplicate<T: 'static>() -> CapabilityDescriptor {
        let _ = std::marker::PhantomData::<T>;
        capability()
    }

    /// Supplies a second declaration of the same capability ID.
    fn second_duplicate<T: 'static>() -> CapabilityDescriptor {
        duplicate::<T>()
    }

    #[derive(Reflect)]
    #[reflect(capabilities(duplicate, second_duplicate))]
    struct Conflict<T> {
        value: T,
    }

    #[test]
    fn test_debug_does_not_hide_conflicts_as_an_empty_set() {
        let descriptor = TypeDescriptor::of::<Conflict<u32>>();
        let output = format!("{descriptor:?}");
        assert!(!output.contains("intrinsic_capability_count"));
        let snapshot = RegistrySnapshotBuilder::new()
            .build()
            .expect("empty snapshot");
        let error = snapshot
            .capabilities(descriptor)
            .expect_err("duplicate declarations must remain a conflict");
        assert_eq!(error.id().as_str(), "example.debug");
    }
}
