import type { PlaybackState } from "@/modules/media-types";
import type { HardwareDecisionEvent } from "../types";

export function resolveHardwareCapabilityVerdict(
  playback: PlaybackState | null,
  snapshot: Record<string, string>,
): string {
  if (!playback) return "unknown";
  if (playback.hw_decode_active) {
    return `active via ${playback.hw_decode_backend || "hardware backend"}`;
  }
  if (playback.hw_decode_mode === "off") return "disabled by preference";
  if (snapshot.hw_decode_fallback) return "fallback to software after hardware attempt";
  if (playback.hw_decode_error) return `software only: ${playback.hw_decode_error}`;
  if (snapshot.hw_decode_decision) return snapshot.hw_decode_decision;
  return playback.hw_decode_mode === "on"
    ? "hardware requested; waiting for result"
    : "auto mode; waiting for decision";
}

export function resolveHardwareDecisionTone(
  stage: string,
  message: string,
): HardwareDecisionEvent["tone"] {
  if (stage === "decode_error") return "error";
  if (stage === "hw_decode_fallback") return "warn";
  if (
    stage === "hw_decode_decision" &&
    message.toLowerCase().includes("hardware decode selected")
  ) {
    return "good";
  }
  return "neutral";
}
