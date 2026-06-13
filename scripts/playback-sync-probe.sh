#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DEFAULT_SOURCE="/Users/wangsen/Downloads/h265测试文件 2/h265_high.mp4"
SOURCE="${1:-$DEFAULT_SOURCE}"
SCENARIO="${2:-steady}"
LOG_PATH="${HOME}/Library/Logs/com.zezeping.mediax/playback-debug.log"

if [[ ! -f "$SOURCE" ]]; then
  echo "error: source file not found: $SOURCE" >&2
  exit 1
fi

case "$SCENARIO" in
  steady)
    # Continuous playback without seek; enough time for sync telemetry (~1s interval).
    ACTIONS="wait:18000;exit:now"
    ;;
  seek)
    # Scrub timeline to surface post-seek transient drift.
    ACTIONS="wait:5000;seek:60;wait:3000;seek:120;wait:3000;seek:180;wait:3000;exit:now"
    ;;
  *)
    echo "error: unknown scenario '$SCENARIO' (expected: steady|seek)" >&2
    exit 1
    ;;
esac

echo "== playback sync probe =="
echo "source:   $SOURCE"
echo "scenario: $SCENARIO"
echo "actions:  $ACTIONS"
echo "log:      $LOG_PATH"
echo

cd "$ROOT_DIR"

export MEDIAX_AUTOPROBE_SOURCE="$SOURCE"
export MEDIAX_AUTOPROBE_ACTIONS="$ACTIONS"

# Autoprobe exits via exit:now; keep a hard timeout fallback for macOS (no coreutils timeout).
pnpm tauri dev &
PROBE_PID=$!
PROBE_EXIT=0
ELAPSED=0
while kill -0 "$PROBE_PID" 2>/dev/null; do
  if [[ "$ELAPSED" -ge 120 ]]; then
    echo "warning: probe timed out after 120s; terminating pid $PROBE_PID" >&2
    kill "$PROBE_PID" 2>/dev/null || true
    wait "$PROBE_PID" 2>/dev/null || true
    PROBE_EXIT=124
    break
  fi
  sleep 1
  ELAPSED=$((ELAPSED + 1))
done
if [[ "$PROBE_EXIT" -eq 0 ]]; then
  wait "$PROBE_PID" || PROBE_EXIT=$?
fi

if [[ ! -f "$LOG_PATH" ]]; then
  echo "error: playback log not found at $LOG_PATH" >&2
  exit 1
fi

echo
echo "== log analysis =="
"$ROOT_DIR/scripts/analyze-playback-sync-log.sh" "$LOG_PATH" "$SCENARIO"

if [[ "$PROBE_EXIT" -ne 0 && "$PROBE_EXIT" -ne 124 ]]; then
  echo "warning: probe process exited with code $PROBE_EXIT" >&2
fi
