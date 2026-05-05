<script setup lang="ts">
import { useWindowFileDrop } from "./composables/useWindowFileDrop";
import OpenUrlModal from "./components/OpenUrlModal";
import PlaybackControls from "./components/PlaybackControls";
import MediaViewport from "./components/MediaViewport";
import StatusAlerts from "./components/StatusAlerts.vue";
import { useHomePageBindings } from "./useHomePageBindings";
import { useHomePageViewModel } from "./useHomePageViewModel";

const viewModel = useHomePageViewModel();
const { dropActive } = useWindowFileDrop({
  openPath: viewModel.openPath,
});

const {
  controlsVisible,
  hasSource,
  mediaViewportEvents,
  mediaViewportProps,
  playbackControlsEvents,
  playbackControlsProps,
  shellEvents,
  statusAlertProps,
  urlDialogInputValue,
  urlDialogEvents,
  urlDialogProps,
  urlDialogVisible,
} = useHomePageBindings(viewModel);
</script>

<template>
  <main class="h-screen w-screen overflow-hidden bg-transparent">
    <section
      class="relative h-full w-full"
      @pointermove="shellEvents.onPointerMove"
      @pointerdown="shellEvents.onPointerActivate"
      @touchstart.passive="shellEvents.onPointerActivate"
      @focusin="shellEvents.onFocusIn"
      @mouseleave="shellEvents.onPointerLeave"
    >
      <MediaViewport
        v-bind="mediaViewportProps"
        v-on="mediaViewportEvents"
      />
      <div
        v-if="dropActive"
        class="pointer-events-none absolute inset-5 z-40 flex items-center justify-center rounded-[28px] border border-dashed border-white/35 bg-black/28 backdrop-blur-md"
      >
        <div class="rounded-2xl border border-white/12 bg-black/45 px-5 py-3 text-center text-sm font-medium tracking-[0.02em] text-white/92 shadow-[0_18px_48px_rgba(0,0,0,0.38)]">
          拖拽媒体文件到这里即可播放
        </div>
      </div>
      <PlaybackControls
        v-if="hasSource"
        class="absolute bottom-0 left-1/2 z-30 w-[min(760px,calc(100vw-32px))] -translate-x-1/2 opacity-100 transition-[opacity,transform] duration-300 ease-out will-change-transform"
        :class="!controlsVisible ? 'pointer-events-none translate-y-[120%] opacity-0' : ''"
        v-bind="playbackControlsProps"
        v-on="playbackControlsEvents"
      />
      <StatusAlerts v-bind="statusAlertProps" />
      <OpenUrlModal
        v-if="urlDialogVisible"
        v-model:open="urlDialogVisible"
        v-model:input-value="urlDialogInputValue"
        v-bind="urlDialogProps"
        v-on="urlDialogEvents"
      />
    </section>
  </main>
</template>

<style scoped>
/* .media-player-page removed (Tailwind) */

/* .player-shell removed (Tailwind) */

/* :deep(.player-canvas) sizing handled by component */

/* .overlay-controls removed (Tailwind) */

/* .overlay-controls-hidden removed (Tailwind) */

/* .overlay-error removed (Tailwind) */

:deep(.ant-alert) {
  border-radius: 8px;
}

:deep(.ant-empty-description) {
  color: rgba(255, 255, 255, 0.75);
}

:deep(.ant-alert-message) {
  color: #fff;
}

:deep(.ant-alert-error) {
  background: rgba(255, 77, 79, 0.2);
  border: 1px solid rgba(255, 77, 79, 0.35);
}

:deep(.video) {
  object-fit: contain;
}
</style>
