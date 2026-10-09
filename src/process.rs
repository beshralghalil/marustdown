use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Runs `command` with `input` on stdin and returns its stdout, at most `limit` bytes.
/// `None` if it can't start, fails, or outlives `timeout`; it is then killed along
/// with everything it spawned. No shell is involved and stderr is discarded.
pub fn run(mut command: Command, input: Vec<u8>, timeout: Duration, limit: u64) -> Option<Vec<u8>> {
    let deadline = Instant::now() + timeout;
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    own_group(&mut command);
    let mut child = command.spawn().ok()?;
    let (mut stdin, mut stdout) = (child.stdin.take()?, child.stdout.take()?);
    thread::spawn(move || stdin.write_all(&input));
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut out = Vec::new();
        let _ = (&mut stdout).take(limit).read_to_end(&mut out);
        let _ = tx.send(out);
    });
    match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(output) if exited_ok(&mut child, deadline) => Some(output),
        _ => {
            stop(&mut child);
            None
        }
    }
}

/// Waits until `deadline`; true if the child exited successfully by then.
fn exited_ok(child: &mut Child, deadline: Instant) -> bool {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            _ => return false,
        }
    }
}

/// Starts the child in its own process group, so `stop` reaches anything it spawns.
#[cfg(unix)]
fn own_group(command: &mut Command) {
    std::os::unix::process::CommandExt::process_group(command, 0);
}

#[cfg(not(unix))]
fn own_group(_: &mut Command) {}

/// Kills the child and everything it spawned, then reaps it.
fn stop(child: &mut Child) {
    #[cfg(unix)]
    if let Ok(group) = i32::try_from(child.id()) {
        // SAFETY: kill(2) has no memory-safety requirements.
        unsafe { libc::kill(-group, libc::SIGKILL) };
    }
    let _ = child.kill();
    let _ = child.wait();
}
