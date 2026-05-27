use codex_turn_notify::{
    forward_to_original_client, payload_from_args, payload_from_stdin_if_piped,
    record_notify_payload,
};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    let arg_payload = payload_from_args(args.iter().map(String::as_str));
    let payload = match arg_payload {
        Some(payload) => Some(payload),
        None => payload_from_stdin_if_piped()?,
    };

    let store = record_notify_payload(payload.as_deref())?;
    let codex_home = store
        .paths()
        .status_file
        .parent()
        .map(ToOwned::to_owned)
        .ok_or("missing Codex home directory")?;

    forward_to_original_client(&codex_home, payload.as_deref(), &args[1..]);
    Ok(())
}
