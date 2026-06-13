import { computed } from "vue";
import { usePreferences } from "@/modules/preferences";

export function useAppSurfaceTheme() {
  const { isDark } = usePreferences();

  const sectionTitle = computed(() => (
    isDark.value ? "text-sm font-medium text-white/88" : "text-sm font-medium text-slate-900"
  ));
  const sectionSubtitle = computed(() => (
    isDark.value ? "text-xs text-white/42" : "text-xs text-slate-500"
  ));
  const sectionCaption = computed(() => (
    isDark.value
      ? "text-[10px] uppercase tracking-[0.16em] text-white/45"
      : "text-[10px] uppercase tracking-[0.16em] text-slate-500"
  ));
  const insetPanel = computed(() => (
    isDark.value
      ? "rounded-xl border border-white/8 bg-white/3 p-2.5"
      : "rounded-xl border border-black/8 bg-black/3 p-2.5"
  ));
  const listFrame = computed(() => (
    isDark.value ? "rounded-lg border border-white/8" : "rounded-lg border border-black/8"
  ));
  const listFrameOverflow = computed(() => (
    isDark.value
      ? "overflow-hidden rounded-lg border border-white/8"
      : "overflow-hidden rounded-lg border border-black/8"
  ));
  const countBadge = computed(() => (
    isDark.value
      ? "rounded-full bg-white/10 px-2 py-0.5 text-xs text-white/70"
      : "rounded-full bg-black/6 px-2 py-0.5 text-xs text-slate-600"
  ));
  const rowHover = computed(() => (
    isDark.value
      ? "hover:border-white/8 hover:bg-white/4"
      : "hover:border-black/8 hover:bg-black/4"
  ));
  const rowTitle = computed(() => (
    isDark.value ? "text-sm font-medium text-white/90" : "text-sm font-medium text-slate-900"
  ));
  const rowMeta = computed(() => (
    isDark.value ? "text-[11px] text-white/42" : "text-[11px] text-slate-500"
  ));
  const rowMuted = computed(() => (
    isDark.value ? "text-[10px] text-white/32" : "text-[10px] text-slate-400"
  ));
  const dragHandle = computed(() => (
    isDark.value ? "text-white/35 hover:text-white/70" : "text-slate-400 hover:text-slate-700"
  ));
  const urlText = computed(() => (
    isDark.value ? "text-sm text-white/88" : "text-sm text-slate-800"
  ));
  const listItemHover = computed(() => (
    isDark.value ? "hover:bg-white/3" : "hover:bg-black/4"
  ));
  const drawerMaskStyle = computed(() => ({
    backgroundColor: isDark.value ? "rgba(0,0,0,0.45)" : "rgba(15,23,42,0.22)",
  }));
  const emptyStateBackdrop = computed(() => (
    isDark.value ? "bg-black/80" : "bg-white/80"
  ));
  const emptyStatePanel = "mx-auto max-w-md px-6 py-8 text-center";

  return {
    countBadge,
    dragHandle,
    drawerMaskStyle,
    emptyStateBackdrop,
    emptyStatePanel,
    insetPanel,
    isDark,
    listFrame,
    listFrameOverflow,
    listItemHover,
    rowHover,
    rowMeta,
    rowMuted,
    rowTitle,
    sectionCaption,
    sectionSubtitle,
    sectionTitle,
    urlText,
  };
}
