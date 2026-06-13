import { onBeforeUnmount, ref, watch, type Ref } from "vue";
import type { PlaybackState } from "@/modules/media-types";

const OVERLAY_ACTIVITY_TICK_MS = 100;

type UseLyricsPlaybackTickOptions = {
  enabled: Readonly<Ref<boolean>>;
  playback: Readonly<Ref<PlaybackState | null>>;
  onTick?: (elapsedMs: number) => void;
};

/** 播放中平滑插值 position，供歌词行切换与频谱衰减共用 */
export function useLyricsPlaybackTick(options: UseLyricsPlaybackTickOptions) {
  const interpolatedPosition = ref(0);
  let activityTimer: number | null = null;
  let lastTickAt = Date.now();

  function stopActivityTicker() {
    if (activityTimer === null) {
      return;
    }
    window.clearTimeout(activityTimer);
    activityTimer = null;
  }

  function scheduleActivityTick() {
    if (activityTimer !== null) {
      return;
    }
    activityTimer = window.setTimeout(() => {
      activityTimer = null;
      runActivityTick();
    }, OVERLAY_ACTIVITY_TICK_MS);
  }

  function runActivityTick() {
    const playback = options.playback.value;
    const now = Date.now();
    const elapsedMs = Math.max(0, now - lastTickAt);
    lastTickAt = now;
    if (!options.enabled.value || !playback || playback.status !== "playing") {
      interpolatedPosition.value = playback?.position_seconds ?? 0;
      stopActivityTicker();
      return;
    }
    const durationSeconds = playback.duration_seconds || Number.MAX_SAFE_INTEGER;
    interpolatedPosition.value = Math.min(
      durationSeconds,
      interpolatedPosition.value + (elapsedMs / 1000) * playback.playback_rate,
    );
    options.onTick?.(elapsedMs);
    scheduleActivityTick();
  }

  function syncActivityTicker() {
    lastTickAt = Date.now();
    if (options.enabled.value && options.playback.value?.status === "playing") {
      scheduleActivityTick();
      return;
    }
    stopActivityTicker();
  }

  watch(
    () => [
      options.enabled.value,
      options.playback.value?.position_seconds ?? 0,
      options.playback.value?.status ?? "idle",
      options.playback.value?.playback_rate ?? 1,
      options.playback.value?.current_path ?? "",
    ],
    () => {
      interpolatedPosition.value = options.playback.value?.position_seconds ?? 0;
      syncActivityTicker();
    },
    { immediate: true },
  );

  onBeforeUnmount(() => {
    stopActivityTicker();
  });

  return {
    interpolatedPosition,
  };
}
