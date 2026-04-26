import { defineAsyncComponent } from "vue";
import { useHomePlaybackController } from "./composables/useHomePlaybackController";

const PlaybackControls = defineAsyncComponent({
  loader: () => import("./components/PlaybackControls/index.vue"),
  delay: 120,
});

const OpenUrlModal = defineAsyncComponent({
  loader: () => import("./components/OpenUrlModal/index.vue"),
  delay: 120,
});

export function useHomePageViewModel() {
  const controller = useHomePlaybackController();

  async function handleVideoEnded() {
    await controller.stop();
  }

  return {
    PlaybackControls,
    OpenUrlModal,
    handleVideoEnded,
    ...controller,
  };
}
