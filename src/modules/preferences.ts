import { computed, watchEffect } from "vue";
import { usePreferredDark, useStorage } from "@vueuse/core";

export type ThemePreference = "system" | "dark" | "light";

export type Preferences = {
  theme: ThemePreference;
};

const DEFAULT_PREFERENCES: Preferences = {
  theme: "system",
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
  };
}

