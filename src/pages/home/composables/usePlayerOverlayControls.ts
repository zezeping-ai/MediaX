import { computed, onBeforeUnmount, ref, watch, type Ref } from "vue";

type UsePlayerOverlayControlsOptions = {
  hasSource: Ref<boolean>;
  isBusy: Ref<boolean>;
};

export function usePlayerOverlayControls(options: UsePlayerOverlayControlsOptions) {
  const controlsVisible = ref(true);
  const controlsLocked = ref(false);
  const controlsHovered = ref(false);
  let hideTimer: number | null = null;

  const shouldKeepVisible = computed(
    () => controlsLocked.value || controlsHovered.value || !options.hasSource.value || options.isBusy.value,
  );

  function showControls() {
    controlsVisible.value = true;
  }

  function clearHideTimer() {
    if (hideTimer === null) {
      return;
    }
    window.clearTimeout(hideTimer);
    hideTimer = null;
  }

  function scheduleHideControls() {
    clearHideTimer();
    if (shouldKeepVisible.value) {
      showControls();
      return;
    }
    hideTimer = window.setTimeout(() => {
      controlsVisible.value = false;
    }, 1800);
  }

  function markMouseActive() {
    showControls();
    scheduleHideControls();
  }

  function toggleLock() {
    controlsLocked.value = !controlsLocked.value;
    if (controlsLocked.value) {
      clearHideTimer();
      showControls();
      return;
    }
    scheduleHideControls();
  }

  function onControlsMouseEnter() {
    controlsHovered.value = true;
    clearHideTimer();
    showControls();
  }

  function onControlsMouseLeave() {
    controlsHovered.value = false;
    scheduleHideControls();
  }

  watch(shouldKeepVisible, (keepVisible) => {
    if (keepVisible) {
      clearHideTimer();
      showControls();
      return;
    }
    scheduleHideControls();
  });

  onBeforeUnmount(() => {
    clearHideTimer();
  });

  return {
    controlsVisible,
    controlsLocked,
    scheduleHideControls,
    markMouseActive,
    toggleLock,
    onControlsMouseEnter,
    onControlsMouseLeave,
  };
}
