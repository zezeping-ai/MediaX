import { computed, type Ref } from "vue";
import {
  hasSyncedLyricTimings,
  orderLyricLines,
  resolveActiveLyricIndex,
} from "@/modules/lyrics";
import type { MediaLyricLine } from "@/modules/media-types";

type UseLyricsContentOptions = {
  lyrics: Readonly<Ref<MediaLyricLine[]>>;
  lyricsOffsetSeconds: Readonly<Ref<number>>;
  playbackPositionSeconds: Readonly<Ref<number>>;
};

export function useLyricsContent(options: UseLyricsContentOptions) {
  const orderedLyrics = computed(() => orderLyricLines(options.lyrics.value));
  const hasLyrics = computed(() => orderedLyrics.value.length > 0);
  const hasSyncedLyrics = computed(() => hasSyncedLyricTimings(orderedLyrics.value));
  const activeLyricIndex = computed(() =>
    resolveActiveLyricIndex(
      orderedLyrics.value,
      options.playbackPositionSeconds.value,
      options.lyricsOffsetSeconds.value,
    ),
  );

  return {
    activeLyricIndex,
    hasLyrics,
    hasSyncedLyrics,
    orderedLyrics,
  };
}
