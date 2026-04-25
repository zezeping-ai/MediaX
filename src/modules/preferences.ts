import { computed, watchEffect } from "vue";
import { usePreferredDark, useStorage } from "@vueuse/core";

export type ThemePreference = "system" | "dark" | "light";
export type PlayerVideoScaleMode = "contain" | "cover";

export type Preferences = {
  theme: ThemePreference;
  player: {
    hwDecodeEnabled: boolean;
    parseDebugEnabled: boolean;
    alwaysOnTop: boolean;
    videoScaleMode: PlayerVideoScaleMode;
  };
};

const DEFAULT_PREFERENCES: Preferences = {
  theme: "system",
  player: {
    hwDecodeEnabled: true,
    // 默认打开：方便定位“打开/解析/解码”阶段的问题
    parseDebugEnabled: true,
    alwaysOnTop: true,
    // 默认“自适应”：完整显示视频，必要时留黑边。
    videoScaleMode: "contain",
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

  // Backfill newly added nested player preferences for older localStorage snapshots.
  // `mergeDefaults` may not deep-merge nested objects in all historical states.
  watchEffect(() => {
    const player = preferences.value.player;
    if (!player || !player.videoScaleMode) {
      preferences.value = {
        ...preferences.value,
        player: {
          hwDecodeEnabled: player?.hwDecodeEnabled ?? DEFAULT_PREFERENCES.player.hwDecodeEnabled,
          parseDebugEnabled:
            player?.parseDebugEnabled ?? DEFAULT_PREFERENCES.player.parseDebugEnabled,
          alwaysOnTop: player?.alwaysOnTop ?? DEFAULT_PREFERENCES.player.alwaysOnTop,
          videoScaleMode: DEFAULT_PREFERENCES.player.videoScaleMode,
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
    // 统一使用 data-theme，方便全局样式与多窗口一致
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
    playerHwDecodeEnabled: computed({
      get: () => preferences.value.player.hwDecodeEnabled,
      set: (v: boolean) => {
        preferences.value = {
          ...preferences.value,
          player: { ...preferences.value.player, hwDecodeEnabled: v },
        };
      },
    }),
    playerParseDebugEnabled: computed({
      get: () => preferences.value.player.parseDebugEnabled,
      set: (v: boolean) => {
        preferences.value = {
          ...preferences.value,
          player: { ...preferences.value.player, parseDebugEnabled: v },
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
  };
}

