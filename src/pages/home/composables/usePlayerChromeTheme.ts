import { computed } from "vue";
import { usePreferences } from "@/modules/preferences";
import { playerDarkSurfaceClass } from "./playerSurfaceTokens";

export function usePlayerChromeTheme() {
  const { isDark } = usePreferences();

  const controlsShell = computed(() => (
    isDark.value
      ? `w-full overflow-visible rounded-t-2xl rounded-b-none border border-b-0 shadow-[0_-4px_20px_rgba(0,0,0,0.18)] ${playerDarkSurfaceClass.shell}`
      : "w-full overflow-visible rounded-t-2xl rounded-b-none border border-black/8 border-b-0 bg-[linear-gradient(180deg,rgba(255,255,255,0.88)_0%,rgba(255,255,255,0.96)_100%)] shadow-[0_-4px_16px_rgba(15,23,42,0.06)] backdrop-blur-xl"
  ));

  const circleBtnBase = computed(() => (
    isDark.value
      ? "inline-flex h-8 min-h-8 w-8 min-w-8 items-center justify-center rounded-full p-0! leading-none transition-colors duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white/25 focus-visible:ring-offset-2 focus-visible:ring-offset-black/30 [&_.ant-btn-icon]:m-0! [&_.ant-btn-icon]:flex [&_.ant-btn-icon]:items-center [&_.ant-btn-icon]:justify-center disabled:opacity-55 disabled:cursor-not-allowed"
      : "inline-flex h-8 min-h-8 w-8 min-w-8 items-center justify-center rounded-full p-0! leading-none transition-colors duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-slate-400/35 focus-visible:ring-offset-2 focus-visible:ring-offset-white/80 [&_.ant-btn-icon]:m-0! [&_.ant-btn-icon]:flex [&_.ant-btn-icon]:items-center [&_.ant-btn-icon]:justify-center disabled:opacity-55 disabled:cursor-not-allowed"
  ));

  const circleBtnGhost = computed(() => (
    isDark.value
      ? "border border-white/12 bg-white/6 text-white/90 hover:bg-white/10 hover:text-white"
      : "border border-black/10 bg-black/4 text-slate-700 hover:bg-black/8 hover:text-slate-900"
  ));

  const circleBtnPrimary = computed(() => (
    isDark.value
      ? "border-none bg-white text-slate-950 hover:bg-white"
      : "border-none bg-slate-900 text-white hover:bg-slate-800"
  ));

  const pillBase = computed(() => (
    "inline-flex items-center gap-1.5 px-0 py-0"
  ));

  const tinyPillBtn = computed(() => (
    isDark.value
      ? "inline-flex h-7 items-center gap-1 rounded-full border border-white/10 bg-white/5 px-2.5 text-[11px] font-semibold text-white/85 hover:bg-white/10 hover:text-white transition-colors duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white/25 focus-visible:ring-offset-2 focus-visible:ring-offset-black/30 [&_.ant-btn-icon]:m-0!"
      : "inline-flex h-7 items-center gap-1 rounded-full border border-black/8 bg-black/4 px-2.5 text-[11px] font-semibold text-slate-700 hover:bg-black/8 hover:text-slate-900 transition-colors duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-slate-400/30 focus-visible:ring-offset-2 focus-visible:ring-offset-white [&_.ant-btn-icon]:m-0!"
  ));

  const divider = computed(() => (isDark.value ? "h-4 w-px bg-white/10" : "h-4 w-px bg-black/10"));

  const timelineTime = computed(() => (
    isDark.value ? "text-white/70" : "text-slate-600"
  ));
  const timelineTimeCurrent = computed(() => (
    isDark.value ? "text-white/85" : "text-slate-900"
  ));
  const timelineTimeMuted = computed(() => (
    isDark.value ? "text-white/60" : "text-slate-500"
  ));
  const timelineRail = computed(() => (
    isDark.value ? "bg-white/12" : "bg-black/10"
  ));
  const timelineBuffered = computed(() => (
    isDark.value ? "bg-white/25" : "bg-black/18"
  ));
  const timelinePlayed = computed(() => (
    isDark.value ? "bg-white/85" : "bg-slate-800"
  ));
  const timelineSlider = computed(() => (
    isDark.value
      ? "[&_.ant-slider-handle::after]:bg-white [&_.ant-slider-handle::after]:shadow-[0_0_0_2px_rgba(255,255,255,0.26)]"
      : "[&_.ant-slider-handle::after]:bg-slate-900 [&_.ant-slider-handle::after]:shadow-[0_0_0_2px_rgba(15,23,42,0.18)]"
  ));

  const masterSlider = computed(() => (
    isDark.value
      ? "[&_.ant-slider]:m-0! [&_.ant-slider]:h-2.5! [&_.ant-slider]:min-h-2.5! [&_.ant-slider]:py-0! [&_.ant-slider-handle::after]:bg-white [&_.ant-slider-handle::after]:shadow-[0_0_0_2px_rgba(255,255,255,0.20)] [&_.ant-slider-handle:hover]:opacity-100 [&_.ant-slider-handle]:opacity-90 [&_.ant-slider-rail]:h-[3px] [&_.ant-slider-rail]:bg-white/12 [&_.ant-slider-track]:h-[3px] [&_.ant-slider-track]:bg-white/70"
      : "[&_.ant-slider]:m-0! [&_.ant-slider]:h-2.5! [&_.ant-slider]:min-h-2.5! [&_.ant-slider]:py-0! [&_.ant-slider-handle::after]:bg-slate-900 [&_.ant-slider-handle::after]:shadow-[0_0_0_2px_rgba(15,23,42,0.16)] [&_.ant-slider-handle:hover]:opacity-100 [&_.ant-slider-handle]:opacity-90 [&_.ant-slider-rail]:h-[3px] [&_.ant-slider-rail]:bg-black/10 [&_.ant-slider-track]:h-[3px] [&_.ant-slider-track]:bg-slate-700"
  ));

  const channelPanel = computed(() => (
    isDark.value
      ? `absolute bottom-[calc(100%+12px)] left-1/2 z-20 w-[min(340px,calc(100vw-32px))] -translate-x-1/2 rounded-2xl border p-3 shadow-[0_18px_48px_rgba(0,0,0,0.42)] backdrop-blur-2xl ${playerDarkSurfaceClass.shell}`
      : "absolute bottom-[calc(100%+12px)] left-1/2 z-20 w-[min(340px,calc(100vw-32px))] -translate-x-1/2 rounded-2xl border border-black/8 bg-[linear-gradient(180deg,rgba(255,255,255,0.96)_0%,rgba(248,250,252,0.94)_100%)] p-3 shadow-[0_18px_40px_rgba(15,23,42,0.14)] backdrop-blur-2xl"
  ));

  const floatingIconButton = computed(() => (
    isDark.value
      ? "inline-flex h-9 w-9 items-center justify-center rounded-full border border-white/12 bg-black/40 text-white/88 backdrop-blur-md transition hover:bg-black/55"
      : "inline-flex h-9 w-9 items-center justify-center rounded-full border border-black/8 bg-white/82 text-slate-700 backdrop-blur-md transition hover:bg-white"
  ));

  return {
    isDark,
    channelPanel,
    circleBtnBase,
    circleBtnGhost,
    circleBtnPrimary,
    controlsShell,
    divider,
    floatingIconButton,
    masterSlider,
    pillBase,
    tinyPillBtn,
    timelineBuffered,
    timelinePlayed,
    timelineRail,
    timelineSlider,
    timelineTime,
    timelineTimeCurrent,
    timelineTimeMuted,
  };
}
