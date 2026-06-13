import { invokeMediaCommandValidated } from "./media-command";
import { isMediaSnapshot, type MediaSnapshot } from "./media-types";

export function setMediaLibraryRoots(roots: string[]) {
  return invokeMediaCommandValidated<MediaSnapshot>("media_set_library_roots", isMediaSnapshot, { roots });
}

export function rescanMediaLibrary() {
  return invokeMediaCommandValidated<MediaSnapshot>("media_rescan_library", isMediaSnapshot);
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}

export function scanMediaDirectory(directory: string) {
  return invokeMediaCommandValidated<string[]>("media_scan_directory", isStringArray, { directory });
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

export function getSavedPlaybackPosition(path: string) {
  return invokeMediaCommandValidated<number>("media_saved_playback_position", isFiniteNumber, { path });
}
