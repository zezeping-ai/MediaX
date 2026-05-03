import { invokeMediaCommand } from "../media-command";
import type { PlayerVideoScaleMode } from "../preferences";

export interface MediaWindowControl {
  setAlwaysOnTop: (enabled: boolean) => Promise<void>;
  setVideoScaleMode: (mode: PlayerVideoScaleMode) => Promise<void>;
  toggleFullscreen: () => Promise<boolean>;
  startDragging: () => Promise<void>;
}

function hasTauriWindowRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function createNoopWindowControl(): MediaWindowControl {
  return {
    setAlwaysOnTop: async () => {},
    setVideoScaleMode: async () => {},
    toggleFullscreen: async () => false,
    startDragging: async () => {},
  };
}

function createTauriWindowControl(): MediaWindowControl {
  return {
    setAlwaysOnTop: async (enabled) => {
      await invokeMediaCommand<void>("window_set_main_always_on_top", { enabled });
    },
    setVideoScaleMode: async (mode) => {
      await invokeMediaCommand<void>("window_set_main_video_scale_mode", { mode });
    },
    toggleFullscreen: () => invokeMediaCommand<boolean>("window_toggle_main_fullscreen"),
    startDragging: () => invokeMediaCommand<void>("window_start_main_dragging"),
  };
}

export const mediaWindowControl = hasTauriWindowRuntime()
  ? createTauriWindowControl()
  : createNoopWindowControl();

export function setMainWindowAlwaysOnTop(enabled: boolean) {
  return mediaWindowControl.setAlwaysOnTop(enabled);
}

export function setMainWindowVideoScaleMode(mode: PlayerVideoScaleMode) {
  return mediaWindowControl.setVideoScaleMode(mode);
}

export function toggleMainWindowFullscreen() {
  return mediaWindowControl.toggleFullscreen();
}

export function startMainWindowDragging() {
  return mediaWindowControl.startDragging();
}
