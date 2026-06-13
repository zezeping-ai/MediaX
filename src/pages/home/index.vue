<script setup lang="ts">
import { watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { Icon } from "@iconify/vue";
import { useWindowFileDrop } from "./composables/useWindowFileDrop";
import OpenUrlModal from "./components/OpenUrlModal";
import PlaylistPanel from "./components/PlaylistPanel/index.vue";
import PlaybackControls from "./components/PlaybackControls";
import MediaViewport from "./components/MediaViewport";
import StatusAlerts from "./components/StatusAlerts.vue";
import { useHomePageBindings } from "./useHomePageBindings";
import { useHomePageViewModel } from "./useHomePageViewModel";
import { usePlayerChromeTheme } from "./composables/usePlayerChromeTheme";

const viewModel = useHomePageViewModel();
const route = useRoute();
const router = useRouter();
const { dropActive } = useWindowFileDrop({
  openPath: viewModel.openPath,
  importPaths: viewModel.importPathsToQueue,
});

const {
  controlsVisible,
  hasSource,
  mediaViewportEvents,
  mediaViewportProps,
  playbackControlsEvents,
  playbackControlsProps,
  playlistPanelEvents,
  playlistPanelProps,
  playlistPanelVisible,
  shellEvents,
  statusAlertProps,
  urlDialogInputValue,
  urlDialogEvents,
  urlDialogProps,
  urlDialogVisible,
} = useHomePageBindings(viewModel);
const { floatingIconButton, isDark } = usePlayerChromeTheme();

watch(
  () => route.query.menuAction,
  async (menuAction) => {
    const action = typeof menuAction === "string" ? menuAction : "";
    if (!action) {
      return;
    }
    if (action === "open_local") {
      await viewModel.openLocalFileByDialog();
    } else if (action === "open_url") {
      viewModel.requestOpenUrlInput();
    }
    const { menuAction: _discard, ...nextQuery } = route.query;
    await router.replace({ query: nextQuery });
  },
  { immediate: true },
);
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
        class="pointer-events-none absolute inset-5 z-40 flex items-center justify-center rounded-[28px] border border-dashed backdrop-blur-md"
        :class="isDark ? 'border-white/35 bg-black/28' : 'border-black/18 bg-white/55'"
      >
        <div
          class="rounded-2xl border px-5 py-3 text-center text-sm font-medium tracking-[0.02em] shadow-[0_18px_48px_rgba(0,0,0,0.38)]"
          :class="isDark ? 'border-white/12 bg-black/45 text-white/92' : 'border-black/10 bg-white/88 text-slate-800'"
        >
          拖拽媒体文件到这里即可播放或加入列表
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
      <button
        v-if="!hasSource"
        type="button"
        class="absolute right-4 top-4 z-40"
        :class="floatingIconButton"
        title="播放列表"
        @click="viewModel.togglePlaylistPanel()"
      >
        <span class="relative inline-flex">
          <Icon icon="lucide:list-music" width="18" height="18" />
          <span
            v-if="viewModel.playlistController.queueCount.value > 0"
            class="absolute -right-2 -top-2 min-w-[14px] rounded-full bg-[#1677ff] px-1 text-[9px] leading-[14px] text-white"
          >
            {{ viewModel.playlistController.queueCount.value > 99 ? "99+" : viewModel.playlistController.queueCount.value }}
          </span>
        </span>
      </button>
      <PlaylistPanel
        v-model:open="playlistPanelVisible"
        v-bind="playlistPanelProps"
        v-on="playlistPanelEvents"
      />
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

html[data-theme="light"] :deep(.ant-empty-description) {
  color: rgba(15, 23, 42, 0.68);
}

html[data-theme="light"] :deep(.ant-alert-message) {
  color: rgba(15, 23, 42, 0.92);
}

html[data-theme="light"] :deep(.ant-alert-error) {
  background: rgba(255, 77, 79, 0.12);
  border: 1px solid rgba(255, 77, 79, 0.28);
}

:deep(.ant-alert-error) {
  background: rgba(255, 77, 79, 0.2);
  border: 1px solid rgba(255, 77, 79, 0.35);
}

:deep(.video) {
  object-fit: contain;
}
</style>
