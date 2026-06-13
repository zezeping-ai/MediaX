#!/usr/bin/env bash
set -euo pipefail

LOG_PATH="${1:-${HOME}/Library/Logs/com.zezeping.mediax/playback-debug.log}"
SCENARIO="${2:-unknown}"

if [[ ! -f "$LOG_PATH" ]]; then
  echo "error: log file not found: $LOG_PATH" >&2
  exit 1
fi

python3 - "$LOG_PATH" "$SCENARIO" <<'PY'
import re
import sys
from statistics import mean

log_path, scenario = sys.argv[1], sys.argv[2]
lines = open(log_path, encoding="utf-8", errors="replace").read().splitlines()

sync_lines = [line for line in lines if "sync_telemetry:" in line]
seek_lines = [line for line in lines if "] seek:" in line]
error_lines = [
    line
    for line in lines
    if any(token in line for token in ("error", "backpressure", "video_gap", "audio_seek_drop"))
]

def parse_field(message: str, key: str):
    match = re.search(rf"{re.escape(key)}=([^ ]+)", message)
    if not match:
        return None
    raw = match.group(1).rstrip("s")
    if raw == "-":
        return None
    try:
        return float(raw)
    except ValueError:
        return None

drifts = []
present_drifts = []
lags = []
gaps = []
jitter_max = []
samples = 0

for line in sync_lines:
    _, _, message = line.partition("sync_telemetry: ")
    clock = parse_field(message, "clock")
    if scenario == "steady" and clock is not None and clock < 3.0:
        continue
    decode_lead = parse_field(message, "av_offset_ms")
    if decode_lead is None:
        decode_lead = parse_field(message, "present_drift_ms")
    if decode_lead is None:
        decode_lead = parse_field(message, "decode_lead_ms")
    if decode_lead is None:
        decode_lead = parse_field(message, "drift_ms")
    present_drift = parse_field(message, "av_offset_ms")
    if present_drift is None:
        present_drift = parse_field(message, "present_drift_ms")
    lag = parse_field(message, "present_lag_ms")
    gap = parse_field(message, "gap_ms")
    jitter = parse_field(message, "jitter_max_ms")
    if decode_lead is not None:
        drifts.append(decode_lead)
    if present_drift is not None:
        present_drifts.append(present_drift)
    if lag is not None:
        lags.append(lag)
    if gap is not None:
        gaps.append(gap)
    if jitter is not None:
        jitter_max.append(jitter)
    samples += 1

print(f"scenario: {scenario}")
print(f"log_lines: {len(lines)}")
print(f"sync_samples: {samples}")
print(f"seek_events: {len(seek_lines)}")
print(f"anomaly_events: {len(error_lines)}")

if not sync_lines:
    print("result: FAIL - no sync_telemetry samples; probe may not have reached steady playback")
    sys.exit(2)

def summarize(name, values):
    if not values:
        print(f"{name}: no data")
        return
    abs_values = [abs(v) for v in values]
    print(
        f"{name}: count={len(values)} "
        f"avg={mean(values):.1f} max_abs={max(abs_values):.1f} "
        f"p95_abs={sorted(abs_values)[int(0.95 * (len(abs_values) - 1))]:.1f}"
    )

summarize("av_offset_ms", drifts)
summarize("present_drift_ms", present_drifts)
summarize("present_lag_ms", lags)
summarize("gap_ms", gaps)
summarize("jitter_max_ms", jitter_max)

issues = []
if drifts:
    max_abs_decode_lead = max(abs(v) for v in drifts)
    if max_abs_decode_lead > 300:
        issues.append(f"decode lead exceeded 300ms (max {max_abs_decode_lead:.1f}ms)")
if present_drifts:
    steady_drifts = present_drifts
    max_abs_present = max(abs(v) for v in steady_drifts)
    p95_abs_present = sorted(abs(v) for v in steady_drifts)[
        int(0.95 * (len(steady_drifts) - 1))
    ]
    if max_abs_present > 100:
        issues.append(f"presented audio/video drift exceeded 100ms (max {max_abs_present:.1f}ms)")
    elif max_abs_present > 50:
        issues.append(f"mild presented audio/video drift detected (max {max_abs_present:.1f}ms)")
    elif p95_abs_present > 35:
        issues.append(f"steady-state present drift p95={p95_abs_present:.1f}ms (within ~1 frame)")
elif drifts:
    max_abs_decode_lead = max(abs(v) for v in drifts)
    if max_abs_decode_lead > 100:
        issues.append(
            "no presented frame telemetry; decode_lead_ms is buffered decode offset, not lip-sync drift"
        )
if lags:
    max_lag = max(lags)
    if max_lag > 80:
        issues.append(f"render present lag exceeded 80ms (max {max_lag:.1f}ms)")
if jitter_max:
    max_jitter = max(jitter_max)
    if max_jitter > 60:
        issues.append(f"video pts jitter exceeded 60ms (max {max_jitter:.1f}ms)")
if error_lines:
    issues.append(f"{len(error_lines)} anomaly event(s) in log")

if scenario == "seek" and present_drifts and max(abs(v) for v in present_drifts) > 50:
    issues.append("post-seek presented drift spike observed; check settle window and audio clock rebuild")
elif scenario == "seek" and drifts and max(abs(v) for v in drifts) > 50:
    issues.append("post-seek decode lead spike observed; may be transient during settle")

if issues:
    print("result: WARN")
    for item in issues:
        print(f"- {item}")
else:
    print("result: PASS - sync metrics within expected thresholds")

if seek_lines:
    print("seek timeline:")
    for line in seek_lines:
        print(f"  {line}")
PY
