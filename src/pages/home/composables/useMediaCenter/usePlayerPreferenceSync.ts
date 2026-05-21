import { onMounted, watch, type Ref } from "vue";
import type { HardwareDecodeMode } from "@/modules/media-types";

type UsePlayerPreferenceSyncOptions = {
  playerHwDecodeMode: Ref<HardwareDecodeMode>;
  playerAlwaysOnTop: Ref<boolean>;
  playerVideoScaleMode: Ref<"contain" | "cover">;
  playerResumeLastPosition: Ref<boolean>;
  applyHwDecodePreference: (mode: HardwareDecodeMode) => Promise<void>;
  applyAlwaysOnTopPreference: (enabled: boolean) => Promise<void>;
  applyVideoScaleModePreference: (mode: "contain" | "cover") => Promise<void>;
  applyResumeLastPositionPreference: (enabled: boolean) => Promise<void>;
  onReady: () => Promise<void>;
};

export function usePlayerPreferenceSync(options: UsePlayerPreferenceSyncOptions) {
  onMounted(async () => {
    await options.onReady();
    await options.applyHwDecodePreference(options.playerHwDecodeMode.value);
    await options.applyAlwaysOnTopPreference(options.playerAlwaysOnTop.value);
    await options.applyVideoScaleModePreference(options.playerVideoScaleMode.value);
    await options.applyResumeLastPositionPreference(options.playerResumeLastPosition.value);
  });

  watch(
    options.playerHwDecodeMode,
    (mode, previousMode) => {
      if (previousMode === undefined || mode === previousMode) {
        return;
      }
      void options.applyHwDecodePreference(mode);
    },
    { immediate: false },
  );

  watch(
    options.playerAlwaysOnTop,
    (enabled) => {
      void options.applyAlwaysOnTopPreference(enabled);
    },
    { immediate: false },
  );

  watch(
    options.playerVideoScaleMode,
    (mode) => {
      void options.applyVideoScaleModePreference(mode);
    },
    { immediate: false },
  );

  watch(
    options.playerResumeLastPosition,
    (enabled) => {
      void options.applyResumeLastPositionPreference(enabled);
    },
    { immediate: false },
  );
}
