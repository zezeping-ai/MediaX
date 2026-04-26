import { computed, onBeforeUnmount, ref, watch } from "vue";
import type { PlaybackState, PreviewFrame } from "@/modules/media-types";
import { usePlaybackTimelineState } from "../../composables/usePlaybackTimelineState";
import type { PlaybackQualityOption } from "./playbackControlsUtils";

type SliderValue = number | [number, number];
type RequestPreviewFrame = (
  positionSeconds: number,
  maxWidth?: number,
  maxHeight?: number,
) => Promise<PreviewFrame | null>;

export interface PlaybackControlsProps {
  playback: PlaybackState | null;
  disabled: boolean;
  playbackRate: number;
  volume: number;
  muted: boolean;
  locked: boolean;
  cacheRecording: boolean;
  cacheOutputPath: string;
  durationSecondsOverride: number;
  qualityOptions: PlaybackQualityOption[];
  selectedQuality: string;
  requestPreviewFrame?: RequestPreviewFrame;
}

export interface PlaybackControlsEmit {
  (event: "play"): void;
  (event: "pause", position: number): void;
  (event: "stop"): void;
  (event: "seek", position: number): void;
  (event: "seek-preview", position: number): void;
  (event: "change-rate", value: number): void;
  (event: "change-volume", value: number): void;
  (event: "change-quality", value: string): void;
  (event: "overlay-interaction-change", value: boolean): void;
  (event: "toggle-mute"): void;
  (event: "toggle-cache"): void;
  (event: "toggle-lock"): void;
}

export function usePlaybackControlsViewModel(
  props: PlaybackControlsProps,
  emit: PlaybackControlsEmit,
) {
  const { currentTime, commitSeek, previewSeekWhilePaused, cancelPreviewSeek } =
    usePlaybackTimelineState({
      playback: () => props.playback,
      onSeek: (seconds) => emit("seek", seconds),
      onSeekPreview: (seconds) => emit("seek-preview", seconds),
    });

  const duration = computed(() => {
    const base = props.playback?.duration_seconds ?? 0;
    const override = props.durationSecondsOverride ?? 0;
    const normalizedBase = Number.isFinite(base) ? Math.max(0, base) : 0;
    const normalizedOverride = Number.isFinite(override) ? Math.max(0, override) : 0;
    return Math.max(normalizedBase, normalizedOverride);
  });

  const canSeek = computed(() => {
    const playback = props.playback;
    if (!playback || !playback.current_path) {
      return false;
    }
    return Number.isFinite(playback.duration_seconds) && playback.duration_seconds > 0;
  });

  const timelineDisabled = computed(() => props.disabled || !canSeek.value);
  const timelineTitle = computed(() =>
    timelineDisabled.value ? "当前流不支持跳转进度" : "拖动调整播放进度",
  );
  const sliderMax = computed(() => Math.max(duration.value, currentTime.value, 1));
  const isPlaying = computed(() => props.playback?.status === "playing");
  const qualityLabel = computed(() => {
    const matched = props.qualityOptions.find((option) => option.key === props.selectedQuality);
    return matched?.label ?? "原画";
  });
  const volumeIcon = computed(() => {
    if (props.muted || props.volume <= 0) {
      return "lucide:volume-x";
    }
    if (props.volume < 0.5) {
      return "lucide:volume-1";
    }
    return "lucide:volume-2";
  });
  const lockIcon = computed(() => (props.locked ? "lucide:lock" : "lucide:lock-open"));
  const cacheIcon = computed(() =>
    props.cacheRecording ? "lucide:database-zap" : "lucide:database",
  );
  const speedDropdownOpen = ref(false);
  const qualityDropdownOpen = ref(false);

  watch(
    () => speedDropdownOpen.value || qualityDropdownOpen.value,
    (open) => {
      emit("overlay-interaction-change", open);
    },
  );

  function setSpeedDropdownOpen(value: boolean) {
    speedDropdownOpen.value = value;
  }

  function setQualityDropdownOpen(value: boolean) {
    qualityDropdownOpen.value = value;
  }

  function handleSpeedChange(key: string | number) {
    speedDropdownOpen.value = false;
    emit("change-rate", Number(key));
  }

  function handleQualityChange(key: string | number) {
    qualityDropdownOpen.value = false;
    emit("change-quality", String(key));
  }

  function handleProgressPreviewUpdate(value: SliderValue) {
    if (!canSeek.value) {
      return;
    }
    previewSeekWhilePaused(normalizeSliderValue(value));
  }

  function handleProgressCommit(value: SliderValue) {
    if (!canSeek.value) {
      return;
    }
    commitSeek(normalizeSliderValue(value));
  }

  function handleVolumeChange(value: SliderValue) {
    emit("change-volume", normalizeSliderValue(value));
  }

  onBeforeUnmount(() => {
    cancelPreviewSeek();
    emit("overlay-interaction-change", false);
  });

  return {
    cacheIcon,
    currentTime,
    duration,
    handleProgressCommit,
    handleProgressPreviewUpdate,
    handleQualityChange,
    handleSpeedChange,
    handleVolumeChange,
    isPlaying,
    lockIcon,
    qualityDropdownOpen,
    qualityLabel,
    setQualityDropdownOpen,
    setSpeedDropdownOpen,
    sliderMax,
    speedDropdownOpen,
    timelineDisabled,
    timelineTitle,
    volumeIcon,
  };
}

function normalizeSliderValue(value: SliderValue) {
  return Array.isArray(value) ? Number(value[0]) : Number(value);
}
