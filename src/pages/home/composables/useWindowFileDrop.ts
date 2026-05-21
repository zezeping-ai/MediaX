import { onBeforeUnmount, onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { resolveDialogPath } from "@/modules/resolve-dialog-path";

type UseWindowFileDropOptions = {
  openPath: (path: string) => Promise<void>;
};

export function useWindowFileDrop(options: UseWindowFileDropOptions) {
  const dropActive = ref(false);
  let dragDepth = 0;
  let unlisten: (() => void) | null = null;

  onMounted(async () => {
    unlisten = await getCurrentWindow().onDragDropEvent(async (event) => {
      if (event.payload.type === "enter") {
        dragDepth += 1;
        dropActive.value = true;
        return;
      }

      if (event.payload.type === "over") {
        dropActive.value = true;
        return;
      }

      if (event.payload.type === "leave") {
        dragDepth = Math.max(0, dragDepth - 1);
        dropActive.value = dragDepth > 0;
        return;
      }

      dragDepth = 0;
      dropActive.value = false;
      const path = resolveDialogPath(event.payload.paths);
      if (!path) {
        return;
      }
      await options.openPath(path);
    });
  });

  onBeforeUnmount(() => {
    dragDepth = 0;
    dropActive.value = false;
    unlisten?.();
    unlisten = null;
  });

  return {
    dropActive,
  };
}
