<script setup lang="ts">
import type { PreviewFrame } from "@/modules/media-types";
import PlaybackCenterControls from "./PlaybackCenterControls";
import PlaybackSideActions from "./PlaybackSideActions.vue";
import PlaybackTimeline from "./PlaybackTimeline.vue";
import { type PlaybackQualityOption } from "./playbackControlsUtils";
import { usePlaybackControlsBindings } from "./usePlaybackControlsBindings";
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

const viewModel = usePlaybackControlsViewModel(props, emit);

const {
  centerControlEvents,
  centerControlProps,
  sideActionEvents,
  sideActionProps,
  timelineEvents,
  timelineProps,
} = usePlaybackControlsBindings({
  props,
  emit,
  viewModel,
});
</script>

<template>
  <section
    class="w-full overflow-visible rounded-t-2xl rounded-b-none border border-white/10 bg-[linear-gradient(180deg,rgba(0,0,0,0.25)_0%,rgba(0,0,0,0.35)_100%)] shadow-[0_18px_60px_rgba(0,0,0,0.55)] backdrop-blur-2xl"
  >
    <div class="px-3.5 pb-2 pt-2.5">
      <PlaybackTimeline
        v-bind="timelineProps"
        @preview="timelineEvents.onPreview"
        @commit="timelineEvents.onCommit"
      />

      <div class="relative mt-1">
        <PlaybackCenterControls
          v-bind="centerControlProps"
          @play="centerControlEvents.onPlay"
          @pause="centerControlEvents.onPause"
          @stop="centerControlEvents.onStop"
          @toggle-speed-open="centerControlEvents.onToggleSpeedOpen"
          @toggle-quality-open="centerControlEvents.onToggleQualityOpen"
          @change-speed="centerControlEvents.onChangeSpeed"
          @change-quality="centerControlEvents.onChangeQuality"
          @toggle-mute="centerControlEvents.onToggleMute"
          @overlay-interaction-change="centerControlEvents.onOverlayInteractionChange"
          @change-volume="centerControlEvents.onChangeVolume"
          @commit-volume="centerControlEvents.onCommitVolume"
          @set-left-channel-volume="centerControlEvents.onSetLeftChannelVolume"
          @set-right-channel-volume="centerControlEvents.onSetRightChannelVolume"
          @set-left-channel-muted="centerControlEvents.onSetLeftChannelMuted"
          @set-right-channel-muted="centerControlEvents.onSetRightChannelMuted"
          @set-channel-routing="centerControlEvents.onSetChannelRouting"
        />

        <div class="absolute right-0 top-1/2 hidden -translate-y-1/2 min-[860px]:block">
          <PlaybackSideActions
            v-bind="sideActionProps"
            @toggle-cache="sideActionEvents.onToggleCache"
            @toggle-lock="sideActionEvents.onToggleLock"
            @export-audio="sideActionEvents.onExportAudio"
          />
        </div>
      </div>
      <div class="mt-2 flex justify-center min-[860px]:hidden">
        <PlaybackSideActions
          v-bind="sideActionProps"
          @toggle-cache="sideActionEvents.onToggleCache"
          @toggle-lock="sideActionEvents.onToggleLock"
          @export-audio="sideActionEvents.onExportAudio"
        />
      </div>
    </div>
  </section>
</template>
