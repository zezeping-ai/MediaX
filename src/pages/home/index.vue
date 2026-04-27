<script setup lang="ts">
import MediaViewport from "./components/MediaViewport/index.vue";
import StatusAlerts from "./components/StatusAlerts.vue";
import { useHomePageViewModel } from "./useHomePageViewModel";

const {
  PlaybackControls,
  OpenUrlModal,
  cacheFinalizedOutputPath,
  cacheOutputPath,
  cacheOutputSizeBytes,
  cacheRecording,
  cacheWriteSpeedBytesPerSecond,
  cancelOpenUrlInput,
  changePlaybackRate,
  changeQuality,
  changeVolume,
  clearUrlPlaylist,
  confirmOpenUrlInput,
  controlsLocked,
  controlsVisible,
  currentSource,
  debugSnapshot,
  debugTimeline,
  debugStageSnapshot,
  displayErrorMessage,
  effectiveDurationSeconds,
  firstFrameAtMs,
  latestTelemetry,
  latestAudioMeter,
  telemetryHistory,
  handlePause,
  handlePlay,
  handlePlayFromUrlPlaylist,
  handleStop,
  handleVideoEnded,
  hasSource,
  hideControlsImmediately,
  isBusy,
  markMouseActive,
  mediaInfoSnapshot,
  metadataAlbum,
  metadataArtist,
  metadataHasCoverArt,
  metadataLyrics,
  metadataMediaKind,
  metadataTitle,
  muted,
  networkReadBytesPerSecond,
  networkSustainRatio,
  onControlsMouseEnter,
  onControlsMouseLeave,
  openLocalFileByDialog,
  playback,
  playbackQualityOptions,
  playbackRate,
  pendingSource,
  recordingNoticeMessage,
  removeUrlFromPlaylist,
  requestOpenUrlInput,
  requestPreviewFrame,
  seek,
  seekPreview,
  selectedQuality,
  setControlsOverlayInteracting,
  setChannelRouting,
  setLeftChannelMuted,
  setLeftChannelVolume,
  setRightChannelMuted,
  setRightChannelVolume,
  toggleCacheRecording,
  toggleLock,
  toggleMute,
  urlDialogVisible,
  urlInputValue,
  urlPlaylist,
  volume,
} = useHomePageViewModel();
</script>

<template>
  <main class="h-screen w-screen overflow-hidden bg-transparent">
    <section
      class="relative h-full w-full"
      @mousemove="markMouseActive"
      @mouseleave="hideControlsImmediately"
    >
      <MediaViewport
        :source="currentSource"
        :pending-source="pendingSource"
        :controls-visible="controlsVisible"
        :playback="playback"
        :loading="isBusy"
        :debug-snapshot="debugSnapshot"
        :debug-timeline="debugTimeline"
        :debug-stage-snapshot="debugStageSnapshot"
        :first-frame-at-ms="firstFrameAtMs"
        :latest-telemetry="latestTelemetry"
        :latest-audio-meter="latestAudioMeter"
        :telemetry-history="telemetryHistory"
        :media-info-snapshot="mediaInfoSnapshot"
        :metadata-album="metadataAlbum"
        :metadata-artist="metadataArtist"
        :metadata-has-cover-art="metadataHasCoverArt"
        :metadata-lyrics="metadataLyrics"
        :metadata-media-kind="metadataMediaKind"
        :metadata-title="metadataTitle"
        :set-left-channel-muted="setLeftChannelMuted"
        :set-channel-routing="setChannelRouting"
        :set-left-channel-volume="setLeftChannelVolume"
        :set-right-channel-muted="setRightChannelMuted"
        :set-right-channel-volume="setRightChannelVolume"
        :network-read-bytes-per-second="networkReadBytesPerSecond"
        :network-sustain-ratio="networkSustainRatio"
        :cache-recording="cacheRecording"
        :cache-output-path="cacheOutputPath"
        :cache-output-size-bytes="cacheOutputSizeBytes"
        :cache-write-speed-bytes-per-second="cacheWriteSpeedBytesPerSecond"
        @ended="handleVideoEnded"
        @quick-open-local="openLocalFileByDialog"
        @quick-open-url="requestOpenUrlInput"
      />
      <PlaybackControls
        v-if="hasSource"
        class="absolute bottom-0 left-1/2 z-30 w-[min(760px,calc(100vw-32px))] -translate-x-1/2 opacity-100 transition-[opacity,transform] duration-300 ease-out will-change-transform"
        :class="!controlsVisible ? 'pointer-events-none translate-y-[120%] opacity-0' : ''"
        :playback="playback"
        :playback-rate="playbackRate"
        :volume="volume"
        :muted="muted"
        :locked="controlsLocked"
        :cache-recording="cacheRecording"
        :cache-output-path="cacheOutputPath"
        :duration-seconds-override="effectiveDurationSeconds"
        :quality-options="playbackQualityOptions"
        :selected-quality="selectedQuality"
        :disabled="!currentSource || isBusy"
        :request-preview-frame="requestPreviewFrame"
        @mouseenter="onControlsMouseEnter"
        @mouseleave="onControlsMouseLeave"
        @mousemove="markMouseActive"
        @play="handlePlay"
        @pause="(position) => handlePause(position)"
        @stop="handleStop"
        @seek="seek"
        @seek-preview="seekPreview"
        @change-rate="(value) => void changePlaybackRate(value)"
        @change-volume="(value) => void changeVolume(value)"
        @change-quality="(value) => void changeQuality(value)"
        @overlay-interaction-change="setControlsOverlayInteracting"
        @toggle-mute="() => void toggleMute()"
        @set-left-channel-volume="(value) => void setLeftChannelVolume(value)"
        @set-right-channel-volume="(value) => void setRightChannelVolume(value)"
        @set-left-channel-muted="(value) => void setLeftChannelMuted(value)"
        @set-right-channel-muted="(value) => void setRightChannelMuted(value)"
        @set-channel-routing="(value) => void setChannelRouting(value)"
        @toggle-cache="toggleCacheRecording"
        @toggle-lock="toggleLock"
      />
      <StatusAlerts
        :cache-finalized-output-path="cacheFinalizedOutputPath"
        :recording-notice-message="recordingNoticeMessage"
        :display-error-message="displayErrorMessage"
      />
      <OpenUrlModal
        v-if="urlDialogVisible"
        v-model:open="urlDialogVisible"
        v-model:input-value="urlInputValue"
        :busy="isBusy"
        :playlist="urlPlaylist"
        @confirm="confirmOpenUrlInput"
        @cancel="cancelOpenUrlInput"
        @clear="clearUrlPlaylist"
        @remove="removeUrlFromPlaylist"
        @select="(url) => (urlInputValue = url)"
        @play="handlePlayFromUrlPlaylist"
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
