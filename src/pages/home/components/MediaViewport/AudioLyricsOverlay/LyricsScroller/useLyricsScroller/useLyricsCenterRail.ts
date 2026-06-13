import { nextTick, onBeforeUnmount, ref, watch, type Ref } from "vue";

type UseLyricsCenterRailOptions = {
  viewportRef: Ref<HTMLElement | null>;
  contentRef: Ref<HTMLElement | null>;
  lineRefs: Ref<Record<number, HTMLElement | null>>;
  centeringIndex: Ref<number>;
  dragging: Ref<boolean>;
};

/** 行中心在 content 容器内的本地 Y（与当前 transform 无关） */
export function measureLineCenterLocalY(contentEl: HTMLElement, lineEl: HTMLElement) {
  const contentTop = contentEl.getBoundingClientRect().top;
  const lineRect = lineEl.getBoundingClientRect();
  return lineRect.top + lineRect.height / 2 - contentTop;
}

/** 找出最接近面板垂直中线的那一行 */
export function findLineIndexAtViewportCenter(options: {
  viewportEl: HTMLElement | null;
  lineCount: number;
  lineRefs: Record<number, HTMLElement | null>;
}) {
  if (!options.viewportEl || options.lineCount <= 0) {
    return -1;
  }
  const viewportRect = options.viewportEl.getBoundingClientRect();
  const viewportCenterY = viewportRect.top + viewportRect.height / 2;
  let bestIndex = 0;
  let bestDistance = Number.POSITIVE_INFINITY;
  for (let index = 0; index < options.lineCount; index += 1) {
    const lineEl = options.lineRefs[index];
    if (!lineEl) {
      continue;
    }
    const lineRect = lineEl.getBoundingClientRect();
    const lineCenterY = lineRect.top + lineRect.height / 2;
    const distance = Math.abs(lineCenterY - viewportCenterY);
    if (distance < bestDistance) {
      bestDistance = distance;
      bestIndex = index;
    }
  }
  return bestIndex;
}

export function useLyricsCenterRail(options: UseLyricsCenterRailOptions) {
  const centerTranslateY = ref(0);
  let frameId = 0;
  let resizeObserver: ResizeObserver | null = null;

  async function updateCenterTranslate() {
    if (options.dragging.value) {
      return;
    }
    await nextTick();
    await new Promise<void>((resolve) => {
      requestAnimationFrame(() => resolve());
    });

    const viewport = options.viewportRef.value;
    const content = options.contentRef.value;
    const index = options.centeringIndex.value;
    const lineEl = options.lineRefs.value[index];
    if (!viewport || !content || index < 0 || !lineEl) {
      if (index < 0 || !viewport) {
        centerTranslateY.value = 0;
      }
      return;
    }

    const lineCenterLocalY = measureLineCenterLocalY(content, lineEl);
    centerTranslateY.value = viewport.clientHeight / 2 - lineCenterLocalY;
  }

  function scheduleCenterUpdate() {
    cancelAnimationFrame(frameId);
    frameId = requestAnimationFrame(() => {
      void updateCenterTranslate();
    });
  }

  function bindResizeObserver() {
    resizeObserver?.disconnect();
    const viewport = options.viewportRef.value;
    if (!viewport || typeof ResizeObserver === "undefined") {
      return;
    }
    resizeObserver = new ResizeObserver(() => {
      scheduleCenterUpdate();
    });
    resizeObserver.observe(viewport);
  }

  watch(
    [options.centeringIndex, options.dragging, options.lineRefs],
    () => {
      scheduleCenterUpdate();
      bindResizeObserver();
    },
    { immediate: true, deep: true, flush: "post" },
  );

  onBeforeUnmount(() => {
    cancelAnimationFrame(frameId);
    resizeObserver?.disconnect();
    resizeObserver = null;
  });

  return {
    centerTranslateY,
    scheduleCenterUpdate,
  };
}
