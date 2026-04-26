import { computed, type Ref } from "vue";
import type { PlaybackState } from "@/modules/media-types";

type CreateMediaInfoSnapshotOptions = {
  playback: Ref<PlaybackState | null>;
  currentSource: Ref<string>;
  metadataDurationSeconds: Ref<number | null>;
  metadataVideoWidth: Ref<number | null>;
  metadataVideoHeight: Ref<number | null>;
  metadataVideoFps: Ref<number | null>;
};

export function createMediaInfoSnapshot(options: CreateMediaInfoSnapshotOptions) {
  return computed<Record<string, string>>(() => {
    const playbackState = options.playback.value;
    const source = options.currentSource.value;
    const duration = playbackState?.duration_seconds || options.metadataDurationSeconds.value || 0;
    const width = options.metadataVideoWidth.value || 0;
    const height = options.metadataVideoHeight.value || 0;
    const fps = options.metadataVideoFps.value || 0;

    const record: Record<string, string> = {};
    if (source) record.source = source;
    if (playbackState?.engine) record.engine = playbackState.engine;
    if (duration > 0) record.duration = `${duration.toFixed(3)}s`;
    if (width > 0 && height > 0) record.resolution = `${width}x${height}`;
    if (fps > 0) record.fps = `${fps.toFixed(3)}fps`;
    if (playbackState?.quality_mode) record.quality = playbackState.quality_mode;
    return record;
  });
}
