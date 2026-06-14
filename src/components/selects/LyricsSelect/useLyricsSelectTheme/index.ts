import { computed, type Ref } from "vue";

type UseLyricsSelectThemeOptions = {
  overlay: Ref<boolean>;
  isDark: Ref<boolean | undefined>;
  transparentOverlay: Ref<boolean>;
};

export function useLyricsSelectTheme(options: UseLyricsSelectThemeOptions) {
  const isDarkTheme = computed(() => options.isDark.value !== false);

  const triggerClass = computed(() => {
    if (!options.overlay.value) {
      return "border-slate-500/20 bg-white text-slate-800 hover:border-slate-400/35 dark:border-white/14 dark:bg-zinc-900 dark:text-white/88 dark:hover:border-white/24";
    }
    if (options.transparentOverlay.value) {
      return isDarkTheme.value
        ? "border-white/10 bg-black/35 text-white/88 hover:border-white/16 hover:bg-black/45"
        : "border-black/10 bg-white/72 text-slate-800 hover:border-black/16 hover:bg-white/88";
    }
    return isDarkTheme.value
      ? "border-white/14 bg-black/45 text-white/88 hover:border-white/24 hover:bg-black/58"
      : "border-black/10 bg-white/78 text-slate-800 hover:border-black/16 hover:bg-white/90";
  });

  const badgeClass = computed(() => (
    isDarkTheme.value ? "bg-white/10 text-white/72" : "bg-black/6 text-slate-600"
  ));

  const chevronClass = computed(() => (
    isDarkTheme.value ? "text-white/45" : "text-slate-400"
  ));

  const menuClass = computed(() => {
    if (!options.overlay.value) {
      return "border-slate-500/20 bg-white shadow-lg dark:border-white/12 dark:bg-zinc-900";
    }
    return isDarkTheme.value
      ? "border-white/12 bg-black/82 shadow-[0_16px_40px_rgba(0,0,0,0.45)]"
      : "border-black/10 bg-white/96 shadow-[0_16px_40px_rgba(15,23,42,0.14)]";
  });

  const itemHoverClass = computed(() => (
    isDarkTheme.value ? "hover:bg-white/8" : "hover:bg-black/5"
  ));

  const itemActiveClass = computed(() => (
    isDarkTheme.value ? "bg-white/10" : "bg-black/6"
  ));

  const indexClass = computed(() => (
    isDarkTheme.value ? "text-white/42" : "text-slate-400"
  ));

  const titleClass = computed(() => (
    isDarkTheme.value ? "text-white/90" : "text-slate-800"
  ));

  const metaClass = computed(() => (
    isDarkTheme.value ? "text-white/45" : "text-slate-500"
  ));

  return {
    badgeClass,
    chevronClass,
    indexClass,
    itemActiveClass,
    itemHoverClass,
    menuClass,
    metaClass,
    titleClass,
    triggerClass,
  };
}
