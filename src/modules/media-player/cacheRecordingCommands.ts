import { invokeMediaCommand } from "../media-command";
import type { CacheRecordingStatus } from "../media-types";

export function playbackGetCacheRecordingStatus() {
  return invokeMediaCommand<CacheRecordingStatus>("playback_get_cache_recording_status");
}

export function playbackStartCacheRecording(outputDir?: string) {
  return invokeMediaCommand<CacheRecordingStatus>("playback_start_cache_recording", {
    outputDir: outputDir ?? null,
  });
}

export function playbackStopCacheRecording() {
  return invokeMediaCommand<CacheRecordingStatus>("playback_stop_cache_recording");
}
