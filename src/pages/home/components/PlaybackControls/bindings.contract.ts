import type { ComputedRef } from "vue";
import type { PlaybackState } from "@/modules/media-types";
import type { PlaybackControlsEmit, PlaybackControlsProps } from "./controls.contract";
import type { PlaybackControlsViewModel } from "./usePlaybackControlsViewModel";
import type { TimelineEventMap } from "./playbackControlsBindings.types";

export type PlaybackControlsBindingsOptions = {
  props: PlaybackControlsProps;
  emit: PlaybackControlsEmit;
  viewModel: PlaybackControlsViewModel;
};

export type TimelineViewProps = {
  currentTime: number;
  bufferedTime: number;
  duration: number;
  decodeBadgeClass: string;
  decodeBadgeLabel: string;
  decodeBadgeTitle: string;
  sliderMax: number;
  timelineDisabled: boolean;
  timelineTitle: string;
  sourceKey: string;
  requestPreviewFrame?: PlaybackControlsProps["requestPreviewFrame"];
};

export type CenterControlViewProps = {
  disabled: boolean;
  isPlaying: boolean;
  playbackRate: number;
  selectedQuality: string;
  qualityLabel: string;
  qualityOptions: PlaybackControlsProps["qualityOptions"];
  muted: boolean;
  volume: number;
  volumeIcon: string;
  leftChannelVolume: number;
  rightChannelVolume: number;
  leftChannelMuted: boolean;
  rightChannelMuted: boolean;
  channelRouting: PlaybackState["channel_routing"];
  speedDropdownOpen: boolean;
  qualityDropdownOpen: boolean;
};

export type SideActionViewProps = {
  cacheRecording: boolean;
  locked: boolean;
  showAudioExport: boolean;
  cacheIcon: string;
  lockIcon: string;
};

export type CenterControlEventMap = {
  play: () => void;
  pause: () => void;
  stop: () => void;
  "toggle-speed-open": (open: boolean) => void;
  "toggle-quality-open": (open: boolean) => void;
  "change-speed": (value: string | number) => void;
  "change-quality": (value: string | number) => void;
  "toggle-mute": () => void;
  "overlay-interaction-change": (value: boolean) => void;
  "change-volume": (value: number | [number, number]) => void;
  "commit-volume": (value: number | [number, number]) => void;
  "set-left-channel-volume": (value: number) => void;
  "set-right-channel-volume": (value: number) => void;
  "set-left-channel-muted": (value: boolean) => void;
  "set-right-channel-muted": (value: boolean) => void;
  "set-channel-routing": (value: PlaybackState["channel_routing"]) => void;
};

export type SideActionEventMap = {
  "toggle-cache": () => void;
  "toggle-lock": () => void;
  "export-audio": () => void;
};

// Vue `defineEmits` tuple-style contracts (for component files)
export type CenterControlEmitContract = {
  play: [];
  pause: [];
  stop: [];
  "toggle-speed-open": [boolean];
  "toggle-quality-open": [boolean];
  "change-speed": [string | number];
  "change-quality": [string | number];
  "toggle-mute": [];
  "overlay-interaction-change": [boolean];
  "change-volume": [number | [number, number]];
  "commit-volume": [number | [number, number]];
  "set-left-channel-volume": [number];
  "set-right-channel-volume": [number];
  "set-left-channel-muted": [boolean];
  "set-right-channel-muted": [boolean];
  "set-channel-routing": [PlaybackState["channel_routing"]];
};

export type SideActionEmitContract = {
  "toggle-cache": [];
  "toggle-lock": [];
  "export-audio": [];
};

export type TimelineEmitContract = {
  preview: [number | [number, number]];
  commit: [number | [number, number]];
};

export type PlaybackControlsBindingsResult = {
  timelineProps: ComputedRef<TimelineViewProps>;
  centerControlProps: ComputedRef<CenterControlViewProps>;
  sideActionProps: ComputedRef<SideActionViewProps>;
  timelineEvents: TimelineEventMap;
  centerControlEvents: CenterControlEventMap;
  sideActionEvents: SideActionEventMap;
};
