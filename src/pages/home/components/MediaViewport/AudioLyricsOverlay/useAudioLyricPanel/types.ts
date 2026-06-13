import type { Ref } from "vue";
import type {
  MediaAudioMeterPayload,
  MediaLyricLine,
  PlaybackState,
} from "@/modules/media-types";

export type UseAudioLyricPanelOptions = {
  mediaKind: Readonly<Ref<"video" | "audio">>;
  playback: Readonly<Ref<PlaybackState | null>>;
  audioMeter: Readonly<Ref<MediaAudioMeterPayload | null>>;
  lyrics: Readonly<Ref<MediaLyricLine[]>>;
  lyricsSource: Readonly<Ref<string | null>>;
  lyricsFetching: Readonly<Ref<boolean>>;
  lyricsOffsetSeconds: Readonly<Ref<number>>;
  lyricsVisible: Readonly<Ref<boolean>>;
  title: Readonly<Ref<string>>;
  artist: Readonly<Ref<string>>;
  album: Readonly<Ref<string>>;
  hasCoverArt: Readonly<Ref<boolean>>;
};

export type StereoBridgeChannel = {
  key: "left" | "right";
  label: "L" | "R";
  bars: number[];
  holdBars: number[];
  peakHold: number;
  peakState: string;
  peakDbfs: string;
  holdDbfs: string;
};
