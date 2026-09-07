//! Piping pup into a reader that closes early (`pup ... | head -50`) must not
//! panic. `pup --help` is used as the payload because it is offline and emits
//! several hundred KB, far more than a pipe buffer holds, so the write to the
//! closed pipe is guaranteed to happen.
#![cfg(unix)]

use std::io::Read;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

const PUP: &str = env!("CARGO_BIN_EXE_pup");

#[test]
fn closed_stdout_kills_pup_quietly() {
    let mut child = Command::new(PUP)
        .arg("--help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn pup");

    let mut stdout = child.stdout.take().expect("stdout was piped");
    let mut head = [0u8; 64];
    let read = stdout.read(&mut head).expect("failed to read from pup");
    assert!(read > 0, "pup produced no output to truncate");

    // Closing the read end is what `head` does once it has its lines.
    drop(stdout);

    let status = child.wait().expect("failed to wait for pup");
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("stderr was piped")
        .read_to_string(&mut stderr)
        .expect("failed to read pup stderr");

    assert!(
        !stderr.contains("panicked") && !stderr.contains("Broken pipe"),
        "pup panicked on a closed pipe; stderr was:\n{stderr}"
    );
    assert_ne!(
        status.code(),
        Some(101),
        "pup exited with the Rust panic status"
    );
    assert_eq!(
        status.signal(),
        Some(libc::SIGPIPE),
        "pup should be killed by SIGPIPE like any other Unix CLI, got {status:?}"
    );
}

#[test]
fn intact_stdout_still_completes_normally() {
    let output = Command::new(PUP)
        .arg("--help")
        .output()
        .expect("failed to run pup");

    assert!(
        output.status.success(),
        "pup --help failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.len() > 64,
        "pup --help produced a truncated {} byte output",
        output.stdout.len()
    );
}
