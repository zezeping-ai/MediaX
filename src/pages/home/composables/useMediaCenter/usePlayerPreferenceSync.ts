import { onMounted, watch, type Ref } from "vue";
import type { HardwareDecodeMode } from "@/modules/media-types";
import type { LyricsProviderPreferences } from "@/modules/preferences";

type UsePlayerPreferenceSyncOptions = {
  playerHwDecodeMode: Ref<HardwareDecodeMode>;
  playerAlwaysOnTop: Ref<boolean>;
  playerVideoScaleMode: Ref<"contain" | "cover">;
  playerResumeLastPosition: Ref<boolean>;
  playerAutoFetchOnlineLyrics: Ref<boolean>;
  playerLyricsProviders: Ref<LyricsProviderPreferences>;
  playerLrcApiBaseUrl: Ref<string>;
  applyHwDecodePreference: (mode: HardwareDecodeMode) => Promise<void>;
  applyAlwaysOnTopPreference: (enabled: boolean) => Promise<void>;
  applyVideoScaleModePreference: (mode: "contain" | "cover") => Promise<void>;
  applyResumeLastPositionPreference: (enabled: boolean) => Promise<void>;
  applyLyricsFetchSettingsPreference: (settings: {
    autoFetchOnlineLyrics: boolean;
    providers: LyricsProviderPreferences;
    lrcApiBaseUrl: string;
  }) => Promise<void>;
  onReady: () => Promise<void>;
};

export function usePlayerPreferenceSync(options: UsePlayerPreferenceSyncOptions) {
  function currentLyricsSettings() {
    return {
      autoFetchOnlineLyrics: options.playerAutoFetchOnlineLyrics.value,
      providers: options.playerLyricsProviders.value,
      lrcApiBaseUrl: options.playerLrcApiBaseUrl.value,
    };
  }

  onMounted(async () => {
    await options.onReady();
    await options.applyHwDecodePreference(options.playerHwDecodeMode.value);
    await options.applyAlwaysOnTopPreference(options.playerAlwaysOnTop.value);
    await options.applyVideoScaleModePreference(options.playerVideoScaleMode.value);
    await options.applyResumeLastPositionPreference(options.playerResumeLastPosition.value);
    await options.applyLyricsFetchSettingsPreference(currentLyricsSettings());
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

  watch(
    [
      options.playerAutoFetchOnlineLyrics,
      options.playerLyricsProviders,
      options.playerLrcApiBaseUrl,
    ],
    () => {
      void options.applyLyricsFetchSettingsPreference(currentLyricsSettings());
    },
    { deep: true, immediate: false },
  );
}
