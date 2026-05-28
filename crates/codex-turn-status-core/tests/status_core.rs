use codex_turn_status_core::{
    encode_ipc_message, merge_display_status, should_clear_notify_for_empty_unread_snapshot,
    should_clear_notify_for_read_thread, should_record_payload, sync_notify_config, CodexIpcEvent,
    CodexIpcFrameDecoder, CodexNotifyEvent, DisplayStatus, HandledStatePolicy, MenuBarPresentation,
    MenuBarTint, MenuContentKey, MenuRefreshState, StatusPaths, StatusState, StatusStore,
    UnreadSnapshot, UnreadTracker,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn status_paths_use_codex_home() {
    let root = PathBuf::from("/tmp/codex-home-for-test");
    let paths = StatusPaths::from_codex_home(&root);

    assert_eq!(paths.status_file, root.join("codex-turn-status.json"));
    assert_eq!(paths.event_file, root.join("codex-turn-status-event.json"));
}

#[test]
fn missing_status_file_loads_idle_state() {
    let dir = temp_dir();
    let store = StatusStore::new(StatusPaths::from_codex_home(&dir));

    let status = store.load_display_status();

    assert_eq!(status.state, StatusState::Idle);
    assert_eq!(status.title, "Codex idle");
}

#[test]
fn completed_turn_writes_status_and_event() {
    let dir = temp_dir();
    let paths = StatusPaths::from_codex_home(&dir);
    let store = StatusStore::new(paths.clone());
    let payload = r#"{"thread-id":"thread-123","turn-id":"turn-456","cwd":"/work","last-assistant-message":"Ready."}"#;

    assert!(store.record_turn_completed(Some(payload)).unwrap());

    let status_json = fs::read_to_string(paths.status_file).unwrap();
    assert!(status_json.contains(r#""state":"needs_attention""#));
    assert!(status_json.contains("codex-turn-status-event.json"));
    assert_eq!(fs::read_to_string(paths.event_file).unwrap(), payload);
}

#[test]
fn needs_attention_status_includes_event_details() {
    let dir = temp_dir();
    let paths = StatusPaths::from_codex_home(&dir);
    let store = StatusStore::new(paths.clone());
    store
        .record_turn_completed(Some(
            r#"{"thread-id":"thread-123","turn-id":"turn-456","cwd":"/work","last-assistant-message":"Codex finished."}"#,
        ))
        .unwrap();

    let status = store.load_display_status();

    assert_eq!(status.state, StatusState::NeedsAttention);
    assert_eq!(status.detail, "Codex finished.");
    assert_eq!(status.cwd.as_deref(), Some("/work"));
    assert_eq!(status.thread_id.as_deref(), Some("thread-123"));
    assert_eq!(status.turn_id.as_deref(), Some("turn-456"));
}

#[test]
fn mark_handled_writes_idle_status() {
    let dir = temp_dir();
    let store = StatusStore::new(StatusPaths::from_codex_home(&dir));

    store.mark_handled().unwrap();

    let status = store.load_display_status();
    assert_eq!(status.state, StatusState::Idle);
}

#[test]
fn presentation_is_icon_only_and_green_for_attention() {
    let presentation = MenuBarPresentation::from_status(&DisplayStatus {
        state: StatusState::NeedsAttention,
        title: "Codex needs attention".to_string(),
        detail: "Ready.".to_string(),
        cwd: None,
        thread_id: None,
        turn_id: None,
        updated_at: None,
    });

    assert_eq!(presentation.title, "");
    assert_eq!(presentation.tint, MenuBarTint::Attention);
    assert_eq!(presentation.icon, "message-check");
}

#[test]
fn handled_policy_no_longer_clears_when_codex_is_active() {
    let needs_attention = DisplayStatus {
        state: StatusState::NeedsAttention,
        title: "Codex needs attention".to_string(),
        detail: "Ready.".to_string(),
        cwd: None,
        thread_id: None,
        turn_id: None,
        updated_at: None,
    };
    let idle = DisplayStatus::idle();

    assert!(!HandledStatePolicy::should_mark_handled(
        &needs_attention,
        Some("com.openai.codex")
    ));
    assert!(!HandledStatePolicy::should_mark_handled(
        &needs_attention,
        Some("Codex")
    ));
    assert!(!HandledStatePolicy::should_mark_handled(
        &idle,
        Some("com.openai.codex")
    ));
    assert!(!HandledStatePolicy::should_mark_handled(
        &needs_attention,
        Some("com.apple.finder")
    ));
}

#[test]
fn unread_conversations_take_priority_over_idle_status() {
    let unread = UnreadSnapshot {
        conversation_count: 2,
        inbox_count: 0,
        monitor_connected: true,
        updated_at: Some("2026-05-27T12:00:00Z".to_string()),
    };

    let status = merge_display_status(DisplayStatus::idle(), unread);

    assert_eq!(status.state, StatusState::NeedsAttention);
    assert_eq!(status.title, "Codex unread");
    assert_eq!(status.detail, "2 unread conversations.");
    assert_eq!(status.updated_at.as_deref(), Some("2026-05-27T12:00:00Z"));
}

#[test]
fn notify_pending_remains_as_fallback_without_unread_activity() {
    let notify_status = DisplayStatus {
        state: StatusState::NeedsAttention,
        title: "Codex needs attention".to_string(),
        detail: "A turn completed.".to_string(),
        cwd: Some("/work".to_string()),
        thread_id: Some("thread-123".to_string()),
        turn_id: Some("turn-456".to_string()),
        updated_at: Some("2026-05-27T12:00:00Z".to_string()),
    };

    let status = merge_display_status(notify_status.clone(), UnreadSnapshot::empty());

    assert_eq!(status, notify_status);
}

#[test]
fn connected_empty_unread_snapshot_suppresses_stale_notify_attention() {
    let notify_status = DisplayStatus {
        state: StatusState::NeedsAttention,
        title: "Codex needs attention".to_string(),
        detail: "A turn completed.".to_string(),
        cwd: Some("/work".to_string()),
        thread_id: Some("thread-123".to_string()),
        turn_id: Some("turn-456".to_string()),
        updated_at: Some("2026-05-27T12:00:00Z".to_string()),
    };
    let unread = UnreadSnapshot {
        conversation_count: 0,
        inbox_count: 0,
        monitor_connected: true,
        updated_at: Some("2026-05-27T12:00:01Z".to_string()),
    };

    let status = merge_display_status(notify_status, unread);

    assert_eq!(status.state, StatusState::Idle);
    assert_eq!(status.title, "Codex idle");
}

#[test]
fn connected_empty_unread_snapshot_marks_stale_notify_clearable() {
    let notify_status = DisplayStatus {
        state: StatusState::NeedsAttention,
        title: "Codex needs attention".to_string(),
        detail: "A turn completed.".to_string(),
        cwd: Some("/work".to_string()),
        thread_id: Some("thread-123".to_string()),
        turn_id: Some("turn-456".to_string()),
        updated_at: Some("2026-05-27T12:00:00Z".to_string()),
    };
    let unread = UnreadSnapshot {
        conversation_count: 0,
        inbox_count: 0,
        monitor_connected: true,
        updated_at: Some("2026-05-27T12:00:01Z".to_string()),
    };

    assert!(should_clear_notify_for_empty_unread_snapshot(
        &notify_status,
        &unread
    ));
}

#[test]
fn menu_refresh_state_skips_unchanged_menu_content() {
    let status = DisplayStatus {
        state: StatusState::NeedsAttention,
        title: "Codex needs attention".to_string(),
        detail: "A turn completed.".to_string(),
        cwd: Some("/work".to_string()),
        thread_id: Some("thread-123".to_string()),
        turn_id: Some("turn-456".to_string()),
        updated_at: Some("2026-05-27T12:00:00Z".to_string()),
    };
    let key = MenuContentKey::from_status(&status, true);
    let mut refresh = MenuRefreshState::new();

    assert!(refresh.should_rebuild(key.clone()));
    assert!(!refresh.should_rebuild(key));
}

#[test]
fn menu_refresh_state_rebuilds_when_actions_change() {
    let status = DisplayStatus::idle();
    let mut refresh = MenuRefreshState::new();

    assert!(refresh.should_rebuild(MenuContentKey::from_status(&status, false)));
    assert!(refresh.should_rebuild(MenuContentKey::from_status(&status, true)));
}

#[test]
fn ipc_read_state_broadcast_decodes_unread_changes() {
    let message = r#"{
      "type": "broadcast",
      "method": "thread-read-state-changed",
      "params": {
        "conversationId": "thread-123",
        "hasUnreadTurn": true
      }
    }"#;

    let event = CodexIpcEvent::from_json_str(message).unwrap();

    assert_eq!(
        event,
        Some(CodexIpcEvent::ThreadReadStateChanged {
            conversation_id: "thread-123".to_string(),
            has_unread_turn: true
        })
    );
}

#[test]
fn ipc_stream_state_broadcast_decodes_nested_unread_snapshot() {
    let message = r#"{
      "type": "broadcast",
      "method": "thread-stream-state-changed",
      "params": {
        "conversationState": {
          "conversationId": "thread-123",
          "hasUnreadTurn": false
        }
      }
    }"#;

    let event = CodexIpcEvent::from_json_str(message).unwrap();

    assert_eq!(
        event,
        Some(CodexIpcEvent::ThreadReadStateChanged {
            conversation_id: "thread-123".to_string(),
            has_unread_turn: false
        })
    );
}

#[test]
fn ipc_frame_decoder_handles_split_and_multiple_frames() {
    let first = serde_json::json!({"type":"broadcast","method":"client-status-changed"});
    let second = serde_json::json!({
      "type": "broadcast",
      "method": "thread-read-state-changed",
      "params": {"conversationId": "thread-123", "hasUnreadTurn": true}
    });
    let mut bytes = encode_ipc_message(&first).unwrap();
    bytes.extend(encode_ipc_message(&second).unwrap());

    let mut decoder = CodexIpcFrameDecoder::new();
    let midpoint = 5;
    assert!(decoder.push(&bytes[..midpoint]).unwrap().is_empty());

    let messages = decoder.push(&bytes[midpoint..]).unwrap();

    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0], first);
    assert_eq!(messages[1], second);
}

