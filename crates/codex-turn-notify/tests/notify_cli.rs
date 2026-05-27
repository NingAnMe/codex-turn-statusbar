use codex_turn_notify::payload_from_args;

#[test]
fn payload_from_args_uses_json_argument() {
    let args = vec![
        "codex-turn-notify".to_string(),
        "turn-ended".to_string(),
        r#"{"turn-id":"turn-123"}"#.to_string(),
    ];

    assert_eq!(
        payload_from_args(args.iter().map(String::as_str)),
        Some(r#"{"turn-id":"turn-123"}"#.to_string())
    );
}

#[test]
fn payload_from_args_ignores_non_json_arguments() {
    let args = vec![
        "codex-turn-notify".to_string(),
        "turn-ended".to_string(),
        "--quiet".to_string(),
    ];

    assert_eq!(payload_from_args(args.iter().map(String::as_str)), None);
}
