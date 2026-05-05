<script setup lang="ts">
import PlaybackCenterControls from "./PlaybackCenterControls";
import PlaybackSideActionsResponsive from "./PlaybackSideActionsResponsive.vue";
import PlaybackTimeline from "./PlaybackTimeline.vue";
import { usePlaybackControlsBindings } from "./usePlaybackControlsBindings";
import {
  usePlaybackControlsViewModel,
  type PlaybackControlsEmit,
  type PlaybackControlsProps,
} from "./usePlaybackControlsViewModel";
const props = defineProps<PlaybackControlsProps>();
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
        v-on="timelineEvents"
      />

      <div class="relative mt-1">
        <PlaybackCenterControls
          v-bind="centerControlProps"
          v-on="centerControlEvents"
        />

        <PlaybackSideActionsResponsive
          v-bind="sideActionProps"
          v-on="sideActionEvents"
        />
      </div>
    </div>
  </section>
</template>
