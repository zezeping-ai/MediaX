import { computed, watchEffect } from "vue";
import { usePreferredDark, useStorage } from "@vueuse/core";

export type ThemePreference = "system" | "dark" | "light";

export type Preferences = {
  theme: ThemePreference;
  player: {
    hwDecodeEnabled: boolean;
    parseDebugEnabled: boolean;
    alwaysOnTop: boolean;
  };
};

const DEFAULT_PREFERENCES: Preferences = {
  theme: "system",
  player: {
    hwDecodeEnabled: false,
    // 默认打开：方便定位“打开/解析/解码”阶段的问题
    parseDebugEnabled: true,
    alwaysOnTop: false,
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
  };
}

