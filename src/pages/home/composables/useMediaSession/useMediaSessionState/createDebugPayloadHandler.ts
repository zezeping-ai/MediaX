import type { MediaDebugPayload } from "@/modules/media-types";
import { MAX_DEBUG_TIMELINE_SIZE } from "./constants";
import type { DebugPayloadHandler, MediaSessionStateRefs } from "./types";

export function createDebugPayloadHandler(state: MediaSessionStateRefs): DebugPayloadHandler {
  return (payload: MediaDebugPayload) => {
    const stage = payload.stage?.trim() || "debug";
    const message = payload.message?.trim() || "";
    const atMs = payload.at_ms ?? Date.now();
    state.debugTimeline.value = [
      ...state.debugTimeline.value,
      { stage, message: message || "-", at_ms: atMs },
    ].slice(-MAX_DEBUG_TIMELINE_SIZE);
    state.debugSnapshot.value = {
      ...state.debugSnapshot.value,
      [stage]: message || "-",
    };
    state.debugStageSnapshot.value = {
      ...state.debugStageSnapshot.value,
      [stage]: {
        message: message || "-",
        at_ms: atMs,
      },
    };
    if (
      state.firstFrameAtMs.value === null
      && (stage === "first_frame" || stage === "video_frame_format" || stage === "video_fps")
    ) {
      state.firstFrameAtMs.value = atMs;
    }
  };
}
