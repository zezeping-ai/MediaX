import { computed, ref, watch, type Ref } from "vue";

interface UseHomePlaybackActionsOptions {
  currentSource: Ref<string>;
  errorMessage: Ref<string>;
  urlInputValue: Ref<string>;
  urlDialogVisible: Ref<boolean>;
  openUrl: (url: string) => Promise<void>;
  handleTransportPlay: () => Promise<void>;
}

export function useHomePlaybackActions(options: UseHomePlaybackActionsOptions) {
  const playerErrorMessage = ref("");

  async function handlePlay() {
    await options.handleTransportPlay();
    playerErrorMessage.value = "";
  }

  async function handlePlayFromUrlPlaylist(url: string) {
    options.urlInputValue.value = url;
    await options.openUrl(url);
    options.urlDialogVisible.value = false;
  }

  watch(options.currentSource, () => {
    playerErrorMessage.value = "";
  });

  return {
    displayErrorMessage: computed(
      () => playerErrorMessage.value || options.errorMessage.value,
    ),
    handlePlay,
    handlePlayFromUrlPlaylist,
    playerErrorMessage,
  };
}
