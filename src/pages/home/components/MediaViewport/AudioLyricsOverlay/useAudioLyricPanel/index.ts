import { computed } from "vue";
import { useLyricsContent } from "./useLyricsContent";
import { useLyricsPlaybackTick } from "./useLyricsPlaybackTick";
import { useOverlayLayout } from "./useOverlayLayout";
import { useStereoBridge } from "./useStereoBridge";
import type { UseAudioLyricPanelOptions } from "./types";

export { formatLyricsSourceLabel } from "@/modules/lyrics";
export type { UseAudioLyricPanelOptions } from "./types";

export function useAudioLyricPanel(options: UseAudioLyricPanelOptions) {
  const showAudioLyricPanel = computed(() => options.mediaKind.value === "audio");
  const hasCoverArt = computed(() => options.hasCoverArt.value);
  const isMasterMuted = computed(() => options.playback.value?.muted ?? false);
  const audioMeterSampleRate = computed(() => options.audioMeter.value?.sample_rate);

  const { decay, stereoBridgeChannels } = useStereoBridge({
    audioMeter: options.audioMeter,
  });

  const { interpolatedPosition } = useLyricsPlaybackTick({
    enabled: showAudioLyricPanel,
    playback: options.playback,
    onTick: (elapsedMs) => {
      decay(elapsedMs);
    },
  });

  const {
    activeLyricIndex,
    hasLyrics,
    hasSyncedLyrics,
    orderedLyrics,
  } = useLyricsContent({
    lyrics: options.lyrics,
    lyricsOffsetSeconds: options.lyricsOffsetSeconds,
    playbackPositionSeconds: interpolatedPosition,
  });

  const layout = useOverlayLayout({
    hasCoverArt: options.hasCoverArt,
    lyricsSource: options.lyricsSource,
    lyricsVisible: options.lyricsVisible,
    hasLyrics,
    title: options.title,
    artist: options.artist,
    album: options.album,
    audioMeterSampleRate,
    isMasterMuted,
  });

  return {
    ...layout,
    activeLyricIndex,
    hasCoverArt,
    hasLyrics,
    hasSyncedLyrics,
    isMasterMuted,
    orderedLyrics,
    playbackPositionSeconds: interpolatedPosition,
    showAudioLyricPanel,
    stereoBridgeChannels,
    stageFrameClass: computed(() => ""),
  };
}
