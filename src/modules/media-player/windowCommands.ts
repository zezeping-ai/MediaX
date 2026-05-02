import { invokeMediaCommand } from "../media-command";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { PlayerVideoScaleMode } from "../preferences";

export function setMainWindowAlwaysOnTop(enabled: boolean) {
  return invokeMediaCommand<void>("window_set_main_always_on_top", { enabled });
}

export function setMainWindowVideoScaleMode(mode: PlayerVideoScaleMode) {
  return invokeMediaCommand<void>("window_set_main_video_scale_mode", { mode });
}

export async function toggleMainWindowFullscreen() {
  const window = getCurrentWindow();
  const isFullscreen = await window.isFullscreen();
  await window.setFullscreen(!isFullscreen);
}

export async function startMainWindowDragging() {
  await getCurrentWindow().startDragging();
}
