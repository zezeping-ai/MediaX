import { useWindowSize } from "@vueuse/core";
import { computed, type Ref } from "vue";
import { formatLyricsSourceLabel } from "@/modules/lyrics";
import { usePreferences } from "@/modules/preferences";
import { playerDarkSurfaceClass } from "@/pages/home/composables/playerSurfaceTokens";

type UseOverlayLayoutOptions = {
  hasCoverArt: Readonly<Ref<boolean>>;
  lyricsSource: Readonly<Ref<string | null>>;
  lyricsVisible: Readonly<Ref<boolean>>;
  hasLyrics: Readonly<Ref<boolean>>;
  title: Readonly<Ref<string>>;
  artist: Readonly<Ref<string>>;
  album: Readonly<Ref<string>>;
  audioMeterSampleRate: Readonly<Ref<number | undefined>>;
  isMasterMuted: Readonly<Ref<boolean>>;
};

export function useOverlayLayout(options: UseOverlayLayoutOptions) {
  const { isDark } = usePreferences();
  const { height: viewportHeight, width: viewportWidth } = useWindowSize();

  const useDenseStageLayout = computed(() => viewportHeight.value < 820);
  const useCompactStereoBridge = computed(() => viewportHeight.value < 760 || viewportWidth.value < 1100);
  const showStereoBridge = computed(() => {
    const height = viewportHeight.value;
    const width = viewportWidth.value;
    if (height >= 620) {
      return true;
    }
    return height >= 560 && width >= 980;
  });

  const trackTitle = computed(() => options.title.value || "Unknown Track");
  const trackMetaLine = computed(() =>
    [
      options.artist.value || "",
      options.album.value && options.album.value !== options.artist.value ? options.album.value : "",
      options.audioMeterSampleRate.value
        ? `${Math.round(options.audioMeterSampleRate.value / 100) / 10}kHz`
        : "",
      options.hasCoverArt.value ? "Cover" : "",
    ].filter(Boolean).join(" · "),
  );
  const lyricsSourceLabel = computed(() => formatLyricsSourceLabel(options.lyricsSource.value));
  const showLyricsContent = computed(() => options.lyricsVisible.value && options.hasLyrics.value);

  const overlayScrimClass = computed(() => (
    isDark.value ? "bg-black/80" : "bg-white/80"
  ));
  const contentInsetClass = computed(() => (
    useDenseStageLayout.value
      ? "absolute inset-x-3 top-2.5 bottom-[6.5rem] md:inset-x-5 md:top-3 md:bottom-[7rem]"
      : "absolute inset-x-4 top-3 bottom-[7rem] md:inset-x-6 md:top-4 md:bottom-[7.5rem]"
  ));
  const overlayShellClass = computed(() => (
    useDenseStageLayout.value
      ? "mx-auto flex h-full w-full max-w-5xl flex-col gap-1.5"
      : "mx-auto flex h-full w-full max-w-5xl flex-col gap-2"
  ));
  const mainPanelClass = computed(() => {
    if (options.hasCoverArt.value) {
      const border = isDark.value ? "border-white/8" : "border-black/8";
      return useDenseStageLayout.value
        ? `relative flex min-h-0 flex-1 flex-col overflow-hidden rounded-2xl border ${border} bg-transparent`
        : `relative flex min-h-0 flex-1 flex-col overflow-hidden rounded-[20px] border ${border} bg-transparent`;
    }
    if (isDark.value) {
      const panel = playerDarkSurfaceClass.panel;
      const shadow = playerDarkSurfaceClass.panelShadow;
      return useDenseStageLayout.value
        ? `flex min-h-0 flex-1 flex-col overflow-hidden rounded-2xl border ${panel} ${shadow}`
        : `flex min-h-0 flex-1 flex-col overflow-hidden rounded-[20px] border ${panel} ${shadow}`;
    }
    return useDenseStageLayout.value
      ? "flex min-h-0 flex-1 flex-col overflow-hidden rounded-2xl border border-black/6 bg-white/82 shadow-[0_10px_32px_rgba(15,23,42,0.08)] backdrop-blur-md"
      : "flex min-h-0 flex-1 flex-col overflow-hidden rounded-[20px] border border-black/6 bg-white/86 shadow-[0_12px_36px_rgba(15,23,42,0.09)] backdrop-blur-md";
  });
  const headerPanelClass = computed(() => (
    useDenseStageLayout.value
      ? "shrink-0 border-b px-3 py-1.5"
      : "shrink-0 border-b px-3.5 py-2 md:px-4"
  ));
  const headerDividerClass = computed(() => (
    isDark.value ? "border-white/10" : "border-black/5"
  ));
  const lyricsStageClass = computed(() => (
    useDenseStageLayout.value
      ? "relative flex min-h-0 flex-1 flex-col px-3 pb-3 pt-1.5"
      : "relative flex min-h-0 flex-1 flex-col px-3.5 pb-4 pt-2 md:px-4 md:pb-5"
  ));
  const stereoBridgeFrameClass = computed(() => {
    if (options.hasCoverArt.value) {
      const border = isDark.value ? "border-white/8" : "border-black/8";
      return useDenseStageLayout.value
        ? `shrink-0 rounded-2xl border ${border} bg-transparent px-2 py-1.5`
        : `shrink-0 rounded-[18px] border ${border} bg-transparent px-2.5 py-2`;
    }
    if (isDark.value) {
      const stereo = playerDarkSurfaceClass.stereo;
      return useDenseStageLayout.value
        ? `shrink-0 rounded-2xl border ${stereo} px-2 py-1.5`
        : `shrink-0 rounded-[18px] border ${stereo} px-2.5 py-2`;
    }
    return useDenseStageLayout.value
      ? "shrink-0 rounded-2xl border border-black/6 bg-white/78 px-2 py-1.5 backdrop-blur-md"
      : "shrink-0 rounded-[18px] border border-black/6 bg-white/82 px-2.5 py-2 backdrop-blur-md";
  });
  const titleTextClass = computed(() => {
    const base = "min-w-0 truncate text-sm font-semibold leading-none md:text-base";
    if (isDark.value) {
      return options.hasCoverArt.value
        ? `${base} text-white [text-shadow:0_1px_10px_rgba(0,0,0,0.55)]`
        : `${base} text-white`;
    }
    return options.hasCoverArt.value
      ? `${base} text-slate-900 [text-shadow:0_1px_8px_rgba(255,255,255,0.7)]`
      : `${base} text-slate-900`;
  });
  const trackMetaClass = computed(() => (
    isDark.value
      ? (options.hasCoverArt.value
        ? "mt-0.5 truncate text-[11px] tracking-wide text-white/62 [text-shadow:0_1px_8px_rgba(0,0,0,0.45)]"
        : "min-w-0 truncate text-[11px] tracking-wide text-white/58")
      : (options.hasCoverArt.value
        ? "mt-0.5 truncate text-[11px] tracking-wide text-slate-600"
        : "mt-0.5 truncate text-[11px] tracking-wide text-slate-500")
  ));
  const stereoCaptionClass = computed(() => (isDark.value ? "text-white/50" : "text-slate-500"));
  const stereoMetaClass = computed(() => (isDark.value ? "text-white/58" : "text-slate-500"));
  const emptyLyricsClass = computed(() => (
    isDark.value ? "text-white/68" : "text-slate-600"
  ));

  return {
    contentInsetClass,
    emptyLyricsClass,
    headerDividerClass,
    headerPanelClass,
    isDark,
    lyricsSourceLabel,
    lyricsStageClass,
    mainPanelClass,
    overlayScrimClass,
    overlayShellClass,
    showLyricsContent,
    showStereoBridge,
    stereoBridgeFrameClass,
    stereoCaptionClass,
    stereoMetaClass,
    titleTextClass,
    trackMetaClass,
    trackMetaLine,
    trackTitle,
    useCompactStereoBridge,
    useDenseStageLayout,
  };
}
