// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Bounded child-process tests for type registration initialization.

use std::io::BufRead;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Barrier;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

const CHILD_MODE_ENV: &str = "QUBIT_REFLECT_RF003_CHILD_MODE";
const READY_MARKER: &str = "RF003_INITIALIZER_READY";
const READY_TIMEOUT: Duration = Duration::from_secs(5);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(30);

struct DirectRecursive;
struct CycleA;
struct CycleB;
struct OrdinaryPanic;
struct ReentrantRetry;
struct AcyclicA;
struct AcyclicB;
struct Competing;
struct Warm;

#[test]
fn test_registration_reentry_child() {
    let Ok(mode) = std::env::var(CHILD_MODE_ENV) else {
        return;
    };

    match mode.as_str() {
        "direct" => {
            let _ = qubit_reflect::descriptor::TypeRegistration::<DirectRecursive>::get_or_init_with(|| {
                announce_ready();
                let _ = qubit_reflect::descriptor::TypeRegistration::<DirectRecursive>::get_or_init_with(
                    qubit_reflect::descriptor::TypeRegistration::new,
                );
                qubit_reflect::descriptor::TypeRegistration::new()
            });
        }
        "cycle" => {
            let _ = qubit_reflect::descriptor::TypeRegistration::<CycleA>::get_or_init_with(|| {
                qubit_reflect::descriptor::TypeRegistration::<CycleB>::get_or_init_with(|| {
                    announce_ready();
                    let _ = qubit_reflect::descriptor::TypeRegistration::<CycleA>::get_or_init_with(
                        qubit_reflect::descriptor::TypeRegistration::new,
                    );
                    qubit_reflect::descriptor::TypeRegistration::new()
                });
                qubit_reflect::descriptor::TypeRegistration::new()
            });
        }
        "retry" => {
            let first = std::panic::catch_unwind(|| {
                let _ = qubit_reflect::descriptor::TypeRegistration::<ReentrantRetry>::get_or_init_with(|| {
                    announce_ready();
                    let _ = qubit_reflect::descriptor::TypeRegistration::<ReentrantRetry>::get_or_init_with(
                        qubit_reflect::descriptor::TypeRegistration::new,
                    );
                    qubit_reflect::descriptor::TypeRegistration::new()
                });
            });
            assert!(first.is_err());

            let attempts = AtomicUsize::new(0);
            let _ = qubit_reflect::descriptor::TypeRegistration::<ReentrantRetry>::get_or_init_with(|| {
                attempts.fetch_add(1, Ordering::SeqCst);
                qubit_reflect::descriptor::TypeRegistration::new()
            });
            assert_eq!(attempts.load(Ordering::SeqCst), 1);
        }
        "normal" => run_normal_initialization_cases(),
        other => panic!("unknown RF-003 child mode: {other}"),
    }
}

#[test]
fn test_direct_reentry_fails_fast_in_child_process() {
    assert_reentry_fails_fast("direct");
}

#[test]
fn test_indirect_reentry_fails_fast_in_child_process() {
    assert_reentry_fails_fast("cycle");
}

#[test]
fn test_reentrant_initializer_panic_cleans_up_and_can_be_retried() {
    let result = run_child_probe("retry");
    assert_child_completed(&result, "retry");
    assert_eq!(result.exit_code, Some(0), "retry child did not succeed: {result:?}");
    assert!(
        result.output.contains("recursive type registration initialization"),
        "retry child did not report the reentry diagnostic: {result:?}",
    );
}

#[test]
fn test_normal_initialization_cases_complete_in_child_process() {
    let result = run_child_probe("normal");
    assert_child_completed(&result, "normal");
    assert_eq!(result.exit_code, Some(0), "normal child did not succeed: {result:?}");
}

fn run_normal_initialization_cases() {
    let first = std::panic::catch_unwind(|| {
        let _ = qubit_reflect::descriptor::TypeRegistration::<OrdinaryPanic>::get_or_init_with(|| {
            announce_ready();
            panic!("ordinary initializer panic")
        });
    });
    assert!(first.is_err());

    let attempts = AtomicUsize::new(0);
    let _ = qubit_reflect::descriptor::TypeRegistration::<OrdinaryPanic>::get_or_init_with(|| {
        attempts.fetch_add(1, Ordering::SeqCst);
        qubit_reflect::descriptor::TypeRegistration::new()
    });
    assert_eq!(attempts.load(Ordering::SeqCst), 1);

    let nested_attempts = AtomicUsize::new(0);
    let _ = qubit_reflect::descriptor::TypeRegistration::<AcyclicA>::get_or_init_with(|| {
        let _ = qubit_reflect::descriptor::TypeRegistration::<AcyclicB>::get_or_init_with(|| {
            nested_attempts.fetch_add(1, Ordering::SeqCst);
            qubit_reflect::descriptor::TypeRegistration::new()
        });
        qubit_reflect::descriptor::TypeRegistration::new()
    });
    assert_eq!(nested_attempts.load(Ordering::SeqCst), 1);

    const THREADS: usize = 8;
    let barrier = Arc::new(Barrier::new(THREADS));
    let calls = Arc::new(AtomicUsize::new(0));
    announce_ready();
    let workers: Vec<_> = (0..THREADS)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let calls = Arc::clone(&calls);
            thread::spawn(move || {
                barrier.wait();
                qubit_reflect::descriptor::TypeRegistration::<Competing>::get_or_init_with(|| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    qubit_reflect::descriptor::TypeRegistration::new()
                }) as *const _ as usize
            })
        })
        .collect();
    let addresses: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().expect("registration worker"))
        .collect();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(addresses.iter().all(|address| *address == addresses[0]));

    let _ = qubit_reflect::descriptor::TypeRegistration::<Warm>::get_or_init_with(
        qubit_reflect::descriptor::TypeRegistration::new,
    );
    let should_not_run = || panic!("warm initializer must not run");
    let _ = qubit_reflect::descriptor::TypeRegistration::<Warm>::get_or_init_with(should_not_run);
}

