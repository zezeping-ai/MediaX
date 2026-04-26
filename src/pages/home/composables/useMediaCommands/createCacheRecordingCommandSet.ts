import {
  playbackGetCacheRecordingStatus,
  playbackStartCacheRecording,
  playbackStopCacheRecording,
} from "@/modules/media-player";
import type { CacheRecordingStatus } from "@/modules/media-types";

export interface CacheRecordingCommandSet {
  getCacheRecordingStatus: () => Promise<CacheRecordingStatus>;
  startCacheRecording: (outputDir?: string) => Promise<CacheRecordingStatus>;
  stopCacheRecording: () => Promise<CacheRecordingStatus>;
}

export function createCacheRecordingCommandSet(): CacheRecordingCommandSet {
  return {
    getCacheRecordingStatus: () => playbackGetCacheRecordingStatus(),
    startCacheRecording: (outputDir) => playbackStartCacheRecording(outputDir),
    stopCacheRecording: () => playbackStopCacheRecording(),
  };
}
