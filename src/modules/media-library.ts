import { invokeMediaCommandValidated } from "./media-command";
import { isMediaSnapshot, type MediaSnapshot } from "./media-types";

export function setMediaLibraryRoots(roots: string[]) {
  return invokeMediaCommandValidated<MediaSnapshot>("media_set_library_roots", isMediaSnapshot, { roots });
}

export function rescanMediaLibrary() {
  return invokeMediaCommandValidated<MediaSnapshot>("media_rescan_library", isMediaSnapshot);
}
