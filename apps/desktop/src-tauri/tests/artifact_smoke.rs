/* ==========================================================================
 * Release artifact smoke test
 * Starts the built application the way a user does — by double-clicking an
 * executable, not by running the toolchain — and checks the four things a
 * shipped build has to do on an isolated desktop:
 *
 * 1. it starts with no Node, pnpm, Cargo, or repository anywhere in its
 *    environment, and from a directory that holds nothing but the executable;
 * 2. it writes its startup line to its own log;
 * 3. it creates a visible window;
 * 4. it exits cleanly when that window is closed with no work in flight.
 *
 * The application's own data directory follows the environment the launch sets,
 * so a run writes its log and its WebView2 profile into a scratch profile under
 * the temp directory and never touches the state or the log of a real
 * installation. Everything is isolated: the executable copy, the working
 * directory, the environment, and the application data.
 *
 * Run it after a release build:
 *
 *   cargo test --test artifact_smoke -- --nocapture
 *
 * or point it at any built binary with `CROSSPORT_APP_EXE`. Without a built
 * artifact the test reports that it was skipped and passes: the file is a
 * production check, not a unit test, and `cargo test` on a fresh clone must
 * stay meaningful.
 * ========================================================================== */

#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// How long the backend may take to write its startup line.
const START_TIMEOUT: Duration = Duration::from_secs(60);
/// How long the first window may take to appear. A machine whose WebView2
/// runtime has never started spends its own first-run initialization here —
/// around a minute, once, was observed on this host — so the budget is generous
/// while a runtime that has started before opens a window in under a second.
const WINDOW_TIMEOUT: Duration = Duration::from_secs(150);
/// How long a clean shutdown may take after the window is closed.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(30);

fn app_exe() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CROSSPORT_APP_EXE") {
        let path = PathBuf::from(path);
        return path.is_file().then_some(path);
    }
    // `tauri build` renames cargo's `crossport` binary to the configured
    // `mainBinaryName` before bundling, so a bundled artifact is
    // `CrossPort.exe`; the raw cargo name is accepted too, because a plain
    // `cargo build --release` produces that one without the rename.
    let release = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release");
    ["crossport.exe", "CrossPort.exe"]
        .iter()
        .map(|name| release.join(name))
        .find(|candidate| candidate.is_file())
}

/// The log file the isolated run appends to. The application resolves its log
/// directory from `LOCALAPPDATA`, which the launch points at the scratch
/// profile, so the path here is the isolated one and not a real install's.
fn log_file(profile: &Path) -> PathBuf {
    profile
        .join("AppData")
        .join("Local")
        .join("com.crossport.app")
        .join("logs")
        .join("crossport.log")
}

fn read_log(profile: &Path) -> String {
    std::fs::read_to_string(log_file(profile)).unwrap_or_default()
}

/// Everything the application is allowed to see: Windows itself, the user's
/// profile, and the temporary directory. No Node, no pnpm, no Cargo, and none
/// of the development environment this repository is built in.
fn scrubbed_environment(profile: &Path) -> Vec<(String, String)> {
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
    let temp = profile.join("Temp");
    let appdata = profile.join("AppData").join("Roaming");
    let local_appdata = profile.join("AppData").join("Local");
    vec![
        ("SystemRoot".to_string(), system_root.clone()),
        ("SystemDrive".to_string(), system_root[..2].to_string()),
        ("windir".to_string(), system_root.clone()),
        (
            "PATH".to_string(),
            format!("{system_root}\\System32;{system_root}"),
        ),
        ("TEMP".to_string(), temp.display().to_string()),
        ("TMP".to_string(), temp.display().to_string()),
        ("USERPROFILE".to_string(), profile.display().to_string()),
        ("APPDATA".to_string(), appdata.display().to_string()),
        (
            "LOCALAPPDATA".to_string(),
            local_appdata.display().to_string(),
        ),
        ("HOMEDRIVE".to_string(), system_root[..2].to_string()),
        ("HOMEPATH".to_string(), "\\".to_string()),
        (
            "USERNAME".to_string(),
            std::env::var("USERNAME").unwrap_or_else(|_| "crossport".to_string()),
        ),
        (
            "NUMBER_OF_PROCESSORS".to_string(),
            std::env::var("NUMBER_OF_PROCESSORS").unwrap_or_else(|_| "1".to_string()),
        ),
        (
            "PROCESSOR_ARCHITECTURE".to_string(),
            std::env::var("PROCESSOR_ARCHITECTURE").unwrap_or_else(|_| "AMD64".to_string()),
        ),
    ]
}

fn spawn_isolated(exe: &Path, working_directory: &Path, profile: &Path) -> Child {
    let mut command = Command::new(exe);
    command
        .current_dir(working_directory)
        .env_clear()
        .envs(scrubbed_environment(profile))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command.spawn().expect("the artifact starts")
}

