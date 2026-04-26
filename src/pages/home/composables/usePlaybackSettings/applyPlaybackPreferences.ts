import {
  applyAlwaysOnTopPreference,
  applyHwDecodePreference,
  applyVideoScaleModePreference,
} from "@/modules/player-settings-actions";
import type { PlayerVideoScaleMode } from "@/modules/preferences";
import type { HardwareDecodeMode, MediaSnapshot } from "@/modules/media-types";

type ConfigureDecoderMode = (mode: HardwareDecodeMode) => Promise<MediaSnapshot>;

export function createPlaybackPreferenceAppliers(configureDecoderMode: ConfigureDecoderMode) {
  async function applyHwDecode(mode: HardwareDecodeMode) {
    return applyHwDecodePreference(mode, configureDecoderMode);
  }

  async function applyAlwaysOnTop(enabled: boolean) {
    await applyAlwaysOnTopPreference(enabled);
  }

  async function applyVideoScaleMode(mode: PlayerVideoScaleMode) {
    await applyVideoScaleModePreference(mode);
  }

  return {
    applyAlwaysOnTop,
    applyHwDecode,
    applyVideoScaleMode,
  };
}
