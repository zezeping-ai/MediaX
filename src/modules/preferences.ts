import { computed, watchEffect } from "vue";
import { usePreferredDark, useStorage } from "@vueuse/core";
import type { HardwareDecodeMode } from "./media-types";

export type ThemePreference = "system" | "dark" | "light";
export type PlayerVideoScaleMode = "contain" | "cover";

export type LyricsProviderPreferences = {
  lrclib: boolean;
  lrcapi: boolean;
  kugou: boolean;
  netease: boolean;
};

export type Preferences = {
  theme: ThemePreference;
  player: {
    hwDecodeMode: HardwareDecodeMode;
    alwaysOnTop: boolean;
    videoScaleMode: PlayerVideoScaleMode;
    showDownlinkSpeed: boolean;
    showUplinkSpeed: boolean;
    resumeLastPosition: boolean;
    autoFetchOnlineLyrics: boolean;
    showLyrics: boolean;
    lyricsProviders: LyricsProviderPreferences;
    lrcApiBaseUrl: string;
  };
};

const DEFAULT_LYRICS_PROVIDERS: LyricsProviderPreferences = {
  lrclib: true,
  lrcapi: true,
  kugou: true,
  netease: true,
};

const DEFAULT_PREFERENCES: Preferences = {
  theme: "system",
  player: {
    hwDecodeMode: "auto",
    alwaysOnTop: true,
    videoScaleMode: "contain",
    showDownlinkSpeed: true,
    showUplinkSpeed: true,
    resumeLastPosition: true,
    autoFetchOnlineLyrics: true,
    showLyrics: true,
    lyricsProviders: { ...DEFAULT_LYRICS_PROVIDERS },
    lrcApiBaseUrl: "",
  },
};

/**
 * 企业级可维护性：统一的偏好配置入口
 * - 使用 localStorage 持久化（via VueUse `useStorage`）
 * - 同步主题到整个应用（含偏好设置窗口）
 */
