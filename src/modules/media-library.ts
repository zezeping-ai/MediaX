import { invoke } from "@tauri-apps/api/core";
import type { MediaSnapshot } from "./media-types";

export function setMediaLibraryRoots(roots: string[]) {
  return invoke<MediaSnapshot>("media_set_library_roots", { roots });
}

export function rescanMediaLibrary() {
  return invoke<MediaSnapshot>("media_rescan_library");
}
