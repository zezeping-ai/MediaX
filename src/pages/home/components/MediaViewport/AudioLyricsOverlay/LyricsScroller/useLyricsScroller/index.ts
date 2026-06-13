import { computed, onBeforeUnmount, ref, toRef, watch, type ComponentPublicInstance, type Ref } from "vue";
import { useElementSize } from "@vueuse/core";
import type { MediaLyricLine } from "@/modules/media-types";
import { computeLyricsOffsetForLine } from "@/modules/lyrics";
import { findLineIndexAtViewportCenter, useLyricsCenterRail } from "./useLyricsCenterRail";
import { useLyricsDragSync } from "./useLyricsDragSync";
import { useLyricsScrollerTheme } from "./theme";

type UseLyricsScrollerOptions = {
  lines: Ref<MediaLyricLine[]>;
  activeIndex: Ref<number>;
  playbackPositionSeconds: Ref<number>;
  dense: Ref<boolean>;
  transparentOverlay: Ref<boolean>;
  isDark?: Ref<boolean | undefined>;
  dragEnabled: Ref<boolean>;
  baseOffsetSeconds: Ref<number>;
  onDragPreview: (deltaSeconds: number) => void;
  onOffsetCommit: (nextOffsetSeconds: number) => void;
  onDraggingChange: (dragging: boolean) => void;
};

function resolveActiveIndex(index: number, lineCount: number) {
  if (index >= 0) {
    return index;
  }
  return lineCount > 0 ? 0 : -1;
}

export function useLyricsScroller(options: UseLyricsScrollerOptions) {
  const viewportRef = ref<HTMLElement | null>(null);
  const contentRef = ref<HTMLElement | null>(null);
  const lineRefs = ref<Record<number, HTMLElement | null>>({});
  const frozenScrollIndex = ref(-1);
  const calibrationPlaybackSeconds = ref(0);

  const { height: viewportHeight } = useElementSize(viewportRef);
  const listPaddingY = computed(() => {
    const height = viewportRef.value?.clientHeight ?? viewportHeight.value;
    return Math.max(0, height / 2);
  });

  const theme = useLyricsScrollerTheme({
    dense: options.dense,
    isDark: options.isDark,
    transparentOverlay: options.transparentOverlay,
  });

  function resolveCenteredLineIndex() {
    return findLineIndexAtViewportCenter({
      viewportEl: viewportRef.value,
      lineCount: options.lines.value.length,
      lineRefs: lineRefs.value,
    });
  }

  function resolveCenteredAbsoluteOffset() {
    const index = resolveCenteredLineIndex();
    if (index < 0) {
      return null;
    }
    return computeLyricsOffsetForLine(
      options.lines.value,
      index,
      calibrationPlaybackSeconds.value,
    );
  }

  const {
    dragging,
    dragActivated,
    dragTranslateY,
    handlePointerDown,
    handlePointerMove,
    handlePointerUp,
    handlePointerCancel,
  } = useLyricsDragSync({
    enabled: () => Boolean(options.dragEnabled.value && options.lines.value.length > 0),
    onDragStart: () => {
      frozenScrollIndex.value = resolveActiveIndex(options.activeIndex.value, options.lines.value.length);
      calibrationPlaybackSeconds.value = options.playbackPositionSeconds.value;
    },
    resolveCommitOffset: () => resolveCenteredAbsoluteOffset(),
    resolvePreviewOffset: () => (dragActivated.value ? resolveCenteredAbsoluteOffset() : null),
    resolvePreviewBaseOffset: () => options.baseOffsetSeconds.value,
    onPreviewDelta: options.onDragPreview,
    onCommit: options.onOffsetCommit,
  });

  const scrollIndex = computed(() => {
    if (dragging.value && frozenScrollIndex.value >= 0) {
      return frozenScrollIndex.value;
    }
    return resolveActiveIndex(options.activeIndex.value, options.lines.value.length);
  });

  const { centerTranslateY, scheduleCenterUpdate } = useLyricsCenterRail({
    viewportRef,
    contentRef,
    lineRefs,
    centeringIndex: scrollIndex,
    dragging,
  });

  watch(dragging, (value) => {
    options.onDraggingChange(value);
    if (!value) {
      frozenScrollIndex.value = -1;
      scheduleCenterUpdate();
    }
  });

  const totalTranslateY = computed(() => (
    centerTranslateY.value + (dragging.value ? dragTranslateY.value : 0)
  ));

  const highlightedIndex = computed(() => (
    dragActivated.value ? resolveCenteredLineIndex() : scrollIndex.value
  ));

  const contentStyle = computed(() => ({
    transform: `translate3d(0, ${totalTranslateY.value}px, 0)`,
    transition: dragging.value ? "none" : "transform 280ms cubic-bezier(0.22, 1, 0.36, 1)",
  }));

  const viewportClass = computed(() => [
    "relative",
    "min-h-[8rem]",
    "flex-1",
    "overflow-hidden",
    "touch-none",
    "select-none",
    "pointer-events-auto",
    options.dragEnabled.value ? (dragging.value ? "cursor-grabbing" : "cursor-grab") : "",
  ]);

  function setLineRef(index: number, element: Element | ComponentPublicInstance | null) {
    lineRefs.value[index] = element instanceof HTMLElement ? element : null;
  }

  function blockWindowDrag(event: MouseEvent) {
    event.stopPropagation();
  }

  watch(options.lines, () => scheduleCenterUpdate(), { deep: true, flush: "post" });
  watch(options.activeIndex, () => scheduleCenterUpdate(), { flush: "post" });
  watch(options.baseOffsetSeconds, () => scheduleCenterUpdate());
  watch(viewportHeight, () => scheduleCenterUpdate());
  watch(lineRefs, () => scheduleCenterUpdate(), { deep: true, flush: "post" });

  onBeforeUnmount(() => {
    lineRefs.value = {};
  });

  return {
    ...theme,
    blockWindowDrag,
    contentRef,
    contentStyle,
    dragging,
    handlePointerCancel,
    handlePointerDown,
    handlePointerMove,
    handlePointerUp,
    highlightedIndex,
    listPaddingY,
    setLineRef,
    viewportClass,
    viewportRef,
  };
}

export function useLyricsScrollerProps(props: {
  lines: MediaLyricLine[];
  activeIndex: number;
  playbackPositionSeconds: number;
  dense: boolean;
  transparentOverlay: boolean;
  isDark?: boolean;
  dragEnabled?: boolean;
  baseOffsetSeconds?: number;
}, emit: {
  (event: "drag-preview", deltaSeconds: number): void;
  (event: "offset-commit", nextOffsetSeconds: number): void;
  (event: "dragging-change", dragging: boolean): void;
}) {
  return useLyricsScroller({
    lines: toRef(props, "lines"),
    activeIndex: toRef(props, "activeIndex"),
    playbackPositionSeconds: toRef(props, "playbackPositionSeconds"),
    dense: toRef(props, "dense"),
    transparentOverlay: toRef(props, "transparentOverlay"),
    isDark: toRef(props, "isDark"),
    dragEnabled: computed(() => Boolean(props.dragEnabled)),
    baseOffsetSeconds: computed(() => props.baseOffsetSeconds ?? 0),
    onDragPreview: (deltaSeconds) => emit("drag-preview", deltaSeconds),
    onOffsetCommit: (nextOffsetSeconds) => emit("offset-commit", nextOffsetSeconds),
    onDraggingChange: (dragging) => emit("dragging-change", dragging),
  });
}
