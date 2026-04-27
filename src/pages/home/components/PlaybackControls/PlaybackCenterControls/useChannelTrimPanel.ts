import { onBeforeUnmount, onMounted, ref, watch, type Ref } from "vue";

type UseChannelTrimPanelOptions = {
  rootRef: Ref<HTMLElement | null>;
  leftChannelVolume: Ref<number>;
  rightChannelVolume: Ref<number>;
  speedDropdownOpen: Ref<boolean>;
  qualityDropdownOpen: Ref<boolean>;
  emitOverlayInteractionChange: (open: boolean) => void;
};

export function useChannelTrimPanel(options: UseChannelTrimPanelOptions) {
  const channelPanelOpen = ref(false);
  const leftVolumePreview = ref(1);
  const rightVolumePreview = ref(1);

  watch(
    [options.leftChannelVolume, options.rightChannelVolume],
    ([left, right]) => {
      leftVolumePreview.value = left;
      rightVolumePreview.value = right;
    },
    { immediate: true },
  );

  watch(channelPanelOpen, (open) => {
    options.emitOverlayInteractionChange(open);
  });

  watch(
    [options.speedDropdownOpen, options.qualityDropdownOpen],
    ([speedOpen, qualityOpen]) => {
      if (speedOpen || qualityOpen) {
        channelPanelOpen.value = false;
      }
    },
  );

  function toggleChannelPanel() {
    channelPanelOpen.value = !channelPanelOpen.value;
  }

  function handleDocumentPointerDown(event: PointerEvent) {
    if (!channelPanelOpen.value) {
      return;
    }
    const root = options.rootRef.value;
    const target = event.target;
    if (!(target instanceof Node) || !root || root.contains(target)) {
      return;
    }
    channelPanelOpen.value = false;
  }

  onMounted(() => {
    document.addEventListener("pointerdown", handleDocumentPointerDown);
  });

  onBeforeUnmount(() => {
    document.removeEventListener("pointerdown", handleDocumentPointerDown);
    options.emitOverlayInteractionChange(false);
  });

  return {
    channelPanelOpen,
    leftVolumePreview,
    rightVolumePreview,
    toggleChannelPanel,
  };
}
