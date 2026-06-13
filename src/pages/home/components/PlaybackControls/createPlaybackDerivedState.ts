import { computed } from "vue";
import {
  formatDecodeBadgeLabel,
  formatDecodeBadgeTitle,
  type PlaybackQualityOption,
} from "./playbackControlsUtils";
import { usePreferences } from "@/modules/preferences";
import type { PlaybackControlsProps } from "./usePlaybackControlsViewModel";

export function createPlaybackDerivedState(
  props: PlaybackControlsProps,
  duration: { value: number },
  currentTime: { value: number },
  volumePreview: { value: number },
) {
  const { isDark } = usePreferences();
  const canSeek = computed(() => {
    const playback = props.playback;
    if (!playback || !playback.current_path) {
      return false;
    }
    return duration.value > 0;
  });

  const timelineDisabled = computed(() => props.disabled || !canSeek.value);
  const timelineTitle = computed(() =>
    timelineDisabled.value ? "当前流不支持跳转进度" : "拖动调整播放进度",
  );
  const sliderMax = computed(() => Math.max(duration.value, currentTime.value, 1));
  const isPlaying = computed(() => props.playback?.status === "playing");
  const qualityLabel = computed(() => {
    const matched = props.qualityOptions.find((option: PlaybackQualityOption) => option.key === props.selectedQuality);
    return matched?.label ?? "原画";
  });
  const showDecodeBadge = computed(() => props.playback?.media_kind !== "audio");
  const decodeBadgeLabel = computed(() =>
    formatDecodeBadgeLabel(Boolean(props.playback?.hw_decode_active)),
  );
  const decodeBadgeTitle = computed(() =>
    formatDecodeBadgeTitle(
      Boolean(props.playback?.hw_decode_active),
      props.playback?.hw_decode_backend,
      props.playback?.hw_decode_error,
    ),
  );
  const decodeBadgeClass = computed(() => {
    if (props.playback?.hw_decode_active) {
      return isDark.value
        ? "border-emerald-400/28 bg-emerald-500/12 text-emerald-100"
        : "border-emerald-500/25 bg-emerald-50 text-emerald-800";
    }
    return isDark.value
      ? "border-amber-300/28 bg-amber-500/12 text-amber-100"
      : "border-amber-500/25 bg-amber-50 text-amber-800";
  });
  const volumeIcon = computed(() => {
    if (props.muted || volumePreview.value <= 0) {
      return "lucide:volume-x";
    }
    if (volumePreview.value < 0.5) {
      return "lucide:volume-1";
    }
    return "lucide:volume-2";
  });
  const lockIcon = computed(() => (props.locked ? "lucide:lock" : "lucide:lock-open"));
  const cacheIcon = computed(() =>
    props.cacheRecording ? "lucide:database-zap" : "lucide:database",
  );

  return {
    cacheIcon,
    canSeek,
    showDecodeBadge,
    decodeBadgeClass,
    decodeBadgeLabel,
    decodeBadgeTitle,
    isPlaying,
    lockIcon,
    qualityLabel,
    sliderMax,
    timelineDisabled,
    timelineTitle,
    volumeIcon,
  };
}
