import { invokeMediaCommand } from "../media-command";
import type { PlayerVideoScaleMode } from "../preferences";

export function setMainWindowAlwaysOnTop(enabled: boolean) {
  return invokeMediaCommand<void>("window_set_main_always_on_top", { enabled });
}

export function setMainWindowVideoScaleMode(mode: PlayerVideoScaleMode) {
  return invokeMediaCommand<void>("window_set_main_video_scale_mode", { mode });
}

export function toggleMainWindowFullscreen() {
  return invokeMediaCommand<boolean>("window_toggle_main_fullscreen");
}

export function startMainWindowDragging() {
  return invokeMediaCommand<void>("window_start_main_dragging");
}
