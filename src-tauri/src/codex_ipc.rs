use chrono::{SecondsFormat, Utc};
use codex_turn_status_core::{
    encode_ipc_message, should_clear_notify_for_read_thread, CodexIpcEvent, CodexIpcFrameDecoder,
    StatusStore, UnreadTracker,
};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CLIENT_TYPE: &str = "codex-turn-statusbar";
const RECONNECT_DELAY: Duration = Duration::from_secs(1);
const INBOX_POLL_DELAY: Duration = Duration::from_secs(5);

pub fn start_unread_monitor(store: StatusStore, tracker: Arc<Mutex<UnreadTracker>>) {
    start_ipc_monitor(store, tracker.clone());
    start_inbox_poll(tracker);
}

fn start_ipc_monitor(store: StatusStore, tracker: Arc<Mutex<UnreadTracker>>) {
    thread::spawn(move || loop {
        if let Err(_error) = run_ipc_session(&store, &tracker) {
            set_monitor_connected(&tracker, false);
            thread::sleep(RECONNECT_DELAY);
        }
    });
}

#[cfg(target_os = "macos")]
fn run_ipc_session(
    store: &StatusStore,
    tracker: &Arc<Mutex<UnreadTracker>>,
) -> std::io::Result<()> {
    use std::os::unix::net::UnixStream;

    let socket_path = resolve_socket_path()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "missing Codex socket"))?;
    let mut stream = UnixStream::connect(socket_path)?;
    stream.write_all(&initialize_frame())?;
    set_monitor_connected(tracker, true);

    let mut decoder = CodexIpcFrameDecoder::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Codex IPC closed",
            ));
        }

        let messages = decoder.push(&buffer[..read]).map_err(to_io_error)?;
        for message in messages {
            handle_ipc_message(store, tracker, &mut stream, message)?;
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn run_ipc_session(
    _store: &StatusStore,
    _tracker: &Arc<Mutex<UnreadTracker>>,
) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Codex IPC monitor is not implemented for this platform yet",
    ))
}

fn handle_ipc_message<W: Write>(
    store: &StatusStore,
    tracker: &Arc<Mutex<UnreadTracker>>,
    writer: &mut W,
    message: Value,
) -> std::io::Result<()> {
    if message.get("type").and_then(Value::as_str) == Some("client-discovery-request") {
        if let Some(request_id) = message.get("requestId").and_then(Value::as_str) {
            let response = json!({
                "type": "client-discovery-response",
                "requestId": request_id,
                "response": {"canHandle": false}
            });
            writer.write_all(&encode_ipc_message(&response).map_err(to_io_error)?)?;
        }
        return Ok(());
    }

    let Some(event) = CodexIpcEvent::from_value(&message) else {
        return Ok(());
    };

    let (conversation_id, has_unread_turn) = match &event {
        CodexIpcEvent::ThreadReadStateChanged {
            conversation_id,
            has_unread_turn,
        } => (conversation_id.clone(), *has_unread_turn),
    };

    if let Ok(mut tracker) = tracker.lock() {
        tracker.apply_ipc_event(event, Some(now_timestamp()));
    }

    let status = store.load_display_status();
    if should_clear_notify_for_read_thread(&status, &conversation_id, has_unread_turn) {
        let _ = store.mark_handled();
    }

    Ok(())
}

fn start_inbox_poll(tracker: Arc<Mutex<UnreadTracker>>) {
    thread::spawn(move || loop {
        let count = unread_inbox_count().unwrap_or(0);
        if let Ok(mut tracker) = tracker.lock() {
            tracker.set_inbox_count(count, Some(now_timestamp()));
        }
        thread::sleep(INBOX_POLL_DELAY);
    });
}

fn unread_inbox_count() -> Option<usize> {
    let db_path = codex_sqlite_path()?;
    let query = "\
SELECT \
  (SELECT COUNT(*) FROM automation_runs WHERE read_at IS NULL) + \
  (SELECT COUNT(*) FROM inbox_items WHERE read_at IS NULL);";
    let output = Command::new("sqlite3")
        .arg(db_path)
        .arg(query)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout)
        .ok()?
        .trim()
        .parse::<usize>()
        .ok()
}

fn codex_sqlite_path() -> Option<PathBuf> {
    let codex_home = env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))?;

    [
        codex_home.join("sqlite/codex.db"),
        codex_home.join("sqlite/codex-dev.db"),
    ]
    .into_iter()
    .find(|path| path.exists())
}

#[cfg(target_os = "macos")]
fn resolve_socket_path() -> Option<PathBuf> {
    let socket_dir = env::temp_dir().join("codex-ipc");
    if let Some(uid) = current_uid() {
        let path = socket_dir.join(format!("ipc-{uid}.sock"));
        if path.exists() {
            return Some(path);
        }
    }

    let fallback = socket_dir.join("ipc.sock");
    if fallback.exists() {
        return Some(fallback);
    }

    fs::read_dir(socket_dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .find(|path| path.extension().and_then(|value| value.to_str()) == Some("sock"))
}

#[cfg(target_os = "macos")]
fn current_uid() -> Option<String> {
    let output = Command::new("id").arg("-u").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn initialize_frame() -> Vec<u8> {
    let request_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let message = json!({
        "type": "request",
        "requestId": format!("codex-turn-statusbar-{request_id}"),
        "sourceClientId": "initializing-client",
        "version": 0,
        "method": "initialize",
        "params": {"clientType": CLIENT_TYPE}
    });
    encode_ipc_message(&message).unwrap_or_default()
}

fn set_monitor_connected(tracker: &Arc<Mutex<UnreadTracker>>, connected: bool) {
    if let Ok(mut tracker) = tracker.lock() {
        tracker.set_monitor_connected(connected, Some(now_timestamp()));
    }
}

fn now_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn to_io_error(error: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string())
}
