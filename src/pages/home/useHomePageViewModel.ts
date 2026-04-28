import { useHomePlaybackController } from "./composables/useHomePlaybackController";

export function useHomePageViewModel() {
  const controller = useHomePlaybackController();

  async function handleVideoEnded() {
    await controller.stop();
  }

  return {
    handleVideoEnded,
    ...controller,
  };
}