export function usePreferences() {
  const preferences = useStorage<Preferences>(
    "mediax.preferences",
    DEFAULT_PREFERENCES,
    localStorage,
    {
      mergeDefaults: true,
    },
  );

  watchEffect(() => {
    const player = preferences.value.player;
    if (
      !player
      || !player.videoScaleMode
      || typeof player.showDownlinkSpeed !== "boolean"
      || typeof player.showUplinkSpeed !== "boolean"
      || typeof player.resumeLastPosition !== "boolean"
      || typeof player.autoFetchOnlineLyrics !== "boolean"
      || typeof player.showLyrics !== "boolean"
      || !player.lyricsProviders
    ) {
      preferences.value = {
        ...preferences.value,
        player: {
          hwDecodeMode: resolveStoredHwDecodeMode(player),
          alwaysOnTop: player?.alwaysOnTop ?? DEFAULT_PREFERENCES.player.alwaysOnTop,
          videoScaleMode: player?.videoScaleMode ?? DEFAULT_PREFERENCES.player.videoScaleMode,
          showDownlinkSpeed:
            player?.showDownlinkSpeed ?? DEFAULT_PREFERENCES.player.showDownlinkSpeed,
          showUplinkSpeed:
            player?.showUplinkSpeed ?? DEFAULT_PREFERENCES.player.showUplinkSpeed,
          resumeLastPosition:
            player?.resumeLastPosition ?? DEFAULT_PREFERENCES.player.resumeLastPosition,
          autoFetchOnlineLyrics:
            player?.autoFetchOnlineLyrics ?? DEFAULT_PREFERENCES.player.autoFetchOnlineLyrics,
          showLyrics: player?.showLyrics ?? DEFAULT_PREFERENCES.player.showLyrics,
          lyricsProviders: {
            lrclib: player?.lyricsProviders?.lrclib ?? DEFAULT_LYRICS_PROVIDERS.lrclib,
            lrcapi: player?.lyricsProviders?.lrcapi ?? DEFAULT_LYRICS_PROVIDERS.lrcapi,
            kugou: player?.lyricsProviders?.kugou ?? DEFAULT_LYRICS_PROVIDERS.kugou,
            netease: player?.lyricsProviders?.netease ?? DEFAULT_LYRICS_PROVIDERS.netease,
          },
          lrcApiBaseUrl: player?.lrcApiBaseUrl ?? DEFAULT_PREFERENCES.player.lrcApiBaseUrl,
        },
      };
    }
  });

  const preferredDark = usePreferredDark();

  const resolvedTheme = computed<"dark" | "light">(() => {
    if (preferences.value.theme === "system") {
      return preferredDark.value ? "dark" : "light";
    }
    return preferences.value.theme;
  });

  watchEffect(() => {
    document.documentElement.dataset.theme = resolvedTheme.value;
  });

  return {
    preferences,
    resolvedTheme,
    isDark: computed(() => resolvedTheme.value === "dark"),
    theme: computed({
      get: () => preferences.value.theme,
      set: (v: ThemePreference) => {
        preferences.value = { ...preferences.value, theme: v };
      },
    }),
    playerHwDecodeMode: computed({
      get: () => preferences.value.player.hwDecodeMode,
      set: (v: HardwareDecodeMode) => {
        preferences.value = {
          ...preferences.value,
          player: { ...preferences.value.player, hwDecodeMode: v },
        };
      },
    }),
    playerAlwaysOnTop: computed({
      get: () => preferences.value.player.alwaysOnTop,
      set: (v: boolean) => {
        preferences.value = {
          ...preferences.value,
          player: { ...preferences.value.player, alwaysOnTop: v },
        };
      },
    }),
    playerVideoScaleMode: computed({
      get: () => preferences.value.player.videoScaleMode,
      set: (v: PlayerVideoScaleMode) => {
        preferences.value = {
          ...preferences.value,
          player: { ...preferences.value.player, videoScaleMode: v },
        };
      },
    }),
    playerShowDownlinkSpeed: computed({
      get: () => preferences.value.player.showDownlinkSpeed,
      set: (v: boolean) => {
        preferences.value = {
          ...preferences.value,
          player: { ...preferences.value.player, showDownlinkSpeed: v },
        };
      },
    }),
    playerShowUplinkSpeed: computed({
      get: () => preferences.value.player.showUplinkSpeed,
      set: (v: boolean) => {
        preferences.value = {
          ...preferences.value,
          player: { ...preferences.value.player, showUplinkSpeed: v },
        };
      },
    }),
    playerResumeLastPosition: computed({
      get: () => preferences.value.player.resumeLastPosition,
      set: (v: boolean) => {
        preferences.value = {
          ...preferences.value,
          player: { ...preferences.value.player, resumeLastPosition: v },
        };
      },
    }),
    playerAutoFetchOnlineLyrics: computed({
      get: () => preferences.value.player.autoFetchOnlineLyrics,
      set: (v: boolean) => {
        preferences.value = {
          ...preferences.value,
          player: { ...preferences.value.player, autoFetchOnlineLyrics: v },
        };
      },
    }),
    playerShowLyrics: computed({
      get: () => preferences.value.player.showLyrics,
      set: (v: boolean) => {
        preferences.value = {
          ...preferences.value,
          player: { ...preferences.value.player, showLyrics: v },
        };
      },
    }),
    playerLyricsProviders: computed({
      get: () => preferences.value.player.lyricsProviders,
      set: (v: LyricsProviderPreferences) => {
        preferences.value = {
          ...preferences.value,
          player: { ...preferences.value.player, lyricsProviders: v },
        };
      },
    }),
    playerLrcApiBaseUrl: computed({
      get: () => preferences.value.player.lrcApiBaseUrl,
      set: (v: string) => {
        preferences.value = {
          ...preferences.value,
          player: { ...preferences.value.player, lrcApiBaseUrl: v },
        };
      },
    }),
  };
}

type LegacyStoredPlayerPreferences = Partial<Preferences["player"]> & {
  hwDecodeEnabled?: boolean;
};

function resolveStoredHwDecodeMode(
  player: Preferences["player"] | LegacyStoredPlayerPreferences | undefined,
): HardwareDecodeMode {
  const mode = player?.hwDecodeMode;
  if (mode === "auto" || mode === "on" || mode === "off") {
    return mode;
  }
  if (player && "hwDecodeEnabled" in player && typeof player.hwDecodeEnabled === "boolean") {
    return player.hwDecodeEnabled ? "auto" : "off";
  }
  return DEFAULT_PREFERENCES.player.hwDecodeMode;
}
