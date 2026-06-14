import { computed, type Ref } from "vue";

type UseLyricsSelectThemeOptions = {
  overlay: Ref<boolean>;
  isDark: Ref<boolean | undefined>;
  transparentOverlay: Ref<boolean>;
};

export function useLyricsSelectTheme(options: UseLyricsSelectThemeOptions) {
  const isDarkTheme = computed(() => options.isDark.value !== false);

  const triggerStyle = computed(() => {
    if (!isDarkTheme.value) {
      return undefined;
    }
    if (options.overlay.value && options.transparentOverlay.value) {
      return {
        borderColor: "rgba(255, 255, 255, 0.1)",
        backgroundColor: "rgba(0, 0, 0, 0.35)",
      };
    }
    if (options.overlay.value) {
      return {
        borderColor: "rgba(255, 255, 255, 0.12)",
        backgroundColor: "rgba(0, 0, 0, 0.45)",
      };
    }
    return {
      borderColor: "rgba(255, 255, 255, 0.08)",
      backgroundColor: "rgba(255, 255, 255, 0.03)",
    };
  });

  const menuStyle = computed(() => {
    if (!isDarkTheme.value) {
      return undefined;
    }
    if (options.overlay.value) {
      return {
        borderColor: "rgba(255, 255, 255, 0.1)",
        backgroundColor: "rgba(0, 0, 0, 0.82)",
      };
    }
    return {
      borderColor: "rgba(255, 255, 255, 0.08)",
      backgroundColor: "rgba(12, 12, 14, 0.96)",
    };
  });

  const triggerClass = computed(() => {
    if (!options.overlay.value) {
      return isDarkTheme.value
        ? "text-white/88 hover:border-white/12 hover:bg-white/5"
        : "border-slate-500/20 bg-white text-slate-800 hover:border-slate-400/35";
    }
    if (options.transparentOverlay.value) {
      return isDarkTheme.value
        ? "text-white/88 hover:border-white/16 hover:bg-black/45"
        : "border-black/10 bg-white/72 text-slate-800 hover:border-black/16 hover:bg-white/88";
    }
    return isDarkTheme.value
      ? "text-white/88 hover:border-white/24 hover:bg-black/58"
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
      return isDarkTheme.value
        ? "shadow-lg"
        : "border-slate-500/20 bg-white shadow-lg";
    }
    return isDarkTheme.value
      ? "shadow-[0_16px_40px_rgba(0,0,0,0.45)]"
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
    menuStyle,
    metaClass,
    titleClass,
    triggerClass,
    triggerStyle,
  };
}
