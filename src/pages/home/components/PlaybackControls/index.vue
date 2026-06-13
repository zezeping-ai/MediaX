<script setup lang="ts">
import PlaybackCenterControls from "./PlaybackCenterControls";
import PlaybackSideActionsResponsive from "./PlaybackSideActionsResponsive.vue";
import PlaybackTimeline from "./PlaybackTimeline.vue";
import { usePlayerChromeTheme } from "@/pages/home/composables/usePlayerChromeTheme";
import { usePlaybackControlsBindings } from "./usePlaybackControlsBindings";
import {
  usePlaybackControlsViewModel,
  type PlaybackControlsEmit,
  type PlaybackControlsProps,
} from "./usePlaybackControlsViewModel";
const props = defineProps<PlaybackControlsProps>();
const emit = defineEmits<PlaybackControlsEmit>();

const viewModel = usePlaybackControlsViewModel(props, emit);
const { controlsShell } = usePlayerChromeTheme();

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
  <section :class="controlsShell">
    <div class="px-3.5 pb-1.5 pt-2">
      <PlaybackTimeline
        v-bind="timelineProps"
        v-on="timelineEvents"
      />

      <div class="mt-2.5 hidden min-[860px]:grid min-[860px]:grid-cols-[1fr_auto_1fr] min-[860px]:items-center">
        <PlaybackSideActionsResponsive
          variant="dock-left"
          v-bind="sideActionProps"
          v-on="sideActionEvents"
        />
        <PlaybackCenterControls
          v-bind="centerControlProps"
          v-on="centerControlEvents"
        />
        <PlaybackSideActionsResponsive
          variant="dock-right"
          v-bind="sideActionProps"
          v-on="sideActionEvents"
        />
      </div>

      <div class="mt-2.5 min-[860px]:hidden">
        <PlaybackCenterControls
          v-bind="centerControlProps"
          v-on="centerControlEvents"
        />
        <PlaybackSideActionsResponsive
          variant="stack"
          v-bind="sideActionProps"
          v-on="sideActionEvents"
        />
      </div>
    </div>
  </section>
</template>