#[test]
fn unread_tracker_counts_unread_conversations_from_ipc_events() {
    let mut tracker = UnreadTracker::new();
    tracker.set_monitor_connected(true, Some("2026-05-27T12:00:00Z".to_string()));

    tracker.apply_ipc_event(
        CodexIpcEvent::ThreadReadStateChanged {
            conversation_id: "thread-123".to_string(),
            has_unread_turn: true,
        },
        Some("2026-05-27T12:00:01Z".to_string()),
    );
    tracker.apply_ipc_event(
        CodexIpcEvent::ThreadReadStateChanged {
            conversation_id: "thread-456".to_string(),
            has_unread_turn: true,
        },
        Some("2026-05-27T12:00:02Z".to_string()),
    );
    tracker.apply_ipc_event(
        CodexIpcEvent::ThreadReadStateChanged {
            conversation_id: "thread-123".to_string(),
            has_unread_turn: false,
        },
        Some("2026-05-27T12:00:03Z".to_string()),
    );

    let snapshot = tracker.snapshot();
    assert_eq!(snapshot.conversation_count, 1);
    assert!(snapshot.monitor_connected);
    assert_eq!(snapshot.updated_at.as_deref(), Some("2026-05-27T12:00:03Z"));
}

#[test]
fn read_state_false_clears_only_matching_notify_thread() {
    let status = DisplayStatus {
        state: StatusState::NeedsAttention,
        title: "Codex needs attention".to_string(),
        detail: "Done.".to_string(),
        cwd: None,
        thread_id: Some("thread-123".to_string()),
        turn_id: None,
        updated_at: None,
    };

    assert!(should_clear_notify_for_read_thread(
        &status,
        "thread-123",
        false
    ));
    assert!(!should_clear_notify_for_read_thread(
        &status,
        "thread-456",
        false
    ));
    assert!(!should_clear_notify_for_read_thread(
        &status,
        "thread-123",
        true
    ));
}

