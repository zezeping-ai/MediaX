#!/usr/bin/env bash
# Local A/V regression: open each media under a folder, exercise rate/seek/pause via autoprobe,
# then scan playback-debug.log for hard errors.
# Usage:
#   ./scripts/mediax_av_smoke.sh [DIR]
#   MEDIAX_BIN=/path/to/mediax ./scripts/mediax_av_smoke.sh
#   MEDIAX_SMOKE_RUN_SEC=35 ./scripts/mediax_av_smoke.sh --quick   # only files < 120MiB
#   MEDIAX_NETWORK_SOURCES="https://...mp4,http://...m3u8" ./scripts/mediax_av_smoke.sh
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${MEDIAX_BIN:-$ROOT/src-tauri/target/debug/mediax}"
QUICK=false
if [[ "${1:-}" == "--quick" ]]; then
  QUICK=true
  shift || true
fi
DIR="${1:-$HOME/Downloads/音视频测试}"
LOGDIR="$HOME/Library/Logs/com.zezeping.mediax"
LOGFILE="$LOGDIR/playback-debug.log"
ACTIONS="${MEDIAX_AUTOPROBE_ACTIONS:-wait:1200;seek:1;wait:500;rate:1.5;wait:500;rate:1;wait:400;seek_by:5;wait:400;pause:1;wait:250;resume:1;wait:400}"
RUN_SEC="${MEDIAX_SMOKE_RUN_SEC:-22}"
NETWORK_SOURCES="${MEDIAX_NETWORK_SOURCES:-https://h265web.com/resource/hevc_test_moov_set_head_16s.mp4,http://devimages.apple.com/iphone/samples/bipbop/gear1/prog_index.m3u8}"

if [[ ! -f "$BIN" ]]; then
  echo "mediax binary not found: $BIN (build with: cd $ROOT/src-tauri && cargo build --bin mediax)" >&2
  exit 1
fi
if [[ ! -d "$DIR" ]]; then
  echo "test directory missing: $DIR" >&2
  exit 1
fi

mkdir -p "$LOGDIR"
rm -f "$LOGFILE" "${LOGFILE%.log}.1.log" 2>/dev/null || true

failed=0
local_total=0
local_ok=0
local_fail=0
network_total=0
network_ok=0
network_fail=0
shopt -s nullglob
for f in "$DIR"/*; do
  [[ -f "$f" ]] || continue
  base=$(basename "$f")
  ext_lc="${base##*.}"
  ext_lc=$(printf '%s' "$ext_lc" | tr '[:upper:]' '[:lower:]')
  case "$ext_lc" in mp4|mkv|mov|webm|flac|wav|m4a|mp3) ;; *) continue ;; esac

  size=0
  if stat -f%z "$f" &>/dev/null; then
    size=$(stat -f%z "$f")
  elif stat -c%s "$f" &>/dev/null; then
    size=$(stat -c%s "$f")
  fi
  max=$((120 * 1024 * 1024))
  if $QUICK && [[ "$size" -gt "$max" ]]; then
    echo "[skip] $base (>${max}b quick limit)"
    continue
  fi
  local_total=$((local_total + 1))
  echo "---- $base ----"
  set +e
  MEDIAX_AUTOPROBE_SOURCE="$f" MEDIAX_AUTOPROBE_ACTIONS="$ACTIONS" \
    "$BIN" >/tmp/mediax_smoke.out 2>&1 &
  pid=$!
  sleep "$RUN_SEC"
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  set -e
  if grep -E 'autoprobe_error|decode_error|decode_error_detail' "$LOGFILE" 2>/dev/null | tail -n 5; then
    echo "[fail] errors in log for $base" >&2
    failed=1
    local_fail=$((local_fail + 1))
  else
    echo "[ok] $base"
    local_ok=$((local_ok + 1))
  fi
done

IFS=',' read -r -a network_sources <<< "$NETWORK_SOURCES"
for src in "${network_sources[@]}"; do
  src="$(printf '%s' "$src" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
  [[ -n "$src" ]] || continue
  network_total=$((network_total + 1))
  base="$src"
  if [[ "$src" == *"/"* ]]; then
    base="${src##*/}"
  fi
  echo "---- net:$base ----"
  set +e
  MEDIAX_AUTOPROBE_SOURCE="$src" MEDIAX_AUTOPROBE_ACTIONS="$ACTIONS" \
    "$BIN" >/tmp/mediax_smoke.out 2>&1 &
  pid=$!
  sleep "$RUN_SEC"
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  set -e
  if grep -E 'autoprobe_error|decode_error|decode_error_detail' "$LOGFILE" 2>/dev/null | tail -n 5; then
    echo "[fail] errors in log for net:$base" >&2
    failed=1
    network_fail=$((network_fail + 1))
  else
    echo "[ok] net:$base"
    network_ok=$((network_ok + 1))
  fi
done

echo "---- summary ----"
echo "local: total=$local_total ok=$local_ok fail=$local_fail"
echo "network: total=$network_total ok=$network_ok fail=$network_fail"

if [[ "$failed" != 0 ]]; then
  echo "smoke finished with failures; see $LOGFILE" >&2
  exit 1
fi
echo "all runs clean; log: $LOGFILE"
exit 0