fn assert_reentry_fails_fast(mode: &str) {
    let result = run_child_probe(mode);
    assert_child_completed(&result, mode);
    assert_ne!(
        result.exit_code,
        Some(0),
        "{mode} child unexpectedly succeeded: {result:?}"
    );
    assert!(
        result.output.contains("recursive type registration initialization"),
        "{mode} child did not report the reentry diagnostic: {result:?}",
    );
    let type_name = if mode == "direct" { "DirectRecursive" } else { "CycleA" };
    assert!(
        result.output.contains(type_name),
        "{mode} child diagnostic omitted type name {type_name}: {result:?}",
    );
}

fn assert_child_completed(result: &ProbeResult, mode: &str) {
    assert!(result.ready, "{mode} child never became ready: {result:?}");
    assert!(!result.timed_out, "{mode} child timed out: {result:?}");
    assert!(
        result.kill_result.is_none(),
        "{mode} child reported an unexpected kill result: {result:?}"
    );
    assert!(result.recovered, "{mode} child was not reaped: {result:?}");
}

#[derive(Debug)]
struct ProbeResult {
    ready: bool,
    timed_out: bool,
    exit_code: Option<i32>,
    kill_result: Option<String>,
    recovered: bool,
    output: String,
}

fn run_child_probe(mode: &str) -> ProbeResult {
    let executable = std::env::current_exe().expect("current test executable");
    let mut child = Command::new(executable)
        .arg("--exact")
        .arg("test_registration_reentry_child")
        .arg("--nocapture")
        .env(CHILD_MODE_ENV, mode)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to start {mode} probe child: {error}"));

    let (line_tx, line_rx) = mpsc::channel();
    let stdout = child.stdout.take().expect("piped child stdout");
    let stdout_tx = line_tx.clone();
    let stdout_reader = thread::spawn(move || forward_lines(stdout, "stdout", stdout_tx));
    let stderr = child.stderr.take().expect("piped child stderr");
    let stderr_reader = thread::spawn(move || forward_lines(stderr, "stderr", line_tx));

    let started = Instant::now();
    let total_deadline = started + TOTAL_TIMEOUT;
    let mut ready_at = None;
    let mut output = String::new();

    let (timed_out, status) = loop {
        while let Ok(line) = line_rx.try_recv() {
            record_line(line, &mut ready_at, &mut output);
        }

        if let Some(status) = child.try_wait().expect("poll child status") {
            break (false, Some(status));
        }

        let now = Instant::now();
        if let Some(ready) = ready_at
            && now >= ready + READY_TIMEOUT
        {
            break (true, None);
        }
        if now >= total_deadline {
            break (true, None);
        }

        if let Ok(line) = line_rx.recv_timeout(Duration::from_millis(10)) {
            record_line(line, &mut ready_at, &mut output);
        }
    };

    let (exit_code, kill_result, recovered) = if timed_out {
        // The child may exit between try_wait and kill. Either way, wait must
        // reap it before the probe can report recovery.
        let kill_result = child.kill();
        let wait_result = child.wait();
        (
            wait_result.as_ref().ok().and_then(|status| status.code()),
            Some(format!("{kill_result:?}")),
            wait_result.is_ok(),
        )
    } else {
        let recovered = status.is_some();
        (status.and_then(|status| status.code()), None, recovered)
    };

    stdout_reader.join().expect("stdout reader thread");
    stderr_reader.join().expect("stderr reader thread");
    while let Ok(line) = line_rx.try_recv() {
        record_line(line, &mut ready_at, &mut output);
    }

    let result = ProbeResult {
        ready: ready_at.is_some(),
        timed_out,
        exit_code,
        kill_result,
        recovered,
        output,
    };
    println!("RF003 {mode} child outcome: {result:?}");
    result
}

fn forward_lines(reader: impl std::io::Read, stream: &str, sender: mpsc::Sender<String>) {
    for line in std::io::BufReader::new(reader).lines() {
        match line {
            Ok(line) => {
                if sender.send(format!("{stream}: {line}\n")).is_err() {
                    break;
                }
            }
            Err(error) => {
                let _ = sender.send(format!("{stream}: <read error: {error}>\n"));
                break;
            }
        }
    }
}

fn record_line(line: String, ready_at: &mut Option<Instant>, output: &mut String) {
    if line.contains(READY_MARKER) && ready_at.is_none() {
        *ready_at = Some(Instant::now());
    }
    output.push_str(&line);
}

fn announce_ready() {
    println!("{READY_MARKER}");
    std::io::stdout().flush().expect("flush child ready marker");
}
