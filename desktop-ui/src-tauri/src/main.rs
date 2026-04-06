use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::fs::{OpenOptions, create_dir_all};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::thread::sleep;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, State};

const DEFAULT_UI_URL: &str = "http://localhost:14564/";
const READY_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(250);
const SIDECAR_BASENAME: &str = "goxlr-daemon";

struct ManagedDaemon {
    child: Mutex<Option<Child>>,
    last_error: Mutex<Option<String>>,
}

impl Default for ManagedDaemon {
    fn default() -> Self {
        Self {
            child: Mutex::new(None),
            last_error: Mutex::new(None),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DaemonStatus {
    ready: bool,
    spawned: bool,
    url: String,
    last_error: Option<String>,
    log_dir: String,
    startup_log: String,
    daemon_stdout_log: String,
    daemon_stderr_log: String,
}

#[tauri::command]
fn get_daemon_status(state: State<'_, ManagedDaemon>) -> DaemonStatus {
    let ready = is_ui_ready();
    let spawned = daemon_is_spawned(&state);
    DaemonStatus {
        ready,
        spawned,
        url: DEFAULT_UI_URL.to_string(),
        last_error: get_last_error(&state),
        log_dir: startup_log_dir().display().to_string(),
        startup_log: startup_log_dir().join("startup.log").display().to_string(),
        daemon_stdout_log: startup_log_dir()
            .join("goxlr-daemon.stdout.log")
            .display()
            .to_string(),
        daemon_stderr_log: startup_log_dir()
            .join("goxlr-daemon.stderr.log")
            .display()
            .to_string(),
    }
}

#[tauri::command]
fn ensure_daemon_started(
    app: AppHandle,
    state: State<'_, ManagedDaemon>,
) -> std::result::Result<DaemonStatus, String> {
    log_startup("ensure_daemon_started called");

    if is_ui_ready() {
        log_startup("UI already reachable");
        clear_last_error(&state);
        return Ok(DaemonStatus {
            ready: true,
            spawned: daemon_is_spawned(&state),
            url: DEFAULT_UI_URL.to_string(),
            last_error: None,
            log_dir: startup_log_dir().display().to_string(),
            startup_log: startup_log_dir().join("startup.log").display().to_string(),
            daemon_stdout_log: startup_log_dir()
                .join("goxlr-daemon.stdout.log")
                .display()
                .to_string(),
            daemon_stderr_log: startup_log_dir()
                .join("goxlr-daemon.stderr.log")
                .display()
                .to_string(),
        });
    }

    reap_dead_child(&state);

    if !daemon_is_spawned(&state) {
        let daemon_path = locate_daemon_binary(&app).map_err(|error| {
            let message = format!("Unable to locate goxlr-daemon: {error:#}");
            log_startup(&message);
            set_last_error(&state, message.clone());
            message
        })?;
        log_startup(&format!("Using daemon binary at {}", daemon_path.display()));

        let child = spawn_daemon(&daemon_path).map_err(|error| {
            let message = format!("Unable to start goxlr-daemon: {error:#}");
            log_startup(&message);
            set_last_error(&state, message.clone());
            message
        })?;
        log_startup("Spawned daemon process");

        let mut guard = state.child.lock().expect("daemon child mutex poisoned");
        *guard = Some(child);
    }

    let started = wait_for_ui();
    if !started {
        let message = "Timed out waiting for the daemon HTTP interface at http://localhost:14564/"
            .to_string();
        log_startup(&message);
        set_last_error(&state, message.clone());
        return Err(message);
    }

    log_startup("UI became reachable");
    clear_last_error(&state);
    Ok(DaemonStatus {
        ready: true,
        spawned: true,
        url: DEFAULT_UI_URL.to_string(),
        last_error: None,
        log_dir: startup_log_dir().display().to_string(),
        startup_log: startup_log_dir().join("startup.log").display().to_string(),
        daemon_stdout_log: startup_log_dir()
            .join("goxlr-daemon.stdout.log")
            .display()
            .to_string(),
        daemon_stderr_log: startup_log_dir()
            .join("goxlr-daemon.stderr.log")
            .display()
            .to_string(),
    })
}

#[tauri::command]
fn open_log_directory() -> std::result::Result<(), String> {
    let log_dir = startup_log_dir();
    create_dir_all(&log_dir).map_err(|error| error.to_string())?;

    Command::new("open")
        .arg(&log_dir)
        .spawn()
        .map_err(|error| format!("Unable to open {}: {error}", log_dir.display()))?;

    Ok(())
}

fn main() {
    let app = tauri::Builder::default()
        .manage(ManagedDaemon::default())
        .invoke_handler(tauri::generate_handler![
            ensure_daemon_started,
            get_daemon_status,
            open_log_directory
        ])
        .build(tauri::generate_context!())
        .expect("failed to build MacXLR Desktop");

    app.run(|app_handle, event| {
        if matches!(
            event,
            tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }
        ) {
            let state = app_handle.state::<ManagedDaemon>();
            stop_daemon(&state);
        }
    });
}

fn wait_for_ui() -> bool {
    let deadline = Instant::now() + READY_TIMEOUT;
    while Instant::now() < deadline {
        if is_ui_ready() {
            return true;
        }
        sleep(POLL_INTERVAL);
    }
    false
}

fn is_ui_ready() -> bool {
    let address: SocketAddr = "127.0.0.1:14564".parse().expect("valid socket address");
    let localhost: SocketAddr = "[::1]:14564".parse().expect("valid socket address");
    TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_ok()
        || TcpStream::connect_timeout(&localhost, Duration::from_millis(200)).is_ok()
}

fn daemon_is_spawned(state: &ManagedDaemon) -> bool {
    let mut guard = state.child.lock().expect("daemon child mutex poisoned");
    if let Some(child) = guard.as_mut() {
        return child.try_wait().ok().flatten().is_none();
    }
    false
}

fn reap_dead_child(state: &ManagedDaemon) {
    let mut guard = state.child.lock().expect("daemon child mutex poisoned");
    let should_clear = guard
        .as_mut()
        .and_then(|child| child.try_wait().ok())
        .flatten()
        .is_some();

    if should_clear {
        *guard = None;
    }
}

fn stop_daemon(state: &ManagedDaemon) {
    let mut guard = state.child.lock().expect("daemon child mutex poisoned");
    if let Some(child) = guard.as_mut() {
        let _ = child.kill();
        let _ = child.wait();
    }
    *guard = None;
}

fn spawn_daemon(path: &Path) -> Result<Child> {
    let log_dir = startup_log_dir();
    create_dir_all(&log_dir).with_context(|| format!("creating {}", log_dir.display()))?;

    let stdout_path = log_dir.join("goxlr-daemon.stdout.log");
    let stderr_path = log_dir.join("goxlr-daemon.stderr.log");
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stdout_path)
        .with_context(|| format!("opening {}", stdout_path.display()))?;
    let stderr = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stderr_path)
        .with_context(|| format!("opening {}", stderr_path.display()))?;

