import { onBeforeUnmount, ref } from "vue";
import { onClickOutside } from "@vueuse/core";

export function useLyricsSelectDropdown() {
  const open = ref(false);
  const rootRef = ref<HTMLElement | null>(null);

  const dismiss = onClickOutside(rootRef, () => {
    open.value = false;
  });

  onBeforeUnmount(() => {
    dismiss();
  });

  function toggleOpen() {
    open.value = !open.value;
  }

  function close() {
    open.value = false;
  }

  return {
    close,
    open,
    rootRef,
    toggleOpen,
  };
}
