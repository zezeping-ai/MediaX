import { computed, unref } from "vue";
import type {
  PlaybackControlsEmit,
  PlaybackControlsProps,
  PlaybackControlsViewModel,
} from "./usePlaybackControlsViewModel";
import type { PlaybackState } from "@/modules/media-types";

type PlaybackControlsBindingsOptions = {
  props: PlaybackControlsProps;
  emit: PlaybackControlsEmit;
  viewModel: PlaybackControlsViewModel;
};

export function usePlaybackControlsBindings(options: PlaybackControlsBindingsOptions) {
  const { props, emit, viewModel } = options;

  const timelineProps = computed(() => ({
    currentTime: viewModel.currentTime.value,
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
    cacheIcon: unref(viewModel.cacheIcon),
    lockIcon: unref(viewModel.lockIcon),
  }));

  const centerControlEvents = {
    onPlay: () => emit("play"),
    onPause: () => emit("pause", viewModel.currentTime.value),
    onStop: () => emit("stop"),
    onToggleSpeedOpen: viewModel.setSpeedDropdownOpen,
    onToggleQualityOpen: viewModel.setQualityDropdownOpen,
    onChangeSpeed: viewModel.handleSpeedChange,
    onChangeQuality: viewModel.handleQualityChange,
    onToggleMute: () => emit("toggle-mute"),
    onOverlayInteractionChange: (value: boolean) => emit("overlay-interaction-change", value),
    onChangeVolume: viewModel.handleVolumeChange,
    onCommitVolume: viewModel.handleVolumeCommit,
    onSetLeftChannelVolume: (value: number) => emit("set-left-channel-volume", value),
    onSetRightChannelVolume: (value: number) => emit("set-right-channel-volume", value),
    onSetLeftChannelMuted: (value: boolean) => emit("set-left-channel-muted", value),
    onSetRightChannelMuted: (value: boolean) => emit("set-right-channel-muted", value),
    onSetChannelRouting: (value: PlaybackState["channel_routing"]) => emit("set-channel-routing", value),
  };

  const sideActionEvents = {
    onToggleCache: () => emit("toggle-cache"),
    onToggleLock: () => emit("toggle-lock"),
  };

  return {
    centerControlEvents,
    centerControlProps,
    sideActionEvents,
    sideActionProps,
    timelineEvents: {
      onPreview: viewModel.handleProgressPreviewUpdate,
      onCommit: viewModel.handleProgressCommit,
    },
    timelineProps,
  };
}