/// Waits until the log holds a startup line that was not there before.
fn wait_for_startup_line(before: &str, started: Instant, profile: &Path) -> Option<Duration> {
    let deadline = Instant::now() + START_TIMEOUT;
    while Instant::now() < deadline {
        let now = read_log(profile);
        if now.len() > before.len() && now.contains("starting on windows") {
            return Some(started.elapsed());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    None
}

/// The first visible top-level window owned by `pid`.
fn visible_window_of(pid: u32) -> Option<*mut std::ffi::c_void> {
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
    };

    struct Search {
        pid: u32,
        found: *mut std::ffi::c_void,
    }

    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> windows_sys::core::BOOL {
        // SAFETY: `lparam` is the `&mut Search` passed to `EnumWindows` below,
        // and it outlives every call to this callback.
        let search = unsafe { &mut *(lparam as *mut Search) };
        let mut owner = 0u32;
        // SAFETY: `hwnd` comes from the enumeration and `owner` is writable.
        unsafe { GetWindowThreadProcessId(hwnd, &mut owner) };
        if owner == search.pid {
            // SAFETY: `hwnd` comes from the enumeration.
            let visible = unsafe { IsWindowVisible(hwnd) } != 0;
            if visible {
                search.found = hwnd;
                return 0; // Stop the enumeration.
            }
        }
        1
    }

    let mut search = Search {
        pid,
        found: std::ptr::null_mut(),
    };
    // SAFETY: the callback matches `WNDENUMPROC` and the pointer it receives
    // is valid for the duration of the call.
    unsafe {
        EnumWindows(Some(visit), (&mut search as *mut Search) as LPARAM);
    }
    (!search.found.is_null()).then_some(search.found)
}

fn wait_for_window(pid: u32, started: Instant) -> Option<Duration> {
    let deadline = Instant::now() + WINDOW_TIMEOUT;
    while Instant::now() < deadline {
        if visible_window_of(pid).is_some() {
            return Some(started.elapsed());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    None
}

fn close_window(window: *mut std::ffi::c_void) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

    // SAFETY: the handle came from the enumeration for this process's window.
    unsafe {
        PostMessageW(window, WM_CLOSE, 0, 0);
    }
}

fn wait_for_exit(child: &mut Child, timeout: Duration) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok(Some(status)) = child.try_wait() {
            return Some(status);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    None
}

#[test]
fn the_release_artifact_starts_and_stops_on_an_isolated_desktop() {
    let Some(exe) = app_exe() else {
        println!(
            "ARTIFACT skipped: no built application found. Build one with \
             `pnpm --filter desktop exec tauri build`, or set CROSSPORT_APP_EXE."
        );
        return;
    };

    let workspace = std::env::temp_dir().join(format!("crossport-artifact-{}", std::process::id()));
    let working_directory = workspace.join("app");
    let profile = workspace.join("profile");
    std::fs::create_dir_all(&working_directory).expect("the working directory is creatable");
    std::fs::create_dir_all(profile.join("Temp")).expect("the temp directory is creatable");
    std::fs::create_dir_all(profile.join("AppData").join("Local"))
        .expect("the local data directory is creatable");
    std::fs::create_dir_all(profile.join("AppData").join("Roaming"))
        .expect("the roaming data directory is creatable");

    // The executable is copied alone: a build that needed a sibling DLL or a
    // repository checkout would fail to start here.
    let deployed = working_directory.join("CrossPort.exe");
    std::fs::copy(&exe, &deployed).expect("the artifact is copyable");
    assert_eq!(
        std::fs::read_dir(&working_directory)
            .expect("the deployment directory is readable")
            .count(),
        1,
        "the artifact is deployed on its own"
    );

    let log_before = read_log(&profile);
    let started = Instant::now();
    let mut child = spawn_isolated(&deployed, &working_directory, &profile);
    let pid = child.id();

    let backend = wait_for_startup_line(&log_before, started, &profile);
    let window = wait_for_window(pid, started);

    if backend.is_none() || window.is_none() {
        let _ = child.kill();
        let _ = child.wait();
        panic!(
            "the artifact did not come up on an isolated desktop: \
             startup log={backend:?}, window={window:?}, log tail: {}",
            read_log(&profile)
                .lines()
                .last()
                .unwrap_or("<no log written>")
        );
    }
    println!(
        "ARTIFACT startup line: {}ms, window: {}ms",
        backend.expect("checked above").as_millis(),
        window.expect("checked above").as_millis()
    );

    let window = visible_window_of(pid).expect("the window is still there");
    let closing = Instant::now();
    close_window(window);
    let exit = wait_for_exit(&mut child, CLOSE_TIMEOUT);
    let Some(status) = exit else {
        let _ = child.kill();
        let _ = child.wait();
        panic!(
            "closing the window did not stop the application within {CLOSE_TIMEOUT:?}; \
             it may be holding a close guard for work it believes is in flight"
        );
    };
    println!(
        "ARTIFACT clean close: {}ms, exit code {:?}",
        closing.elapsed().as_millis(),
        status.code()
    );
    assert!(
        status.success(),
        "the application must exit cleanly when its last window closes, got {status:?}"
    );

    // The startup line is in the application's own log, in the same directory
    // a real install writes to once the launch is pointed at the real profile.
    let log_after = read_log(&profile);
    assert!(
        log_after.len() > log_before.len(),
        "the isolated run must add its own startup line to the log"
    );

    let _ = std::fs::remove_dir_all(&workspace);
}
