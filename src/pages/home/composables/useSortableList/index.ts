import Sortable from "sortablejs";
import { nextTick, onBeforeUnmount, watch, type Ref } from "vue";

type UseSortableListOptions = {
  containerRef: Ref<HTMLElement | null>;
  scrollRef?: Ref<HTMLElement | null>;
  enabled?: Ref<boolean>;
  handle?: string;
  draggable?: string;
  getItemIds?: () => string[];
  onDragStateChange?: (dragging: boolean) => void;
  onReorder: (oldIndex: number, newIndex: number) => void;
};

function resolveReorderIndices(
  event: Sortable.SortableEvent,
  container: HTMLElement,
  itemIds: string[],
): { oldIndex: number; newIndex: number } | null {
  if (
    event.oldIndex != null
    && event.newIndex != null
    && event.oldIndex !== event.newIndex
  ) {
    return { oldIndex: event.oldIndex, newIndex: event.newIndex };
  }

  const draggedId = event.item.getAttribute("data-id") ?? "";
  if (!draggedId) {
    return null;
  }

  const domIds = [...container.querySelectorAll("[data-sortable-item]")].map(
    (element) => element.getAttribute("data-id") ?? "",
  );
  const oldIndex = itemIds.indexOf(draggedId);
  const newIndex = domIds.indexOf(draggedId);
  if (oldIndex < 0 || newIndex < 0 || oldIndex === newIndex) {
    return null;
  }

  return { oldIndex, newIndex };
}

export function useSortableList(options: UseSortableListOptions) {
  let sortable: Sortable | null = null;
  let dragging = false;

  function destroySortable() {
    sortable?.destroy();
    sortable = null;
  }

  async function mountSortable() {
    if (dragging) {
      return;
    }
    destroySortable();
    await nextTick();
    const container = options.containerRef.value;
    if (!container || options.enabled?.value === false) {
      return;
    }
    const scrollElement = options.scrollRef?.value ?? container;
    sortable = Sortable.create(container, {
      animation: 150,
      direction: "vertical",
      handle: options.handle,
      draggable: options.draggable ?? "[data-sortable-item]",
      forceFallback: true,
      fallbackOnBody: true,
      fallbackTolerance: 3,
      swapThreshold: 0.5,
      invertSwap: true,
      scroll: scrollElement,
      bubbleScroll: true,
      scrollSensitivity: 60,
      scrollSpeed: 12,
      fallbackClass: "playlist-sortable-fallback",
      ghostClass: "playlist-sortable-ghost",
      chosenClass: "playlist-sortable-chosen",
      dragClass: "playlist-sortable-drag",
      onStart: () => {
        dragging = true;
        options.onDragStateChange?.(true);
      },
      onEnd: (event) => {
        dragging = false;
        options.onDragStateChange?.(false);
        const indices = resolveReorderIndices(
          event,
          container,
          options.getItemIds?.() ?? [],
        );
        if (!indices) {
          return;
        }
        options.onReorder(indices.oldIndex, indices.newIndex);
      },
    });
  }

  watch(
    () => [options.containerRef.value, options.scrollRef?.value, options.enabled?.value] as const,
    () => {
      void mountSortable();
    },
    { flush: "post", immediate: true },
  );

  onBeforeUnmount(() => {
    destroySortable();
  });

  return {
    remount: mountSortable,
  };
}