    let mut command = Command::new(path);
    command.arg("--disable-tray");
    command.arg("true");
    command.arg("--log-level");
    command.arg("info");
    command.stdin(Stdio::null());
    command.stdout(Stdio::from(stdout));
    command.stderr(Stdio::from(stderr));

    if let Some(parent) = path.parent() {
        command.current_dir(parent);
    }

    command
        .spawn()
        .with_context(|| format!("spawning {}", path.display()))
}

fn locate_daemon_binary(app: &AppHandle) -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let host_tuple = std::env::consts::ARCH.to_string() + "-apple-darwin";
    let bundled_sidecar_name = format!("{SIDECAR_BASENAME}-{host_tuple}");
    let mut candidates = vec![
        manifest_dir.join("../../target/debug/goxlr-daemon"),
        manifest_dir.join("../../target/release/goxlr-daemon"),
        manifest_dir.join("../../dist/MacXLR.app/Contents/MacOS/goxlr-daemon"),
        manifest_dir.join(format!("../src-tauri/binaries/{bundled_sidecar_name}")),
    ];

    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("goxlr-daemon"));
        candidates.push(resource_dir.join("bin/goxlr-daemon"));
        candidates.push(resource_dir.join(&bundled_sidecar_name));
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            candidates.push(parent.join("goxlr-daemon"));
            candidates.push(parent.join(&bundled_sidecar_name));
            candidates.push(parent.join("../Resources/goxlr-daemon"));
            candidates.push(parent.join(format!("../Resources/{bundled_sidecar_name}")));
        }
    }

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join("../target/debug/goxlr-daemon"));
        candidates.push(current_dir.join("../target/release/goxlr-daemon"));
        candidates.push(current_dir.join("../dist/MacXLR.app/Contents/MacOS/goxlr-daemon"));
    }

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    bail!("searched common development and packaged app locations")
}

fn get_last_error(state: &ManagedDaemon) -> Option<String> {
    state
        .last_error
        .lock()
        .expect("daemon error mutex poisoned")
        .clone()
}

fn set_last_error(state: &ManagedDaemon, message: String) {
    let mut guard = state
        .last_error
        .lock()
        .expect("daemon error mutex poisoned");
    *guard = Some(message);
}

fn clear_last_error(state: &ManagedDaemon) {
    let mut guard = state
        .last_error
        .lock()
        .expect("daemon error mutex poisoned");
    *guard = None;
}

fn startup_log_dir() -> PathBuf {
    std::env::temp_dir().join("macxlr-desktop")
}

fn log_startup(message: &str) {
    let log_dir = startup_log_dir();
    if create_dir_all(&log_dir).is_err() {
        return;
    }

    let log_path = log_dir.join("startup.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        use std::io::Write;
        let _ = writeln!(file, "{}", message);
    }
}
