use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StatusError {
    #[error("status file I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("status JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IPC frame failed: {0}")]
    IpcFrame(String),
    #[error("could not resolve Codex home directory")]
    MissingCodexHome,
}

pub type Result<T> = std::result::Result<T, StatusError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusPaths {
    pub status_file: PathBuf,
    pub event_file: PathBuf,
}

impl StatusPaths {
    pub fn default() -> Result<Self> {
        let root = match env::var_os("CODEX_HOME") {
            Some(value) => PathBuf::from(value),
            None => default_home_dir()
                .ok_or(StatusError::MissingCodexHome)?
                .join(".codex"),
        };

        Ok(Self::from_codex_home(root))
    }

    pub fn from_codex_home<P: AsRef<Path>>(root: P) -> Self {
        let root = root.as_ref();
        Self {
            status_file: root.join("codex-turn-status.json"),
            event_file: root.join("codex-turn-status-event.json"),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusState {
    Idle,
    NeedsAttention,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayStatus {
    pub state: StatusState,
    pub title: String,
    pub detail: String,
    pub cwd: Option<String>,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub updated_at: Option<String>,
}

impl DisplayStatus {
    pub fn idle() -> Self {
        Self {
            state: StatusState::Idle,
            title: "Codex idle".to_string(),
            detail: "No completed turn needs attention.".to_string(),
            cwd: None,
            thread_id: None,
            turn_id: None,
            updated_at: None,
        }
    }

    pub fn error() -> Self {
        Self {
            state: StatusState::Error,
            title: "Codex status unavailable".to_string(),
            detail: "The status file could not be read.".to_string(),
            cwd: None,
            thread_id: None,
            turn_id: None,
            updated_at: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct StatusStore {
    paths: StatusPaths,
}

impl StatusStore {
    pub fn new(paths: StatusPaths) -> Self {
        Self { paths }
    }

    pub fn default() -> Result<Self> {
        Ok(Self::new(StatusPaths::default()?))
    }

    pub fn paths(&self) -> &StatusPaths {
        &self.paths
    }

    pub fn load_display_status(&self) -> DisplayStatus {
        if !self.paths.status_file.exists() {
            return DisplayStatus::idle();
        }

        let snapshot = fs::read_to_string(&self.paths.status_file)
            .ok()
            .and_then(|value| serde_json::from_str::<StatusSnapshot>(&value).ok());

        match snapshot {
            Some(snapshot) => self.display_status_from_snapshot(snapshot),
            None => DisplayStatus::error(),
        }
    }

    pub fn record_turn_completed(&self, payload: Option<&str>) -> Result<bool> {
        if !should_record_payload(payload) {
            return Ok(false);
        }

        ensure_parent_dir(&self.paths.status_file)?;

        if let Some(payload) = payload.map(str::trim).filter(|payload| !payload.is_empty()) {
            fs::write(&self.paths.event_file, payload)?;
        }

        self.write_snapshot(&StatusSnapshot {
            state: StatusState::NeedsAttention,
            updated_at: Some(now_timestamp()),
            event_path: Some(self.paths.event_file.to_string_lossy().to_string()),
        })?;
        Ok(true)
    }

    pub fn mark_handled(&self) -> Result<()> {
        self.write_snapshot(&StatusSnapshot {
            state: StatusState::Idle,
            updated_at: Some(now_timestamp()),
            event_path: None,
        })
    }

    fn display_status_from_snapshot(&self, snapshot: StatusSnapshot) -> DisplayStatus {
        match snapshot.state {
            StatusState::Idle => DisplayStatus::idle(),
            StatusState::Error => DisplayStatus::error(),
            StatusState::NeedsAttention => {
                let event = self.load_event(snapshot.event_path.as_deref());
                DisplayStatus {
                    state: StatusState::NeedsAttention,
                    title: "Codex needs attention".to_string(),
                    detail: normalized_detail(
                        event
                            .as_ref()
                            .and_then(|event| event.last_assistant_message.as_deref()),
                    ),
                    cwd: event.as_ref().and_then(|event| event.cwd.clone()),
                    thread_id: event.as_ref().and_then(|event| event.thread_id.clone()),
                    turn_id: event.as_ref().and_then(|event| event.turn_id.clone()),
                    updated_at: snapshot.updated_at,
                }
            }
        }
    }

    fn load_event(&self, path: Option<&str>) -> Option<CodexNotifyEvent> {
        let path = path
            .map(PathBuf::from)
            .unwrap_or_else(|| self.paths.event_file.clone());
        let value = fs::read_to_string(path).ok()?;
        serde_json::from_str(&value).ok()
    }

    fn write_snapshot(&self, snapshot: &StatusSnapshot) -> Result<()> {
        ensure_parent_dir(&self.paths.status_file)?;
        let data = serde_json::to_vec(snapshot)?;
        let temp_path = self.paths.status_file.with_extension("json.tmp");
        fs::write(&temp_path, data)?;
        fs::rename(temp_path, &self.paths.status_file)?;
        Ok(())
    }
}

pub fn sync_notify_config(existing: &str, notify_target: &str) -> String {
    let notify_line = format!("notify = [\"{}\"]", escape_toml_string(notify_target));
    let mut replaced = false;
    let mut lines = Vec::new();

    for line in existing.lines() {
        if line.starts_with("notify = ") {
            lines.push(notify_line.clone());
            replaced = true;
        } else {
            lines.push(line.to_string());
        }
    }

    let mut updated = if replaced {
        lines.join("\n")
    } else if existing.trim().is_empty() {
        notify_line
    } else {
        format!("{notify_line}\n\n{}", existing.trim_end_matches('\n'))
    };
    updated.push('\n');
    updated
}

pub fn install_notify_helper(
    codex_home: impl AsRef<Path>,
    helper_source: impl AsRef<Path>,
    helper_name: &str,
) -> Result<PathBuf> {
    let codex_home = codex_home.as_ref();
    let helper_source = helper_source.as_ref();
    let target_dir = codex_home.join("bin");
    let target = target_dir.join(helper_name);

    fs::create_dir_all(&target_dir)?;
    fs::copy(helper_source, &target)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&target, fs::Permissions::from_mode(0o755))?;
    }

    let config_file = codex_home.join("config.toml");
    let existing = fs::read_to_string(&config_file).unwrap_or_default();
    let updated = sync_notify_config(&existing, &target.to_string_lossy());
    if updated != existing {
        ensure_parent_dir(&config_file)?;
        if config_file.exists() {
            let backup = config_file.with_extension("toml.bak.codex-turn-statusbar");
            let _ = fs::copy(&config_file, backup);
        }
        let temp_path = config_file.with_extension("toml.tmp");
        fs::write(&temp_path, updated)?;
        fs::rename(temp_path, config_file)?;
    }

    Ok(target)
}

fn escape_toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuBarTint {
    Idle,
    Attention,
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuBarPresentation {
    pub title: String,
    pub icon: String,
    pub tint: MenuBarTint,
    pub tooltip: String,
}

impl MenuBarPresentation {
    pub fn from_status(status: &DisplayStatus) -> Self {
        match status.state {
            StatusState::Idle => Self {
                title: String::new(),
                icon: "circle".to_string(),
                tint: MenuBarTint::Idle,
                tooltip: status.title.clone(),
            },
            StatusState::NeedsAttention => Self {
                title: String::new(),
                icon: "message-check".to_string(),
                tint: MenuBarTint::Attention,
                tooltip: status.title.clone(),
            },
            StatusState::Error => Self {
                title: String::new(),
                icon: "warning".to_string(),
                tint: MenuBarTint::Warning,
                tooltip: status.title.clone(),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuContentKey {
    pub state: StatusState,
    pub title: String,
    pub detail: String,
    pub cwd: Option<String>,
    pub updated_at: Option<String>,
    pub can_mark_handled: bool,
}

impl MenuContentKey {
    pub fn from_status(status: &DisplayStatus, can_mark_handled: bool) -> Self {
        Self {
            state: status.state,
            title: status.title.clone(),
            detail: status.detail.clone(),
            cwd: status.cwd.clone(),
            updated_at: status.updated_at.clone(),
            can_mark_handled,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MenuRefreshState {
    current: Option<MenuContentKey>,
}

impl MenuRefreshState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn should_rebuild(&mut self, next: MenuContentKey) -> bool {
        if self.current.as_ref() == Some(&next) {
            return false;
        }

        self.current = Some(next);
        true
    }
}

pub struct HandledStatePolicy;

impl HandledStatePolicy {
    pub const CODEX_BUNDLE_IDENTIFIER: &'static str = "com.openai.codex";

    pub fn should_mark_handled(status: &DisplayStatus, active_identifier: Option<&str>) -> bool {
        let _ = (status, active_identifier);
        false
    }

    pub fn is_codex_identifier(active_identifier: Option<&str>) -> bool {
        active_identifier
            .map(|value| {
                value == Self::CODEX_BUNDLE_IDENTIFIER
                    || value.to_ascii_lowercase().contains("codex")
            })
            .unwrap_or(false)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UnreadSnapshot {
    pub conversation_count: usize,
    pub inbox_count: usize,
    pub monitor_connected: bool,
    pub updated_at: Option<String>,
}

impl UnreadSnapshot {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn total_count(&self) -> usize {
        self.conversation_count + self.inbox_count
    }
}

pub fn merge_display_status(notify_status: DisplayStatus, unread: UnreadSnapshot) -> DisplayStatus {
    if unread.total_count() == 0 {
        if should_clear_notify_for_empty_unread_snapshot(&notify_status, &unread) {
            return DisplayStatus::idle();
        }
        return notify_status;
    }

    DisplayStatus {
        state: StatusState::NeedsAttention,
        title: "Codex unread".to_string(),
        detail: unread_detail(&unread),
        cwd: None,
        thread_id: None,
        turn_id: None,
        updated_at: unread.updated_at.or(notify_status.updated_at),
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UnreadTracker {
    unread_conversations: BTreeSet<String>,
    inbox_count: usize,
    monitor_connected: bool,
    updated_at: Option<String>,
}

impl UnreadTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_monitor_connected(&mut self, connected: bool, updated_at: Option<String>) {
        self.monitor_connected = connected;
        self.updated_at = updated_at.or_else(|| self.updated_at.clone());
        if !connected {
            self.unread_conversations.clear();
        }
    }

    pub fn set_inbox_count(&mut self, count: usize, updated_at: Option<String>) {
        self.inbox_count = count;
        self.updated_at = updated_at.or_else(|| self.updated_at.clone());
    }

    pub fn apply_ipc_event(&mut self, event: CodexIpcEvent, updated_at: Option<String>) {
        match event {
            CodexIpcEvent::ThreadReadStateChanged {
                conversation_id,
                has_unread_turn,
            } => {
                if has_unread_turn {
                    self.unread_conversations.insert(conversation_id);
                } else {
                    self.unread_conversations.remove(&conversation_id);
                }
            }
        }

        self.updated_at = updated_at.or_else(|| self.updated_at.clone());
    }

    pub fn snapshot(&self) -> UnreadSnapshot {
        UnreadSnapshot {
            conversation_count: self.unread_conversations.len(),
            inbox_count: self.inbox_count,
            monitor_connected: self.monitor_connected,
            updated_at: self.updated_at.clone(),
        }
    }
}

pub fn should_clear_notify_for_read_thread(
    status: &DisplayStatus,
    conversation_id: &str,
    has_unread_turn: bool,
) -> bool {
    status.state == StatusState::NeedsAttention
        && !has_unread_turn
        && status.thread_id.as_deref() == Some(conversation_id)
}

pub fn should_clear_notify_for_empty_unread_snapshot(
    status: &DisplayStatus,
    unread: &UnreadSnapshot,
) -> bool {
    status.state == StatusState::NeedsAttention
        && unread.monitor_connected
        && unread.total_count() == 0
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodexIpcEvent {
    ThreadReadStateChanged {
        conversation_id: String,
        has_unread_turn: bool,
    },
}

const IPC_FRAME_HEADER_BYTES: usize = 4;
const IPC_MAX_FRAME_BYTES: usize = 256 * 1024 * 1024;

pub fn encode_ipc_message(value: &Value) -> Result<Vec<u8>> {
    let payload = serde_json::to_vec(value)?;
    if payload.len() > IPC_MAX_FRAME_BYTES {
        return Err(StatusError::IpcFrame(format!(
            "frame exceeded limit: {} bytes",
            payload.len()
        )));
    }

    let mut frame = Vec::with_capacity(IPC_FRAME_HEADER_BYTES + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

#[derive(Clone, Debug, Default)]
pub struct CodexIpcFrameDecoder {
    buffer: Vec<u8>,
    expected_len: Option<usize>,
}

impl CodexIpcFrameDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Value>> {
        self.buffer.extend_from_slice(bytes);
        let mut messages = Vec::new();

        loop {
            if self.expected_len.is_none() {
                if self.buffer.len() < IPC_FRAME_HEADER_BYTES {
                    break;
                }

                let len = u32::from_le_bytes([
                    self.buffer[0],
                    self.buffer[1],
                    self.buffer[2],
                    self.buffer[3],
                ]) as usize;
                self.buffer.drain(..IPC_FRAME_HEADER_BYTES);

                if len > IPC_MAX_FRAME_BYTES {
                    return Err(StatusError::IpcFrame(format!(
                        "frame exceeded limit: {len} bytes"
                    )));
                }

                self.expected_len = Some(len);
            }

            let expected_len = self.expected_len.unwrap_or_default();
            if self.buffer.len() < expected_len {
                break;
            }

            let payload = self.buffer.drain(..expected_len).collect::<Vec<_>>();
            self.expected_len = None;
            messages.push(serde_json::from_slice::<Value>(&payload)?);
        }

        Ok(messages)
    }
}

impl CodexIpcEvent {
    pub fn from_json_str(message: &str) -> Result<Option<Self>> {
        let value = serde_json::from_str::<Value>(message)?;
        Ok(Self::from_value(&value))
    }

    pub fn from_value(value: &Value) -> Option<Self> {
        if value.get("type").and_then(Value::as_str) != Some("broadcast") {
            return None;
        }

        match value.get("method").and_then(Value::as_str)? {
            "thread-read-state-changed" | "thread-stream-state-changed" => {
                let params = value.get("params")?;
                let conversation_id = read_string_at_paths(
                    params,
                    &[
                        &["conversationId"],
                        &["conversationState", "conversationId"],
                        &["conversationState", "id"],
                    ],
                )?;
                let has_unread_turn = read_bool_at_paths(
                    params,
                    &[&["hasUnreadTurn"], &["conversationState", "hasUnreadTurn"]],
                )?;

                Some(Self::ThreadReadStateChanged {
                    conversation_id,
                    has_unread_turn,
                })
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CodexNotifyEvent {
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    #[serde(rename = "thread-id")]
    pub thread_id: Option<String>,
    #[serde(rename = "turn-id")]
    pub turn_id: Option<String>,
    pub cwd: Option<String>,
    #[serde(rename = "last-assistant-message")]
    pub last_assistant_message: Option<String>,
}

pub fn should_record_payload(payload: Option<&str>) -> bool {
    let Some(payload) = payload.map(str::trim).filter(|payload| !payload.is_empty()) else {
        return true;
    };

    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        return true;
    };

    !is_internal_authorization_event(&value)
}

fn is_internal_authorization_event(value: &Value) -> bool {
    let Some(message) = value
        .get("last-assistant-message")
        .and_then(Value::as_str)
        .map(str::trim)
    else {
        return false;
    };

    let Ok(message_value) = serde_json::from_str::<Value>(message) else {
        return false;
    };

    let Some(object) = message_value.as_object() else {
        return false;
    };

    ["risk_level", "user_authorization", "outcome", "rationale"]
        .iter()
        .all(|key| object.contains_key(*key))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StatusSnapshot {
    state: StatusState,
    #[serde(rename = "updatedAt")]
    updated_at: Option<String>,
    #[serde(rename = "eventPath")]
    event_path: Option<String>,
}

fn normalized_detail(message: Option<&str>) -> String {
    let Some(message) = message else {
        return "A Codex turn completed.".to_string();
    };

    let collapsed = message.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return "A Codex turn completed.".to_string();
    }

    if collapsed.chars().count() <= 180 {
        return collapsed;
    }

    collapsed.chars().take(177).collect::<String>() + "..."
}

fn unread_detail(unread: &UnreadSnapshot) -> String {
    let mut parts = Vec::new();

    match unread.conversation_count {
        0 => {}
        1 => parts.push("1 unread conversation".to_string()),
        count => parts.push(format!("{count} unread conversations")),
    }

    match unread.inbox_count {
        0 => {}
        1 => parts.push("1 unread inbox item".to_string()),
        count => parts.push(format!("{count} unread inbox items")),
    }

    if parts.is_empty() {
        "No unread Codex activity.".to_string()
    } else {
        parts.join(" and ") + "."
    }
}

fn read_string_at_paths(value: &Value, paths: &[&[&str]]) -> Option<String> {
    paths
        .iter()
        .find_map(|path| read_path(value, path).and_then(Value::as_str))
        .map(ToOwned::to_owned)
}

fn read_bool_at_paths(value: &Value, paths: &[&[&str]]) -> Option<bool> {
    paths
        .iter()
        .find_map(|path| read_path(value, path).and_then(Value::as_bool))
}

fn read_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
}

fn now_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn default_home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}
