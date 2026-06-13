import { invokeMediaCommand } from "../media-command";
import type { PlayerVideoScaleMode } from "../preferences";

export function setMainWindowAlwaysOnTop(enabled: boolean) {
  return invokeMediaCommand<void>("window_set_main_always_on_top", { enabled });
}

export function setMainWindowTitle(title?: string | null) {
  return invokeMediaCommand<void>("window_set_main_title", { title: title ?? null });
}

export function setMainWindowVideoScaleMode(mode: PlayerVideoScaleMode) {
  return invokeMediaCommand<void>("window_set_main_video_scale_mode", { mode });
}

export function setRendererBackdropTheme(theme: "light" | "dark") {
  return invokeMediaCommand<void>("window_set_renderer_backdrop_theme", { theme });
}

export function toggleMainWindowFullscreen() {
  return invokeMediaCommand<boolean>("window_toggle_main_fullscreen");
}

export function startMainWindowDragging() {
  return invokeMediaCommand<void>("window_start_main_dragging");
}
