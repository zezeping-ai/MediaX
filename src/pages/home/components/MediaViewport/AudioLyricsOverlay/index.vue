<script setup lang="ts">
import { computed, toRef } from "vue";
import { Icon } from "@iconify/vue";
import type {
  LyricsCandidateSummary,
  MediaAudioMeterPayload,
  MediaLyricLine,
  MediaSnapshot,
  PlaybackState,
} from "@/modules/media-types";
import AudioMetadataEditorModal from "./AudioMetadataEditorModal.vue";
import LyricsSelect from "@/components/selects/LyricsSelect/index.vue";
import LyricsEmptyState from "./LyricsEmptyState.vue";
import LyricsPanelControls from "./LyricsPanelControls.vue";
import LyricsScroller from "./LyricsScroller/index.vue";
import StereoBridgePanel from "./StereoBridgePanel.vue";
import { useAudioLyricPanel } from "./useAudioLyricPanel";
import { useAudioMetadataEditor } from "./useAudioMetadataEditor";
import { useLyricsOverlayControls } from "./useLyricsOverlayControls";

const props = defineProps<{
  mediaKind: "video" | "audio";
  playback: PlaybackState | null;
  audioMeter: MediaAudioMeterPayload | null;
  lyrics: MediaLyricLine[];
  lyricsSource: string | null;
  lyricsCandidateId: string | null;
  lyricsCandidates: LyricsCandidateSummary[];
  lyricsFetching: boolean;
  title: string;
  artist: string;
  album: string;
  hasCoverArt: boolean;
  updatePlaybackSnapshot: (snapshot: MediaSnapshot) => void;
}>();

const currentSourcePath = computed(() => props.playback?.current_path ?? "");

const {
  adjustOffset,
  displayedOffsetSeconds,
  fineOffsetStepSeconds,
  handleDragPreview,
  handleDraggingChange,
  handleOffsetCommit,
  lyricsDragging,
  lyricsVisible,
  playerShowLyrics,
  resetOffset,
  toggleLyricsVisible,
  trackLyricsOffsetSeconds,
} = useLyricsOverlayControls({ currentSourcePath });

const {
  activeLyricIndex,
  contentInsetClass,
  hasCoverArt,
  hasLyrics,
  hasSyncedLyrics,
  headerDividerClass,
  headerPanelClass,
  isDark,
  isMasterMuted,
  lyricsSourceLabel,
  lyricsStageClass,
  mainPanelClass,
  orderedLyrics,
  playbackPositionSeconds,
  overlayScrimClass,
  overlayShellClass,
  showAudioLyricPanel,
  showLyricsContent,
  showStereoBridge,
  stereoBridgeChannels,
  stereoBridgeFrameClass,
  stereoCaptionClass,
  stereoMetaClass,
  titleTextClass,
  trackMetaClass,
  trackMetaLine,
  trackTitle,
  useCompactStereoBridge,
  useDenseStageLayout,
} = useAudioLyricPanel({
  mediaKind: toRef(props, "mediaKind"),
  playback: toRef(props, "playback"),
  audioMeter: toRef(props, "audioMeter"),
  lyrics: toRef(props, "lyrics"),
  lyricsSource: toRef(props, "lyricsSource"),
  lyricsFetching: toRef(props, "lyricsFetching"),
  lyricsOffsetSeconds: trackLyricsOffsetSeconds,
  lyricsVisible,
  title: toRef(props, "title"),
  artist: toRef(props, "artist"),
  album: toRef(props, "album"),
  hasCoverArt: toRef(props, "hasCoverArt"),
});

const metadataEditor = useAudioMetadataEditor({
  sourcePath: () => currentSourcePath.value,
  title: () => props.title,
  artist: () => props.artist,
  album: () => props.album,
  durationSeconds: () => props.playback?.duration_seconds ?? 0,
  lyrics: () => props.lyrics,
  lyricsSource: () => props.lyricsSource,
  hasCoverArt: () => props.hasCoverArt,
  updatePlaybackSnapshot: (snapshot) => props.updatePlaybackSnapshot(snapshot),
});

const {
  album: metadataEditorAlbum,
  artist: metadataEditorArtist,
  canEdit: canEditMetadata,
  coverLoading: metadataEditorCoverLoading,
  coverMarkedForRemoval: metadataEditorCoverMarkedForRemoval,
  coverPreviewUrl: metadataEditorCoverPreviewUrl,
  durationSeconds: metadataEditorDurationSeconds,
  embedLyrics: metadataEditorEmbedLyrics,
  lyricsLrc: metadataEditorLyricsLrc,
  lyricsSelectKey: metadataEditorLyricsSelectKey,
  lyricsSource: metadataEditorLyricsSource,
  open: metadataEditorOpen,
  pickCoverImage: metadataEditorPickCoverImage,
  removeCoverImage: metadataEditorRemoveCoverImage,
  saveEditor,
  saving: metadataEditorSaving,
  showEditor,
  title: metadataEditorTitle,
} = metadataEditor;
</script>

