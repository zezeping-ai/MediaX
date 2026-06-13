import { computed, ref, type Ref } from "vue";
import { useLyricsTrackPreferences } from "@/modules/lyrics";
import { usePreferences } from "@/modules/preferences";

type UseLyricsOverlayControlsOptions = {
  currentSourcePath: Readonly<Ref<string>>;
};

/** 歌词面板：可见性、每曲偏移与拖拽预览 */
export function useLyricsOverlayControls(options: UseLyricsOverlayControlsOptions) {
  const { playerShowLyrics } = usePreferences();
  const {
    hidden: trackLyricsHidden,
    offsetSeconds: trackLyricsOffsetSeconds,
    adjustOffset,
    resetOffset,
    fineOffsetStepSeconds,
  } = useLyricsTrackPreferences(options.currentSourcePath);

  const dragOffsetPreview = ref(0);
  const lyricsDragging = ref(false);

  const lyricsVisible = computed({
    get: () => playerShowLyrics.value && !trackLyricsHidden.value,
    set: (value: boolean) => {
      trackLyricsHidden.value = !value;
    },
  });

  const displayedOffsetSeconds = computed(
    () => trackLyricsOffsetSeconds.value + dragOffsetPreview.value,
  );

  function toggleLyricsVisible() {
    lyricsVisible.value = !lyricsVisible.value;
  }

  function handleDragPreview(deltaSeconds: number) {
    dragOffsetPreview.value = deltaSeconds;
  }

  function handleDraggingChange(dragging: boolean) {
    lyricsDragging.value = dragging;
    if (!dragging) {
      dragOffsetPreview.value = 0;
    }
  }

  function handleOffsetCommit(nextOffsetSeconds: number) {
    trackLyricsOffsetSeconds.value = nextOffsetSeconds;
    dragOffsetPreview.value = 0;
    lyricsDragging.value = false;
  }

  return {
    adjustOffset,
    displayedOffsetSeconds,
    fineOffsetStepSeconds,
    handleDragPreview,
    handleDraggingChange,
    handleOffsetCommit,
    lyricsDragging,
    lyricsVisible,
    playerShowLyrics,
    resetOffset,
    toggleLyricsVisible,
    trackLyricsOffsetSeconds,
  };
}
