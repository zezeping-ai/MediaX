import {
  startMainWindowDragging,
  toggleMainWindowFullscreen,
} from "@/modules/media-player";

export function useViewportWindowController() {
  async function requestToggleFullscreen() {
    await toggleMainWindowFullscreen();
  }

  async function requestStartWindowDrag() {
    await startMainWindowDragging();
  }

  return {
    requestStartWindowDrag,
    requestToggleFullscreen,
  };
}
