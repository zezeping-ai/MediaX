import { setMainWindowAlwaysOnTop, setMainWindowVideoScaleMode } from "./media-player";
import type { PlayerVideoScaleMode } from "./preferences";
import type { HardwareDecodeMode, MediaSnapshot } from "./media-types";

export async function applyHwDecodePreference(
  enabled: boolean,
  configureDecoderMode: (mode: HardwareDecodeMode) => Promise<MediaSnapshot>,
) {
  const mode: HardwareDecodeMode = enabled ? "auto" : "off";
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
