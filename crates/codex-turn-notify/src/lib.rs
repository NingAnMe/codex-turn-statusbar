use codex_turn_status_core::{Result, StatusStore};
use std::io::{self, IsTerminal, Read};
use std::path::Path;

pub fn payload_from_args<'a>(args: impl IntoIterator<Item = &'a str>) -> Option<String> {
    args.into_iter()
        .skip(1)
        .map(str::trim)
        .find(|arg| arg.starts_with('{'))
        .map(ToOwned::to_owned)
}

pub fn payload_from_stdin_if_piped() -> io::Result<Option<String>> {
    let mut stdin = io::stdin();
    if stdin.is_terminal() {
        return Ok(None);
    }

    let mut payload = String::new();
    stdin.read_to_string(&mut payload)?;
    let payload = payload.trim();
    Ok((!payload.is_empty()).then(|| payload.to_string()))
}

pub fn record_notify_payload(payload: Option<&str>) -> Result<StatusStore> {
    let store = StatusStore::default()?;
    let _ = store.record_turn_completed(payload)?;
    Ok(store)
}

#[cfg(target_os = "macos")]
pub fn forward_to_original_client(
    codex_home: &Path,
    payload: Option<&str>,
    forwarded_args: &[String],
) {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let client = codex_home
        .join("computer-use")
        .join("Codex Computer Use.app")
        .join("Contents")
        .join("SharedSupport")
        .join("SkyComputerUseClient.app")
        .join("Contents")
        .join("MacOS")
        .join("SkyComputerUseClient");

    if !client.exists() {
        return;
    }

    let mut command = Command::new(client);
    command.arg("turn-ended").args(forwarded_args);

    if payload.is_some() {
        command.stdin(Stdio::piped());
    }

    command.stdout(Stdio::null()).stderr(Stdio::null());

    let Ok(mut child) = command.spawn() else {
        return;
    };

    if let (Some(payload), Some(mut stdin)) = (payload, child.stdin.take()) {
        let _ = stdin.write_all(payload.as_bytes());
    }
}

#[cfg(not(target_os = "macos"))]
pub fn forward_to_original_client(
    _codex_home: &Path,
    _payload: Option<&str>,
    _forwarded_args: &[String],
) {
}
