import { useEventListener } from "@vueuse/core";
import type { Ref } from "vue";
import type { PlaybackState } from "@/modules/media-types";

type UsePlaybackShortcutsOptions = {
  playback: Ref<PlaybackState | null>;
  onPlay: () => void;
  onPause: (positionSeconds?: number) => void;
  onSeek: (positionSeconds: number) => void;
  onResetRate: () => void;
  onIncreaseRate: () => void;
  onDecreaseRate: () => void;
};

export function usePlaybackShortcuts(options: UsePlaybackShortcutsOptions) {
  const HOLD_TRIGGER_MS = 220;
  const HOLD_TICK_MS = 120;
  const SHORT_SEEK_DELTA_SECONDS = 10;
  const HOLD_BASE_DELTA_SECONDS = 2;
  const HOLD_ACCELERATION_PER_SECOND = 4;
  const HOLD_MAX_DELTA_SECONDS = 16;

  let activeSeekDirection: -1 | 1 | null = null;
  let seekKeyDownAtMs = 0;
  let holdActivated = false;
  let holdTimerId: number | null = null;
  let holdTickTimerId: number | null = null;

  function getPlaybackTimeBounds() {
    const playback = options.playback.value;
    if (!playback || !playback.current_path) {
      return null;
    }
    // For non-seekable streams, duration_seconds is 0. Avoid emitting seek commands.
    if (!Number.isFinite(playback.duration_seconds) || playback.duration_seconds <= 0) {
      return null;
    }
    const max = Number.isFinite(playback.duration_seconds) && playback.duration_seconds > 0
      ? playback.duration_seconds
      : 0;
    return {
      current: Number.isFinite(playback.position_seconds) ? playback.position_seconds : 0,
      max,
    };
  }

  function clampTargetTime(next: number, max: number) {
    if (max <= 0) {
      return Math.max(0, next);
    }
    return Math.min(Math.max(0, next), max);
  }

  function seekByDelta(direction: -1 | 1, deltaSeconds: number) {
    const bounds = getPlaybackTimeBounds();
    if (!bounds) {
      return;
    }
    const target = clampTargetTime(bounds.current + direction * deltaSeconds, bounds.max);
    if (Math.abs(target - bounds.current) < 1e-3) {
      return;
    }
    options.onSeek(target);
  }

  function clearSeekHoldTimers() {
    if (holdTimerId !== null) {
      window.clearTimeout(holdTimerId);
      holdTimerId = null;
    }
    if (holdTickTimerId !== null) {
      window.clearInterval(holdTickTimerId);
      holdTickTimerId = null;
    }
  }

  function endSeekHold(direction: -1 | 1) {
    if (activeSeekDirection !== direction) {
      return;
    }
    const heldForMs = Date.now() - seekKeyDownAtMs;
    const isShortPress = !holdActivated || heldForMs < HOLD_TRIGGER_MS;
    clearSeekHoldTimers();
    activeSeekDirection = null;
    holdActivated = false;
    if (isShortPress) {
      seekByDelta(direction, SHORT_SEEK_DELTA_SECONDS);
    }
  }

  function beginSeekHold(direction: -1 | 1) {
    if (activeSeekDirection !== null) {
      return;
    }
    activeSeekDirection = direction;
    seekKeyDownAtMs = Date.now();
    holdActivated = false;
    clearSeekHoldTimers();
    holdTimerId = window.setTimeout(() => {
      if (activeSeekDirection !== direction) {
        return;
      }
      holdActivated = true;
      holdTickTimerId = window.setInterval(() => {
        if (activeSeekDirection !== direction) {
          return;
        }
        const holdSeconds = (Date.now() - seekKeyDownAtMs) / 1000;
        const deltaPerTick = Math.min(
          HOLD_BASE_DELTA_SECONDS + holdSeconds * HOLD_ACCELERATION_PER_SECOND,
          HOLD_MAX_DELTA_SECONDS,
        );
        seekByDelta(direction, deltaPerTick);
      }, HOLD_TICK_MS);
    }, HOLD_TRIGGER_MS);
  }

  useEventListener(window, "keydown", (event: KeyboardEvent) => {
    const target = event.target as HTMLElement | null;
    if (
      target instanceof HTMLInputElement ||
      target instanceof HTMLTextAreaElement ||
      target?.isContentEditable
    ) {
      return;
    }
    if (event.code === "Space") {
      event.preventDefault();
      if (options.playback.value?.status === "playing") {
        options.onPause(options.playback.value.position_seconds);
      } else {
        options.onPlay();
      }
      return;
    }
    if (event.key === "]") {
      event.preventDefault();
      options.onIncreaseRate();
      return;
    }
    if (event.key === "[") {
      event.preventDefault();
      options.onDecreaseRate();
      return;
    }
    if (event.key === "0") {
      event.preventDefault();
      options.onResetRate();
      return;
    }
    if (event.code === "ArrowLeft") {
      event.preventDefault();
      if (!event.repeat) {
        beginSeekHold(-1);
      }
      return;
    }
    if (event.code === "ArrowRight") {
      event.preventDefault();
      if (!event.repeat) {
        beginSeekHold(1);
      }
    }
  });

  useEventListener(window, "keyup", (event: KeyboardEvent) => {
    if (event.code === "ArrowLeft") {
      event.preventDefault();
      endSeekHold(-1);
      return;
    }
    if (event.code === "ArrowRight") {
      event.preventDefault();
      endSeekHold(1);
    }
  });

  useEventListener(window, "blur", () => {
    clearSeekHoldTimers();
    activeSeekDirection = null;
    holdActivated = false;
  });
}
