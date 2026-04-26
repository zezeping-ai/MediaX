import { setMainWindowAlwaysOnTop, setMainWindowVideoScaleMode } from "./media-player";
import type { PlayerVideoScaleMode } from "./preferences";
import type { HardwareDecodeMode, MediaSnapshot } from "./media-types";

export async function applyHwDecodePreference(
  mode: HardwareDecodeMode,
  configureDecoderMode: (mode: HardwareDecodeMode) => Promise<MediaSnapshot>,
) {
  try {
    return await configureDecoderMode(mode);
  } catch {
    // Keep silent here; player surface already emits error events.
    return null;
  }
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
