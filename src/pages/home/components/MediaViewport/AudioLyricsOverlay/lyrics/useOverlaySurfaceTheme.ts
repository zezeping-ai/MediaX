import { computed, type Ref } from "vue";

type OverlaySurfaceThemeOptions = {
  isDark?: Ref<boolean | undefined>;
};

/** 歌词 overlay 子组件共用的 light/dark 主题选择 */
export function useOverlaySurfaceTheme(options: OverlaySurfaceThemeOptions) {
  const isDarkTheme = computed(() => options.isDark?.value !== false);

  function pick(light: string, dark: string) {
    return computed(() => (isDarkTheme.value ? dark : light));
  }

  const onDarkSurface = isDarkTheme;

  return {
    isDarkTheme,
    onDarkSurface,
    pick,
  };
}
