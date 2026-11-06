#![cfg(unix)]

use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

#[test]
fn help_version_and_usage() {
    let help = output(&["--help"]);
    assert!(help.status.success());
    assert!(
        help.stdout
            .starts_with(b"Run a command repeatedly.\n\nUsage:")
    );
    assert_eq!(help.stderr, b"");
    let version = output(&["-V"]);
    assert!(version.status.success());
    assert_eq!(version.stdout, b"repete 0.1.0\n");
    for args in [vec![], vec!["0"], vec!["-x"], vec!["-n", "NaN", "echo"]] {
        let result = output(&args);
        assert_eq!(result.status.code(), Some(2));
        assert_eq!(result.stdout, b"");
        assert!(String::from_utf8_lossy(&result.stderr).contains("Try 'repete -h' for help."));
    }
}

#[test]
fn finite_count_and_streams_are_untouched() {
    let result = output(&["3", "/bin/sh", "-c", "printf 'out\n'; printf 'err\n' >&2"]);
    assert!(result.status.success());
    assert_eq!(result.stdout, b"out\nout\nout\n");
    assert_eq!(result.stderr, b"err\nerr\nerr\n");
    let zero = output(&["0", "/no/such/repete-test-command"]);
    assert!(zero.status.success());
    assert_eq!(zero.stdout, b"");
    assert_eq!(zero.stderr, b"");
}

#[test]
fn command_arguments_keep_their_original_bytes() {
    let args = [
        OsString::from("a b"),
        OsString::new(),
        OsString::from("$HOME | * ; ' \""),
        OsString::from("-n"),
        OsString::from("--help"),
        OsString::from("--"),
        OsString::from_vec(vec![b'x', 0xff]),
    ];
    let mut command = binary();
    command.args(["1", "/bin/sh", "-c", "printf '%s\\000' \"$@\"", "args"]);
    command.args(&args);
    let result = bounded_output(command, Duration::from_secs(3));
    assert!(result.status.success());
    let expected: Vec<u8> = args
        .iter()
        .flat_map(|arg| arg.clone().into_vec().into_iter().chain([0]))
        .collect();
    assert_eq!(result.stdout, expected);
    assert_eq!(result.stderr, b"");
}

#[test]
fn explicit_shell_pipelines_work() {
    let result = output(&["2", "/bin/sh", "-c", "printf foo | cat"]);
    assert!(result.status.success());
    assert_eq!(result.stdout, b"foofoo");
    assert_eq!(result.stderr, b"");
}

#[test]
fn nonzero_exits_continue_and_return_the_last_status() {
    let temp = TempDir::new();
    let state = temp.0.join("state");
    let mut command = binary();
    command.args([
        "2",
        "/bin/sh",
        "-c",
        "if test -f \"$1\"; then printf second; exit 7; fi; touch \"$1\"; printf first; exit 3",
        "status",
    ]);
    command.arg(state);
    let result = bounded_output(command, Duration::from_secs(3));
    assert_eq!(result.status.code(), Some(7));
    assert_eq!(result.stdout, b"firstsecond");
}

#[test]
fn execution_failures_stop_with_shell_style_codes() {
    let temp = TempDir::new();
    let missing = temp.0.join("missing");
    let denied = temp.0.join("denied");
    fs::write(&denied, "text").unwrap();
    fs::set_permissions(&denied, fs::Permissions::from_mode(0o600)).unwrap();
    #[cfg(target_os = "macos")]
    let invalid = {
        // A Mach-O header with an invalid CPU type fails at launch on macOS.
        let path = temp.0.join("invalid");
        let header: Vec<u8> = [0xfeed_facf_u32, 0xdead_beef, 0, 2, 0, 0, 0, 0]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect();
        fs::write(&path, header).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    };
    for (path, code) in [
        (missing, 127),
        (denied, 126),
        #[cfg(target_os = "macos")]
        (invalid, 126),
    ] {
        let mut command = binary();
        command.args([OsStr::new("2"), path.as_os_str()]);
        let result = bounded_output(command, Duration::from_secs(3));
        assert_eq!(result.status.code(), Some(code));
        assert_eq!(result.stdout, b"");
        assert!(String::from_utf8_lossy(&result.stderr).starts_with("repete: cannot execute"));
    }
}

#[test]
fn direct_child_signal_stops_repetition() {
    let result = output(&["3", "/bin/sh", "-c", "printf once; kill -TERM $$"]);
    assert_eq!(result.status.code(), Some(143));
    assert_eq!(result.stdout, b"once");
    assert_eq!(result.stderr, b"");
}

#[test]
fn one_run_does_not_sleep_after_completion() {
    for option in ["-n", "-N"] {
        let mut command = binary();
        command.args(["1", option, "60", "/bin/sh", "-c", "printf done"]);
        let result = bounded_output(command, Duration::from_secs(3));
        assert!(result.status.success());
        assert_eq!(result.stdout, b"done");
    }
}

#[test]
fn both_interval_modes_delay_the_second_execution() {
    for option in ["-n", "-N"] {
        let mut command = binary();
        command.args(["2", option, "0.3", "/bin/sh", "-c", "printf x"]);
        let started = Instant::now();
        let result = bounded_output(command, Duration::from_secs(5));
        assert!(result.status.success());
        assert_eq!(result.stdout, b"xx");
        assert!(started.elapsed() >= Duration::from_millis(250), "{option}");
    }
}

#[test]
fn infinite_mode_repeats_until_its_process_group_is_interrupted() {
    let temp = TempDir::new();
    let log = temp.0.join("runs");
    let mut command = binary();
    command.args(["-n", "0.03", "/bin/sh", "-c", "printf x >> \"$1\"", "log"]);
    command.arg(&log).process_group(0);
    command.stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while fs::read(&log).unwrap_or_default().len() < 3 {
        if Instant::now() >= deadline || child.try_wait().unwrap().is_some() {
            signal_group(&child, 9);
            let _ = child.wait();
            panic!("infinite mode did not repeat");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    signal_group(&child, 2);
    let result = wait(child, Duration::from_secs(3));
    assert_eq!(result.status.signal(), Some(2));
    let runs = fs::read(&log).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(fs::read(&log).unwrap(), runs);
}

fn output(args: &[&str]) -> Output {
    let mut command = binary();
    command.args(args);
    bounded_output(command, Duration::from_secs(3))
}

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_repete"))
}

fn bounded_output(mut command: Command, timeout: Duration) -> Output {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    wait(command.spawn().unwrap(), timeout)
}

fn wait(mut child: Child, timeout: Duration) -> Output {
    let deadline = Instant::now() + timeout;
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("command exceeded {timeout:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn signal_group(child: &Child, signal: i32) {
    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }
    let group = -i32::try_from(child.id()).unwrap();
    // This child was placed in its own process group before spawning.
    unsafe { kill(group, signal) };
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "repete-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
