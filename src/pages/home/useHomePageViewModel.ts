import { useHomePlaybackController } from "./composables/useHomePlaybackController";

export function useHomePageViewModel() {
  const controller = useHomePlaybackController();

  async function handleVideoEnded() {
    await controller.handleTrackEnded();
  }

  return {
    handleVideoEnded,
    ...controller,
  };
}
