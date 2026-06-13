import { onBeforeUnmount, ref } from "vue";

const DRAG_ACTIVATION_PX = 6;

type UseLyricsDragSyncOptions = {
  enabled: () => boolean;
  onPreviewDelta: (deltaSeconds: number) => void;
  onCommit: (nextOffsetSeconds: number) => void;
  resolveCommitOffset: () => number | null;
  resolvePreviewOffset?: () => number | null;
  resolvePreviewBaseOffset?: () => number;
  onDragStart?: () => void;
  onActivate?: () => void;
};

export function useLyricsDragSync(options: UseLyricsDragSyncOptions) {
  const dragging = ref(false);
  const dragActivated = ref(false);
  const dragTranslateY = ref(0);

  let pointerId: number | null = null;
  let startClientY = 0;
  let activated = false;
  let captureElement: HTMLElement | null = null;

  function resetDragState() {
    dragging.value = false;
    dragActivated.value = false;
    dragTranslateY.value = 0;
    pointerId = null;
    activated = false;
    captureElement = null;
    options.onPreviewDelta(0);
  }

  function emitPreviewDelta() {
    const absoluteOffset = options.resolvePreviewOffset?.() ?? null;
    if (absoluteOffset === null || !Number.isFinite(absoluteOffset)) {
      options.onPreviewDelta(0);
      return;
    }
    const baseOffset = options.resolvePreviewBaseOffset?.() ?? 0;
    options.onPreviewDelta(absoluteOffset - baseOffset);
  }

  function commitDrag() {
    if (!activated) {
      resetDragState();
      return;
    }
    const nextOffset = options.resolveCommitOffset();
    if (nextOffset === null || !Number.isFinite(nextOffset)) {
      resetDragState();
      return;
    }
    options.onCommit(nextOffset);
    resetDragState();
  }

  function handlePointerDown(event: PointerEvent, captureTarget?: HTMLElement | null) {
    if (!options.enabled() || event.button !== 0) {
      return;
    }
    event.stopPropagation();
    event.preventDefault();
    pointerId = event.pointerId;
    startClientY = event.clientY;
    activated = false;
    dragging.value = true;
    dragActivated.value = false;
    dragTranslateY.value = 0;
    options.onDragStart?.();
    captureElement = captureTarget ?? (event.currentTarget as HTMLElement | null);
    captureElement?.setPointerCapture(event.pointerId);
  }

  function handlePointerMove(event: PointerEvent) {
    if (!dragging.value || pointerId !== event.pointerId) {
      return;
    }
    const deltaY = event.clientY - startClientY;
    if (!activated && Math.abs(deltaY) < DRAG_ACTIVATION_PX) {
      return;
    }
    if (!activated) {
      activated = true;
      dragActivated.value = true;
      options.onActivate?.();
    }
    event.preventDefault();
    dragTranslateY.value = deltaY;
    emitPreviewDelta();
  }

  function handlePointerUp(event: PointerEvent) {
    if (!dragging.value || pointerId !== event.pointerId) {
      return;
    }
    captureElement?.releasePointerCapture(event.pointerId);
    commitDrag();
  }

  function handlePointerCancel(event: PointerEvent) {
    if (!dragging.value || pointerId !== event.pointerId) {
      return;
    }
    captureElement?.releasePointerCapture(event.pointerId);
    resetDragState();
  }

  onBeforeUnmount(() => {
    resetDragState();
  });

  return {
    dragging,
    dragActivated,
    dragTranslateY,
    handlePointerDown,
    handlePointerMove,
    handlePointerUp,
    handlePointerCancel,
  };
}
