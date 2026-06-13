import {
  playbackSetLyricsFetchSettings,
  playbackSetResumeLastPosition,
  setMainWindowAlwaysOnTop,
  setMainWindowVideoScaleMode,
  setMainWindowVideoPictureTune,
} from "./media-player";
import type { LyricsProviderPreferences, PlayerVideoScaleMode } from "./preferences";
import type { VideoPictureTune } from "./video-picture-tune";
import { normalizeVideoPictureTune } from "./video-picture-tune";
import type { HardwareDecodeMode, MediaSnapshot } from "./media-types";

let hwDecodeApplyQueue: Promise<MediaSnapshot | null> = Promise.resolve(null);

export async function applyHwDecodePreference(
  mode: HardwareDecodeMode,
  configureDecoderMode: (mode: HardwareDecodeMode) => Promise<MediaSnapshot>,
) {
  const run = hwDecodeApplyQueue.then(async () => {
    try {
      return await configureDecoderMode(mode);
    } catch {
      // Keep silent here; player surface already emits error events.
      return null;
    }
  });
  hwDecodeApplyQueue = run.then(() => null, () => null);
  return run;
}

export async function applyAlwaysOnTopPreference(enabled: boolean) {
  try {
    await setMainWindowAlwaysOnTop(enabled);
  } catch {
    // Keep silent here; window behavior should not block settings flow.
  }
}

export async function applyVideoScaleModePreference(mode: PlayerVideoScaleMode) {
  try {
    await setMainWindowVideoScaleMode(mode);
  } catch {
    // Keep silent here; rendering preference should not break playback flow.
  }
}

export async function applyVideoPictureTunePreference(tune: VideoPictureTune) {
  try {
    await setMainWindowVideoPictureTune(normalizeVideoPictureTune(tune));
  } catch {
    // Keep silent here; rendering preference should not break playback flow.
  }
}

export async function applyResumeLastPositionPreference(enabled: boolean) {
  try {
    await playbackSetResumeLastPosition(enabled);
  } catch {
    // Keep silent here; resume preference should not block settings flow.
  }
}

export async function applyLyricsFetchSettingsPreference(settings: {
  autoFetchOnlineLyrics: boolean;
  providers: LyricsProviderPreferences;
  lrcApiBaseUrl: string;
}) {
  try {
    await playbackSetLyricsFetchSettings(settings);
  } catch {
    // Keep silent here; lyrics preference should not block settings flow.
  }
}
