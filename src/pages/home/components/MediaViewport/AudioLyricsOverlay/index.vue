<script setup lang="ts">
import { computed, toRef } from "vue";
import type {
  LyricsCandidateSummary,
  MediaAudioMeterPayload,
  MediaLyricLine,
  MediaSnapshot,
  PlaybackState,
} from "@/modules/media-types";
import LyricsCandidatePicker from "./LyricsCandidatePicker.vue";
import LyricsEmptyState from "./LyricsEmptyState.vue";
import LyricsPanelControls from "./LyricsPanelControls.vue";
import LyricsScroller from "./LyricsScroller/index.vue";
import StereoBridgePanel from "./StereoBridgePanel.vue";
import { useAudioLyricPanel } from "./useAudioLyricPanel";
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
                <div class="flex min-w-0 items-center gap-2">
                  <p :class="titleTextClass">
                    {{ trackTitle }}
                  </p>
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
                <LyricsCandidatePicker
                  v-if="props.lyricsCandidates.length > 1 && lyricsVisible"
                  :candidate-id="props.lyricsCandidateId"
                  :candidates="props.lyricsCandidates"
                  :fetching="props.lyricsFetching"
                  :transparent-overlay="hasCoverArt"
                  :is-dark="isDark"
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
  </div>
</template>
