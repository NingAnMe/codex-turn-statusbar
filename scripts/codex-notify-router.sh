#!/bin/zsh
set -u

CODEX_HOME_DIR="${CODEX_HOME:-$HOME/.codex}"
STATUS_FILE="$CODEX_HOME_DIR/codex-turn-status.json"
EVENT_FILE="$CODEX_HOME_DIR/codex-turn-status-event.json"
ORIGINAL_CLIENT="$CODEX_HOME_DIR/computer-use/Codex Computer Use.app/Contents/SharedSupport/SkyComputerUseClient.app/Contents/MacOS/SkyComputerUseClient"

mkdir -p "$CODEX_HOME_DIR"

payload=""
if [[ "$#" -gt 0 ]]; then
  for arg in "$@"; do
    if [[ "$arg" == \{* ]]; then
      payload="$arg"
      break
    fi
  done
fi

if [[ -z "$payload" && ! -t 0 ]]; then
  payload="$(cat)"
fi

is_internal_authorization_event=0
if [[ "$payload" == *'"risk_level"'* &&
      "$payload" == *'"user_authorization"'* &&
      "$payload" == *'"outcome"'* &&
      "$payload" == *'"rationale"'* ]]; then
  is_internal_authorization_event=1
fi

if [[ "$is_internal_authorization_event" -eq 0 ]]; then
  if [[ -n "$payload" ]]; then
    printf "%s" "$payload" > "$EVENT_FILE"
  fi

  timestamp="$(date -u "+%Y-%m-%dT%H:%M:%SZ")"
  escaped_event_path="${EVENT_FILE//\\/\\\\}"
  escaped_event_path="${escaped_event_path//\"/\\\"}"
  tmp_file="$STATUS_FILE.$$"

  printf '{"state":"needs_attention","updatedAt":"%s","eventPath":"%s"}\n' \
    "$timestamp" \
    "$escaped_event_path" \
    > "$tmp_file"
  mv "$tmp_file" "$STATUS_FILE"
fi

if [[ -x "$ORIGINAL_CLIENT" ]]; then
  if [[ -n "$payload" ]]; then
    (printf "%s" "$payload" | "$ORIGINAL_CLIENT" turn-ended "$@" >/dev/null 2>&1) &
  else
    ("$ORIGINAL_CLIENT" turn-ended "$@" >/dev/null 2>&1) &
  fi
fi

exit 0
