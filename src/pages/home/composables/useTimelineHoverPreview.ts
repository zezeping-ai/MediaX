import { computed, ref, watch } from "vue";
import { throttle } from "lodash-es";
import type { PreviewFrame } from "@/modules/media-types";

type RequestPreviewFrame = (positionSeconds: number, maxWidth?: number, maxHeight?: number) => Promise<PreviewFrame | null>;

const HOVER_PREVIEW_INTERVAL_MS = 100;
const HOVER_CACHE_LIMIT = 32;
const PREVIEW_WIDTH = 160;
const PREVIEW_HEIGHT = 90;

export function useTimelineHoverPreview(
  durationGetter: () => number,
  sourceKeyGetter: () => string,
  requestPreviewFrame?: RequestPreviewFrame,
) {
  const previewContainerRef = ref<HTMLElement | null>(null);
  const hoverVisible = ref(false);
  const hoverLeft = ref(0);
  const hoverSeconds = ref(0);
  const hoverImageSrc = ref("");
  const hoverImageWidth = ref(PREVIEW_WIDTH);
  const hoverImageHeight = ref(PREVIEW_HEIGHT);
  const hoverPreviewCache = new Map<number, PreviewFrame>();
  let latestRequestToken = 0;

  const canShowPreview = computed(() => hoverVisible.value && Boolean(hoverImageSrc.value));

  function resetPreviewState() {
    hoverVisible.value = false;
    hoverImageSrc.value = "";
    latestRequestToken += 1;
    hoverPreviewCache.clear();
    requestHoverPreviewThrottled.cancel();
  }

  function setHoverCache(secondsBucket: number, frame: PreviewFrame) {
    if (hoverPreviewCache.has(secondsBucket)) {
      hoverPreviewCache.delete(secondsBucket);
    }
    hoverPreviewCache.set(secondsBucket, frame);
    while (hoverPreviewCache.size > HOVER_CACHE_LIMIT) {
      const oldest = hoverPreviewCache.keys().next().value;
      if (typeof oldest !== "number") {
        break;
      }
      hoverPreviewCache.delete(oldest);
    }
  }

  async function fetchPreview(seconds: number) {
    if (!requestPreviewFrame) {
      return;
    }
    const bucket = Math.round(seconds * 2) / 2;
    const cached = hoverPreviewCache.get(bucket);
    if (cached) {
      hoverImageSrc.value = `data:${cached.mime_type};base64,${cached.data_base64}`;
      hoverImageWidth.value = cached.width;
      hoverImageHeight.value = cached.height;
      return;
    }

    const token = ++latestRequestToken;
    const frame = await requestPreviewFrame(seconds, PREVIEW_WIDTH, PREVIEW_HEIGHT);
    if (!frame || token !== latestRequestToken) {
      return;
    }

    setHoverCache(bucket, frame);
    hoverImageSrc.value = `data:${frame.mime_type};base64,${frame.data_base64}`;
    hoverImageWidth.value = frame.width;
    hoverImageHeight.value = frame.height;
  }

  // Leading+trailing keeps hover responsive while preserving last mouse position.
  const requestHoverPreviewThrottled = throttle(
    (seconds: number) => {
      void fetchPreview(seconds);
    },
    HOVER_PREVIEW_INTERVAL_MS,
    { leading: true, trailing: true },
  );

  function onTimelineMouseMove(event: MouseEvent) {
    const el = previewContainerRef.value;
    if (!el) {
      return;
    }
    const rect = el.getBoundingClientRect();
    if (rect.width <= 0) {
      return;
    }

    const x = Math.min(Math.max(event.clientX - rect.left, 0), rect.width);
    const ratio = x / rect.width;
    const duration = Math.max(durationGetter(), 0);
    const seconds = duration > 0 ? duration * ratio : 0;
    hoverLeft.value = x;
    hoverSeconds.value = seconds;
    hoverVisible.value = true;
    requestHoverPreviewThrottled(seconds);
  }

  function onTimelineMouseLeave() {
    hoverVisible.value = false;
    hoverImageSrc.value = "";
    latestRequestToken += 1;
    requestHoverPreviewThrottled.cancel();
  }

  function dispose() {
    resetPreviewState();
  }

  watch(
    sourceKeyGetter,
    () => {
      resetPreviewState();
    },
    { immediate: true },
  );

  return {
    previewContainerRef,
    hoverLeft,
    hoverSeconds,
    hoverImageSrc,
    hoverImageWidth,
    hoverImageHeight,
    canShowPreview,
    onTimelineMouseMove,
    onTimelineMouseLeave,
    dispose,
  };
}
