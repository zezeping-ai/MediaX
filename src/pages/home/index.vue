<script setup lang="ts">
import OpenUrlModal from "./components/OpenUrlModal";
import PlaybackControls from "./components/PlaybackControls";
import MediaViewport from "./components/MediaViewport";
import StatusAlerts from "./components/StatusAlerts.vue";
import { useHomePageBindings } from "./useHomePageBindings";
import { useHomePageViewModel } from "./useHomePageViewModel";

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
} = useHomePageBindings(useHomePageViewModel());
</script>

<template>
  <main class="h-screen w-screen overflow-hidden bg-transparent">
    <section
      class="relative h-full w-full"
      @mousemove="shellEvents.onPointerMove"
      @pointermove="shellEvents.onPointerMove"
      @pointerdown="shellEvents.onPointerActivate"
      @touchstart.passive="shellEvents.onPointerActivate"
      @focusin="shellEvents.onFocusIn"
      @mouseleave="shellEvents.onPointerLeave"
    >
      <MediaViewport
        v-bind="mediaViewportProps"
        @ended="mediaViewportEvents.onEnded"
        @quick-open-local="mediaViewportEvents.onQuickOpenLocal"
        @quick-open-url="mediaViewportEvents.onQuickOpenUrl"
      />
      <PlaybackControls
        v-if="hasSource"
        class="absolute bottom-0 left-1/2 z-30 w-[min(760px,calc(100vw-32px))] -translate-x-1/2 opacity-100 transition-[opacity,transform] duration-300 ease-out will-change-transform"
        :class="!controlsVisible ? 'pointer-events-none translate-y-[120%] opacity-0' : ''"
        v-bind="playbackControlsProps"
        @mouseenter="playbackControlsEvents.onMouseEnter"
        @mouseleave="playbackControlsEvents.onMouseLeave"
        @mousemove="playbackControlsEvents.onMouseMove"
        @play="playbackControlsEvents.onPlay"
        @pause="playbackControlsEvents.onPause"
        @stop="playbackControlsEvents.onStop"
        @seek="playbackControlsEvents.onSeek"
        @seek-preview="playbackControlsEvents.onSeekPreview"
        @change-rate="playbackControlsEvents.onChangeRate"
        @change-volume="playbackControlsEvents.onChangeVolume"
        @change-quality="playbackControlsEvents.onChangeQuality"
        @overlay-interaction-change="playbackControlsEvents.onOverlayInteractionChange"
        @toggle-mute="playbackControlsEvents.onToggleMute"
        @set-left-channel-volume="playbackControlsEvents.onSetLeftChannelVolume"
        @set-right-channel-volume="playbackControlsEvents.onSetRightChannelVolume"
        @set-left-channel-muted="playbackControlsEvents.onSetLeftChannelMuted"
        @set-right-channel-muted="playbackControlsEvents.onSetRightChannelMuted"
        @set-channel-routing="playbackControlsEvents.onSetChannelRouting"
        @toggle-cache="playbackControlsEvents.onToggleCache"
        @toggle-lock="playbackControlsEvents.onToggleLock"
      />
      <StatusAlerts v-bind="statusAlertProps" />
      <OpenUrlModal
        v-if="urlDialogVisible"
        v-model:open="urlDialogVisible"
        v-model:input-value="urlDialogInputValue"
        v-bind="urlDialogProps"
        @confirm="urlDialogEvents.onConfirm"
        @cancel="urlDialogEvents.onCancel"
        @clear="urlDialogEvents.onClear"
        @remove="urlDialogEvents.onRemove"
        @select="urlDialogEvents.onSelect"
        @play="urlDialogEvents.onPlay"
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
