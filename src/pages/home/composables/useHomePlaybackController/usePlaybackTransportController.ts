import { ref, watch, type ComputedRef } from "vue";
import { clamp } from "lodash-es";
import type { PlaybackState } from "@/modules/media-types";
import { usePlaybackShortcuts } from "./usePlaybackShortcuts";

type UsePlaybackTransportControllerOptions = {
  playback: ComputedRef<PlaybackState | null>;
  play: () => Promise<void>;
  pause: () => Promise<void>;
  stop: () => Promise<void>;
  seek: (positionSeconds: number) => Promise<void>;
  setRate: (rate: number) => Promise<void>;
  setVolume: (volume: number) => Promise<void>;
  setMuted: (muted: boolean) => Promise<void>;
};

export function usePlaybackTransportController(options: UsePlaybackTransportControllerOptions) {
  const playbackRate = ref(1);
  const volume = ref(1);
  const muted = ref(false);

  async function handlePlay() {
    await options.play();
  }

  async function handlePause(positionSeconds?: number) {
    if (typeof positionSeconds === "number" && Number.isFinite(positionSeconds)) {
      await options.seek(positionSeconds);
    }
    await options.pause();
  }

  async function handleStop() {
    if (options.playback.value?.status === "playing") {
      await handlePause(options.playback.value.position_seconds);
      return;
    }
    await options.stop();
  }

  async function changePlaybackRate(rate: number) {
    playbackRate.value = rate;
    await options.setRate(rate);
  }

  async function changeVolume(nextVolume: number) {
    const normalized = clamp(nextVolume, 0, 1);
    volume.value = normalized;
    muted.value = normalized <= 0;
    await options.setVolume(normalized);
  }

  async function toggleMute() {
    muted.value = !muted.value;
    await options.setMuted(muted.value);
  }

  function increasePlaybackRate() {
    const nextRate = Math.min(3, Number((playbackRate.value + 0.1).toFixed(1)));
    void changePlaybackRate(nextRate);
  }

  function decreasePlaybackRate() {
    const nextRate = Math.max(0.1, Number((playbackRate.value - 0.1).toFixed(1)));
    void changePlaybackRate(nextRate);
  }

  usePlaybackShortcuts({
    playback: options.playback,
    onPlay: () => void handlePlay(),
    onPause: (positionSeconds) => void handlePause(positionSeconds),
    onSeek: (positionSeconds) => void options.seek(positionSeconds),
    onResetRate: () => void changePlaybackRate(1),
    onIncreaseRate: increasePlaybackRate,
    onDecreaseRate: decreasePlaybackRate,
  });

  watch(options.playback, (value) => {
    if (!value) {
      playbackRate.value = 1;
      return;
    }
    playbackRate.value = value.playback_rate ?? 1;
  });

  return {
    changePlaybackRate,
    changeVolume,
    handlePause,
    handlePlay,
    handleStop,
    muted,
    playbackRate,
    toggleMute,
    volume,
  };
}
