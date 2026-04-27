import { clamp } from "lodash-es";
import type { Ref } from "vue";
import { normalizePlaybackRate } from "@/modules/player-constraints";
import type { UsePlaybackTransportControllerOptions } from "./types";

interface CreateTransportActionsOptions {
  options: UsePlaybackTransportControllerOptions;
  playbackRate: Ref<number>;
  volume: Ref<number>;
  muted: Ref<boolean>;
}

export function createTransportActions({
  options,
  playbackRate,
  volume,
  muted,
}: CreateTransportActionsOptions) {
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
    const normalized = normalizePlaybackRate(rate);
    playbackRate.value = normalized;
    await options.setRate(normalized);
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
    const nextRate = normalizePlaybackRate(playbackRate.value + 0.1);
    void changePlaybackRate(nextRate);
  }

  function decreasePlaybackRate() {
    const nextRate = normalizePlaybackRate(playbackRate.value - 0.1);
    void changePlaybackRate(nextRate);
  }

  return {
    changePlaybackRate,
    changeVolume,
    decreasePlaybackRate,
    handlePause,
    handlePlay,
    handleStop,
    increasePlaybackRate,
    toggleMute,
  };
}
