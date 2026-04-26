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
          :current-time="currentTime"
          :duration="duration"
          :disabled="disabled"
          :is-playing="isPlaying"
          :playback-rate="playbackRate"
          :selected-quality="selectedQuality"
          :quality-label="qualityLabel"
          :quality-options="qualityOptions"
          :muted="muted"
          :volume="volume"
          :volume-icon="volumeIcon"
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
          @change-volume="handleVolumeChange"
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