<template>
  <div
    v-if="showAudioLyricPanel"
    class="pointer-events-none absolute inset-0 z-20 overflow-hidden"
  >
    <div
      class="pointer-events-none absolute inset-0"
      :class="overlayScrimClass"
    />

    <div class="pointer-events-auto" :class="contentInsetClass" data-no-window-drag="true">
      <div :class="overlayShellClass">
        <StereoBridgePanel
          v-if="showStereoBridge"
          :channels="stereoBridgeChannels"
          :audio-meter="props.audioMeter"
          :frame-class="stereoBridgeFrameClass"
          :caption-class="stereoCaptionClass"
          :meta-class="stereoMetaClass"
          :compact="useCompactStereoBridge"
          :is-dark="isDark"
        />

        <div :class="mainPanelClass" data-no-window-drag="true">
          <div :class="[headerPanelClass, headerDividerClass, 'relative z-20 overflow-visible']">
            <div class="relative flex min-w-0 items-start justify-between gap-3">
              <div class="min-w-0 flex-1">
                <div class="flex min-w-0 items-center gap-1.5">
                  <span :class="titleTextClass">
                    {{ trackTitle }}
                  </span>
                  <button
                    v-if="canEditMetadata"
                    type="button"
                    class="pointer-events-auto inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-md border transition hover:opacity-90"
                    :class="isDark
                      ? 'border-white/14 bg-black/45 text-white/72 hover:border-white/24'
                      : 'border-black/10 bg-white/78 text-slate-600 hover:border-black/16'"
                    title="编辑歌曲信息"
                    data-no-window-drag="true"
                    @click="showEditor()"
                  >
                    <Icon icon="lucide:pencil" width="12" height="12" aria-hidden="true" />
                  </button>
                  <span
                    v-if="isMasterMuted"
                    class="shrink-0 rounded-full border px-1.5 py-px text-[9px] uppercase tracking-[0.16em]"
                    :class="isDark
                      ? 'border-red-300/20 bg-red-950/30 text-red-200/80'
                      : 'border-red-400/25 bg-red-50 text-red-600'"
                  >
                    Muted
                  </span>
                </div>
                <p
                  v-if="trackMetaLine"
                  :class="trackMetaClass"
                >
                  {{ trackMetaLine }}
                </p>
              </div>

              <div class="pointer-events-auto flex shrink-0 items-center gap-2">
                <LyricsPanelControls
                  :lyrics-visible="lyricsVisible"
                  :offset-seconds="displayedOffsetSeconds"
                  :offset-step-seconds="fineOffsetStepSeconds"
                  :has-synced-lyrics="hasSyncedLyrics"
                  :dragging="lyricsDragging"
                  :is-dark="isDark"
                  @toggle-visible="toggleLyricsVisible"
                  @reset-offset="resetOffset"
                  @adjust-offset="adjustOffset"
                />
                <LyricsSelect
                  v-if="props.lyricsCandidates.length > 1 && lyricsVisible"
                  mode="candidates"
                  compact
                  overlay
                  :is-dark="isDark"
                  :transparent-overlay="hasCoverArt"
                  :candidates="props.lyricsCandidates"
                  :fetching="props.lyricsFetching"
                  :selected-id="props.lyricsCandidateId"
                  :update-playback-snapshot="props.updatePlaybackSnapshot"
                />
                <span
                  v-if="lyricsSourceLabel || props.lyricsFetching"
                  class="text-[9px] uppercase tracking-[0.18em]"
                  :class="isDark ? 'text-white/45' : 'text-slate-400'"
                >
                  {{ props.lyricsFetching ? "Fetching…" : lyricsSourceLabel }}
                </span>
              </div>
            </div>
          </div>

          <div :class="lyricsStageClass">
            <div class="relative z-0 flex min-h-0 flex-1 flex-col">
              <LyricsScroller
                v-if="showLyricsContent || props.lyricsFetching"
                :lines="orderedLyrics"
                :active-index="activeLyricIndex"
                :playback-position-seconds="playbackPositionSeconds"
                :fetching="props.lyricsFetching"
                :dense="useDenseStageLayout"
                :transparent-overlay="hasCoverArt"
                :is-dark="isDark"
                :drag-enabled="hasSyncedLyrics"
                :base-offset-seconds="trackLyricsOffsetSeconds"
                @drag-preview="handleDragPreview"
                @dragging-change="handleDraggingChange"
                @offset-commit="handleOffsetCommit"
              />
              <LyricsEmptyState
                v-else-if="hasLyrics && !lyricsVisible"
                :message="playerShowLyrics ? '歌词已隐藏' : '歌词显示已在设置中关闭'"
                :action-label="playerShowLyrics ? '显示歌词' : undefined"
                :is-dark="isDark"
                @action="toggleLyricsVisible"
              />
              <LyricsEmptyState
                v-else
                message="未找到歌词"
                :is-dark="isDark"
              />
            </div>
          </div>
        </div>
      </div>
    </div>

    <AudioMetadataEditorModal
      v-model:open="metadataEditorOpen"
      v-model:title="metadataEditorTitle"
      v-model:artist="metadataEditorArtist"
      v-model:album="metadataEditorAlbum"
      v-model:lyrics-lrc="metadataEditorLyricsLrc"
      v-model:embed-lyrics="metadataEditorEmbedLyrics"
      :duration-seconds="metadataEditorDurationSeconds"
      :lyrics-source="metadataEditorLyricsSource"
      :lyrics-select-key="metadataEditorLyricsSelectKey"
      :cover-loading="metadataEditorCoverLoading"
      :cover-preview-url="metadataEditorCoverPreviewUrl"
      :cover-marked-for-removal="metadataEditorCoverMarkedForRemoval"
      :saving="metadataEditorSaving"
      :on-save="saveEditor"
      :on-pick-cover="metadataEditorPickCoverImage"
      :on-remove-cover="metadataEditorRemoveCoverImage"
    />
  </div>
</template>