#[test]
fn codex_event_decodes_notify_keys() {
    let event: CodexNotifyEvent = serde_json::from_str(
        r#"{"type":"agent-turn-complete","thread-id":"thread-123","turn-id":"turn-456","cwd":"/work","last-assistant-message":"Done."}"#,
    )
    .unwrap();

    assert_eq!(event.event_type.as_deref(), Some("agent-turn-complete"));
    assert_eq!(event.thread_id.as_deref(), Some("thread-123"));
    assert_eq!(event.turn_id.as_deref(), Some("turn-456"));
    assert_eq!(event.cwd.as_deref(), Some("/work"));
    assert_eq!(event.last_assistant_message.as_deref(), Some("Done."));
}

#[test]
fn internal_authorization_events_do_not_trigger_attention() {
    let payload = r#"{
      "type": "agent-turn-complete",
      "last-assistant-message": "{\"risk_level\":\"low\",\"user_authorization\":\"high\",\"outcome\":\"allow\",\"rationale\":\"safe\"}"
    }"#;

    assert!(!should_record_payload(Some(payload)));

    let dir = temp_dir();
    let paths = StatusPaths::from_codex_home(&dir);
    let store = StatusStore::new(paths.clone());
    assert!(!store.record_turn_completed(Some(payload)).unwrap());
    assert!(!paths.status_file.exists());
}

#[test]
fn sync_notify_config_inserts_notify_line_at_top() {
    let updated = sync_notify_config(
        "theme = \"dark\"\n",
        "/Users/ning/.codex/bin/codex-turn-notify",
    );

    assert_eq!(
        updated,
        "notify = [\"/Users/ning/.codex/bin/codex-turn-notify\"]\n\ntheme = \"dark\"\n"
    );
}

#[test]
fn sync_notify_config_replaces_existing_notify_line() {
    let updated = sync_notify_config(
        "theme = \"dark\"\nnotify = [\"old\"]\n",
        "/Users/ning/.codex/bin/codex-turn-notify",
    );

    assert_eq!(
        updated,
        "theme = \"dark\"\nnotify = [\"/Users/ning/.codex/bin/codex-turn-notify\"]\n"
    );
}

#[test]
fn sync_notify_config_escapes_toml_string_chars() {
    let updated = sync_notify_config("", "C:\\Users\\Ning\\\"Codex\"\\codex-turn-notify.exe");

    assert_eq!(
        updated,
        "notify = [\"C:\\\\Users\\\\Ning\\\\\\\"Codex\\\"\\\\codex-turn-notify.exe\"]\n"
    );
}

fn temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("codex-turn-status-test-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}
