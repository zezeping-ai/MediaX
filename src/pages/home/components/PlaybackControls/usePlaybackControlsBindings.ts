import { computed, unref } from "vue";
import type { PlaybackState } from "@/modules/media-types";
import type { TimelineEventMap } from "./playbackControlsBindings.types";
import type {
  PlaybackControlsBindingsOptions,
  PlaybackControlsBindingsResult,
  CenterControlEventMap,
  SideActionEventMap,
} from "./bindings.contract";

export function usePlaybackControlsBindings(
  options: PlaybackControlsBindingsOptions,
): PlaybackControlsBindingsResult {
  const { props, emit, viewModel } = options;

  const timelineProps = computed(() => ({
    currentTime: viewModel.currentTime.value,
    bufferedTime: viewModel.bufferedPosition.value,
    duration: viewModel.duration.value,
    decodeBadgeClass: unref(viewModel.decodeBadgeClass),
    decodeBadgeLabel: unref(viewModel.decodeBadgeLabel),
    decodeBadgeTitle: unref(viewModel.decodeBadgeTitle),
    sliderMax: viewModel.sliderMax.value,
    timelineDisabled: viewModel.timelineDisabled.value,
    timelineTitle: viewModel.timelineTitle.value,
    sourceKey: props.playback?.current_path ?? "",
    requestPreviewFrame: props.requestPreviewFrame,
  }));

  const centerControlProps = computed(() => ({
    disabled: props.disabled,
    isPlaying: viewModel.isPlaying.value,
    playbackRate: props.playbackRate,
    selectedQuality: props.selectedQuality,
    qualityLabel: viewModel.qualityLabel.value,
    qualityOptions: props.qualityOptions,
    muted: props.muted,
    volume: viewModel.volumePreview.value,
    volumeIcon: unref(viewModel.volumeIcon),
    leftChannelVolume: props.playback?.left_channel_volume ?? 1,
    rightChannelVolume: props.playback?.right_channel_volume ?? 1,
    leftChannelMuted: props.playback?.left_channel_muted ?? false,
    rightChannelMuted: props.playback?.right_channel_muted ?? false,
    channelRouting: props.playback?.channel_routing ?? "stereo",
    speedDropdownOpen: viewModel.speedDropdownOpen.value,
    qualityDropdownOpen: viewModel.qualityDropdownOpen.value,
  }));

  const sideActionProps = computed(() => ({
    cacheRecording: props.cacheRecording,
    locked: props.locked,
    showAudioExport: props.showAudioExport,
    cacheIcon: unref(viewModel.cacheIcon),
    lockIcon: unref(viewModel.lockIcon),
  }));

  const centerControlEvents: CenterControlEventMap = {
    play: () => emit("play"),
    pause: () => emit("pause", viewModel.currentTime.value),
    stop: () => emit("stop"),
    "toggle-speed-open": viewModel.setSpeedDropdownOpen,
    "toggle-quality-open": viewModel.setQualityDropdownOpen,
    "change-speed": viewModel.handleSpeedChange,
    "change-quality": viewModel.handleQualityChange,
    "toggle-mute": () => emit("toggle-mute"),
    "overlay-interaction-change": (value: boolean) => emit("overlay-interaction-change", value),
    "change-volume": viewModel.handleVolumeChange,
    "commit-volume": viewModel.handleVolumeCommit,
    "set-left-channel-volume": (value: number) => emit("set-left-channel-volume", value),
    "set-right-channel-volume": (value: number) => emit("set-right-channel-volume", value),
    "set-left-channel-muted": (value: boolean) => emit("set-left-channel-muted", value),
    "set-right-channel-muted": (value: boolean) => emit("set-right-channel-muted", value),
    "set-channel-routing": (value: PlaybackState["channel_routing"]) => emit("set-channel-routing", value),
  };

  const sideActionEvents: SideActionEventMap = {
    "toggle-cache": () => emit("toggle-cache"),
    "toggle-lock": () => emit("toggle-lock"),
    "export-audio": () => emit("export-audio"),
  };

  const timelineEvents: TimelineEventMap = {
    preview: viewModel.handleProgressPreviewUpdate,
    commit: viewModel.handleProgressCommit,
  };

  return {
    centerControlEvents,
    centerControlProps,
    sideActionEvents,
    sideActionProps,
    timelineEvents,
    timelineProps,
  };
}
