<script setup lang="ts">
import type { PreviewFrame } from "@/modules/media-types";
import PlaybackCenterControls from "./PlaybackCenterControls.vue";
import PlaybackSideActions from "./PlaybackSideActions.vue";
import PlaybackTimeline from "./PlaybackTimeline.vue";
import { type PlaybackQualityOption } from "./playbackControlsUtils";
import {
  usePlaybackControlsViewModel,
  type PlaybackControlsEmit,
  type PlaybackControlsProps,
} from "./usePlaybackControlsViewModel";

type RequestPreviewFrame = (
  positionSeconds: number,
  maxWidth?: number,
  maxHeight?: number,
) => Promise<PreviewFrame | null>;

interface PlaybackControlsViewProps extends Omit<PlaybackControlsProps, "qualityOptions" | "requestPreviewFrame"> {
  qualityOptions: PlaybackQualityOption[];
  requestPreviewFrame?: RequestPreviewFrame;
}

const props = defineProps<PlaybackControlsViewProps>();
const emit = defineEmits<PlaybackControlsEmit>();

const {
  cacheIcon,
  currentTime,
  decodeBadgeClass,
  decodeBadgeLabel,
  decodeBadgeTitle,
  duration,
  handleProgressCommit,
  handleProgressPreviewUpdate,
  handleQualityChange,
  handleSpeedChange,
  handleVolumeChange,
  handleVolumeCommit,
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
  volumePreview,
  volumeIcon,
} = usePlaybackControlsViewModel(props, emit);
</script>

<template>
  <section
    class="w-full overflow-visible rounded-t-2xl rounded-b-none border border-white/10 bg-[linear-gradient(180deg,rgba(0,0,0,0.25)_0%,rgba(0,0,0,0.35)_100%)] shadow-[0_18px_60px_rgba(0,0,0,0.55)] backdrop-blur-2xl"
  >
    <div class="px-3.5 pb-2 pt-2.5">
      <PlaybackTimeline
        :current-time="currentTime"
        :duration="duration"
        :decode-badge-class="decodeBadgeClass"
        :decode-badge-label="decodeBadgeLabel"
        :decode-badge-title="decodeBadgeTitle"
        :slider-max="sliderMax"
        :timeline-disabled="timelineDisabled"
        :timeline-title="timelineTitle"
        :source-key="playback?.current_path ?? ''"
        :request-preview-frame="requestPreviewFrame"
        @preview="handleProgressPreviewUpdate"
        @commit="handleProgressCommit"
      />

      <div
        class="mt-1 grid grid-cols-[40px_minmax(0,1fr)_40px_40px] items-center gap-2 max-[720px]:grid-cols-[34px_minmax(0,1fr)_34px_34px]"
      >
        <div aria-hidden="true" />
        <PlaybackCenterControls
          :disabled="disabled"
          :is-playing="isPlaying"
          :playback-rate="playbackRate"
          :selected-quality="selectedQuality"
          :quality-label="qualityLabel"
          :quality-options="qualityOptions"
          :muted="muted"
          :volume="volumePreview"
          :volume-icon="volumeIcon"
          :left-channel-volume="playback?.left_channel_volume ?? 1"
          :right-channel-volume="playback?.right_channel_volume ?? 1"
          :left-channel-muted="playback?.left_channel_muted ?? false"
          :right-channel-muted="playback?.right_channel_muted ?? false"
          :channel-routing="playback?.channel_routing ?? 'stereo'"
          :speed-dropdown-open="speedDropdownOpen"
          :quality-dropdown-open="qualityDropdownOpen"
          @play="emit('play')"
          @pause="emit('pause', currentTime)"
          @stop="emit('stop')"
          @toggle-speed-open="setSpeedDropdownOpen"
          @toggle-quality-open="setQualityDropdownOpen"
          @change-speed="handleSpeedChange"
          @change-quality="handleQualityChange"
          @toggle-mute="emit('toggle-mute')"
          @overlay-interaction-change="emit('overlay-interaction-change', $event)"
          @change-volume="handleVolumeChange"
          @commit-volume="handleVolumeCommit"
          @set-left-channel-volume="emit('set-left-channel-volume', $event)"
          @set-right-channel-volume="emit('set-right-channel-volume', $event)"
          @set-left-channel-muted="emit('set-left-channel-muted', $event)"
          @set-right-channel-muted="emit('set-right-channel-muted', $event)"
          @set-channel-routing="emit('set-channel-routing', $event)"
        />

        <PlaybackSideActions
          :cache-recording="cacheRecording"
          :locked="locked"
          :cache-icon="cacheIcon"
          :lock-icon="lockIcon"
          @toggle-cache="emit('toggle-cache')"
          @toggle-lock="emit('toggle-lock')"
        />
      </div>
    </div>
  </section>
</template>
