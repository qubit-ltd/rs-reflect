use std::fs;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

use qubit_reflect::descriptor::Reflect;
use qubit_reflect::descriptor::TypeRegistration;

struct ReentrantFixture;

impl Reflect for ReentrantFixture {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        if let Some(path) = std::env::var_os("QUBIT_REFLECT_REENTRANCY_MARKER") {
            let mut marker = fs::File::create(path).expect("create callback marker");
            marker
                .write_all(b"callback-entered")
                .expect("write callback marker");
            let _ = qubit_reflect::ReflectionContext::global().types();
        }
        static REGISTRATION: TypeRegistration<ReentrantFixture> = TypeRegistration::new();
        &REGISTRATION
    }
}

qubit_reflect::__private::codegen_v4::registration::submit_type!(ReentrantFixture);

#[test]
fn public_registration_callback_reentry_child() {
    if std::env::var_os("QUBIT_REFLECT_REENTRANCY_CHILD").is_none() {
        return;
    }
    let panic =
        match std::panic::catch_unwind(|| qubit_reflect::ReflectionContext::global().types()) {
            Ok(_) => panic!("recursive global initialization must panic"),
            Err(panic) => panic,
        };
    assert!(
        panic.downcast_ref::<String>().is_some_and(|message| {
            message == "recursive global reflection context initialization"
        }) || panic.downcast_ref::<&str>().is_some_and(|message| {
            *message == "recursive global reflection context initialization"
        }),
        "unexpected panic payload: {panic:?}"
    );
}

#[test]
fn public_registration_callback_reentry_is_bounded() {
    let marker = std::env::temp_dir().join(format!(
        "qubit-reflect-reentrancy-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos()
    ));
    let mut child = Command::new(std::env::current_exe().expect("test executable path"))
        .arg("--exact")
        .arg("public_registration_callback_reentry_child")
        .arg("--nocapture")
        .env("QUBIT_REFLECT_REENTRANCY_CHILD", "1")
        .env("QUBIT_REFLECT_REENTRANCY_MARKER", &marker)
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
    let marker_contents = fs::read(&marker).ok();
    let _ = fs::remove_file(&marker);
    assert!(
        !timed_out
            && marker_contents.as_deref() == Some(b"callback-entered")
            && String::from_utf8_lossy(&output.stderr)
                .contains("recursive global reflection context initialization"),
        "expected callback marker followed by the fixed diagnostic before the deadline; marker={marker_contents:?}, timed_out={timed_out}, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}
