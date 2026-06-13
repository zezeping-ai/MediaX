import { computed, type Ref } from "vue";
import { useOverlaySurfaceTheme } from "../../lyrics/useOverlaySurfaceTheme";
import { playerDarkSurfaceClass } from "@/pages/home/composables/playerSurfaceTokens";

type UseLyricsScrollerThemeOptions = {
  dense: Ref<boolean>;
  isDark?: Ref<boolean | undefined>;
  transparentOverlay: Ref<boolean>;
};

export function useLyricsScrollerTheme(options: UseLyricsScrollerThemeOptions) {
  const { isDarkTheme, onDarkSurface, pick } = useOverlaySurfaceTheme({
    isDark: options.isDark,
  });

  const emptyStateClass = pick("text-slate-600", "text-white/68");
  const edgeFadeTopClass = computed(() => {
    if (isDarkTheme.value) {
      return options.transparentOverlay.value ? "from-black/80" : playerDarkSurfaceClass.lyricsEdgeFade;
    }
    return options.transparentOverlay.value ? "from-white/80" : "from-white/72";
  });
  const centerRailClass = pick("border-black/10 bg-black/4", "border-white/16 bg-white/6");
  const lineSlotClass = computed(() => (
    options.dense.value
      ? "shrink-0 px-2 py-0.5 text-[13px] leading-snug md:text-[14px]"
      : "shrink-0 px-2 py-0.5 text-[14px] leading-snug md:text-[15px]"
  ));

  function lineTextClass(active: boolean) {
    if (active) {
      return onDarkSurface.value
        ? "max-w-full font-semibold text-white"
        : "max-w-full font-semibold text-slate-900";
    }
    return onDarkSurface.value
      ? "line-clamp-1 font-normal text-white/28"
      : "line-clamp-1 font-normal text-slate-400/80";
  }

  return {
    centerRailClass,
    edgeFadeBottomClass: edgeFadeTopClass,
    edgeFadeTopClass,
    emptyStateClass,
    lineSlotClass,
    lineTextClass,
  };
}
