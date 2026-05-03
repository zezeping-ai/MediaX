import { computed, onBeforeUnmount, ref, type ComputedRef, type Ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  MEDIA_CACHE_RECORDING_STATUS_EVENT,
  type CacheRecordingStatus,
  type MediaEventEnvelope,
  type PlaybackState,
} from "@/modules/media-types";
import type { useMediaCommands } from "../useMediaCommands";

type MediaCommands = ReturnType<typeof useMediaCommands>;

type UseCacheRecordingControllerOptions = {
  commands: MediaCommands;
  currentSource: Ref<string>;
  metadataDurationSeconds: Ref<number | null>;
  playback: ComputedRef<PlaybackState | null>;
  onErrorMessage: (message: string) => void;
  onNoticeMessage: (message: string) => void;
};

export function useCacheRecordingController(options: UseCacheRecordingControllerOptions) {
  const cacheRecording = ref(false);
  const cacheOutputPath = ref("");
  const cacheFinalizedOutputPath = ref("");
  const cacheOutputDir = ref("");
  const cacheOutputSizeBytes = ref<number | null>(null);
  const cacheWriteSpeedBytesPerSecond = ref<number | null>(null);
  const cacheRecordingStartedAtMs = ref<number | null>(null);
  const cacheRecordingStartPositionSeconds = ref<number | null>(null);
  const recordingNowMs = ref(Date.now());
  const recordingClockHandle = ref<number | null>(null);
  const cacheSizeSampleBytes = ref<number | null>(null);
  const cacheSizeSampleAtMs = ref<number | null>(null);
  let unlistenCacheStatusEvent: UnlistenFn | null = null;

  const cacheRecordingElapsedSeconds = computed(() => {
    if (!cacheRecording.value) {
      return 0;
    }
    const startedAt = cacheRecordingStartedAtMs.value ?? 0;
    if (!Number.isFinite(startedAt) || startedAt <= 0) {
      return 0;
    }
    return Math.max(0, (recordingNowMs.value - startedAt) / 1000);
  });

  const effectiveDurationSeconds = computed(() => {
    const base = options.playback.value?.duration_seconds ?? options.metadataDurationSeconds.value ?? 0;
    const baseDuration = Number.isFinite(base) ? Math.max(0, base) : 0;
    if (!cacheRecording.value) {
      return baseDuration;
    }
    const source = options.currentSource.value;
    const isLiveM3u8 = /\.m3u8(\?|#|$)/i.test(source);
    const startPosition = cacheRecordingStartPositionSeconds.value ?? 0;
    const aligned = Math.max(0, startPosition) + cacheRecordingElapsedSeconds.value;
    const override = isLiveM3u8 ? aligned : cacheRecordingElapsedSeconds.value;
    return Math.max(baseDuration, override);
  });

  async function refreshCacheRecordingStatus() {
    applyCacheRecordingStatus(await options.commands.getCacheRecordingStatus());
  }

  function applyCacheRecordingStatus(status: CacheRecordingStatus) {
    cacheRecording.value = status.recording;
    cacheOutputPath.value = status.output_path ?? "";
    cacheFinalizedOutputPath.value = status.finalized_output_path ?? "";
    cacheOutputSizeBytes.value =
      typeof status.output_size_bytes === "number" && Number.isFinite(status.output_size_bytes)
        ? Math.max(0, status.output_size_bytes)
        : null;
    if (!status.recording) {
      cacheWriteSpeedBytesPerSecond.value = null;
      cacheSizeSampleBytes.value = null;
      cacheSizeSampleAtMs.value = null;
    } else if (cacheOutputSizeBytes.value !== null) {
      const now = Date.now();
      const prevSize = cacheSizeSampleBytes.value;
      const prevAt = cacheSizeSampleAtMs.value;
      if (prevSize !== null && prevAt !== null) {
        const dtSeconds = Math.max((now - prevAt) / 1000, 1e-3);
        const deltaBytes = cacheOutputSizeBytes.value - prevSize;
        cacheWriteSpeedBytesPerSecond.value = Math.max(0, deltaBytes / dtSeconds);
      }
      cacheSizeSampleBytes.value = cacheOutputSizeBytes.value;
      cacheSizeSampleAtMs.value = now;
    }
    cacheRecordingStartedAtMs.value = status.started_at_ms ?? null;
    if (!cacheRecording.value) {
      cacheRecordingStartPositionSeconds.value = null;
    }
    if (!cacheOutputDir.value && cacheOutputPath.value) {
      const idx = cacheOutputPath.value.lastIndexOf("/");
      if (idx > 0) {
        cacheOutputDir.value = cacheOutputPath.value.slice(0, idx);
      }
    }
    applyCacheStatusMessage(status.error_message);
  }

  async function toggleCacheRecording() {
    if (cacheRecording.value) {
      const stopped = await options.commands.stopCacheRecording();
      cacheRecording.value = stopped.recording;
      cacheOutputPath.value = stopped.output_path ?? "";
      cacheFinalizedOutputPath.value = stopped.finalized_output_path ?? stopped.output_path ?? "";
      cacheRecordingStartedAtMs.value = stopped.started_at_ms ?? null;
      cacheRecordingStartPositionSeconds.value = null;
      stopRecordingClock();
      applyCacheStatusMessage(stopped.error_message);
      return;
    }
    const selected = await open({
      title: "选择缓存输出目录",
      directory: true,
      multiple: false,
    });
    if (!selected || Array.isArray(selected)) {
      return;
    }
    cacheOutputDir.value = String(selected);
    const started = await options.commands.startCacheRecording(selected);
    cacheRecording.value = started.recording;
    cacheOutputPath.value = started.output_path ?? "";
    cacheFinalizedOutputPath.value = "";
    cacheOutputSizeBytes.value = started.output_size_bytes ?? 0;
    cacheWriteSpeedBytesPerSecond.value = null;
    cacheSizeSampleBytes.value =
      typeof started.output_size_bytes === "number" && Number.isFinite(started.output_size_bytes)
        ? Math.max(0, started.output_size_bytes)
        : 0;
    cacheSizeSampleAtMs.value = Date.now();
    cacheRecordingStartedAtMs.value = started.started_at_ms ?? null;
    cacheRecordingStartPositionSeconds.value = options.playback.value?.position_seconds ?? 0;
    if (cacheRecording.value) {
      startRecordingClock();
    }
  }

  function startCacheStatusPoll() {
    return;
  }

  function stopCacheStatusPoll() {
    return;
  }

  function startRecordingClock() {
    if (recordingClockHandle.value !== null) {
      return;
    }
    recordingNowMs.value = Date.now();
    recordingClockHandle.value = window.setInterval(() => {
      recordingNowMs.value = Date.now();
    }, 500);
  }

  function stopRecordingClock() {
    if (recordingClockHandle.value === null) {
      return;
    }
    window.clearInterval(recordingClockHandle.value);
    recordingClockHandle.value = null;
  }

  void listen<
    MediaEventEnvelope<CacheRecordingStatus> | CacheRecordingStatus
  >(MEDIA_CACHE_RECORDING_STATUS_EVENT, (event) => {
    applyCacheRecordingStatus(resolveCacheStatusPayload(event.payload));
  }).then((unlisten) => {
    unlistenCacheStatusEvent = unlisten;
  });

  onBeforeUnmount(() => {
    if (cacheRecording.value) {
      void options.commands.stopCacheRecording();
    }
    stopRecordingClock();
    unlistenCacheStatusEvent?.();
    unlistenCacheStatusEvent = null;
  });

  return {
    cacheFinalizedOutputPath,
    cacheOutputDir,
    cacheOutputPath,
    cacheOutputSizeBytes,
    cacheRecording,
    cacheWriteSpeedBytesPerSecond,
    effectiveDurationSeconds,
    refreshCacheRecordingStatus,
    startCacheStatusPoll,
    startRecordingClock,
    stopCacheStatusPoll,
    stopRecordingClock,
    toggleCacheRecording,
  };

  function applyCacheStatusMessage(message?: string | null) {
    if (!message) {
      return;
    }
    if (message.includes("录制已自动停止")) {
      options.onNoticeMessage(message);
      return;
    }
    options.onErrorMessage(message);
  }
}

function resolveCacheStatusPayload(
  payload: CacheRecordingStatus | MediaEventEnvelope<CacheRecordingStatus>,
) {
  if (payload && typeof payload === "object" && "payload" in payload) {
    return payload.payload;
  }
  return payload;
}
