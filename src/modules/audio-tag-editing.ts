export const TAG_WRITABLE_AUDIO_EXTENSIONS = [
  "mp3", "flac", "m4a", "ogg", "opus", "ape", "wma", "aiff", "alac", "mka", "wav",
] as const;

export function isRemoteMediaPath(path: string) {
  const scheme = path.split(":")[0]?.toLowerCase() ?? "";
  return ["http", "https", "rtsp", "rtmp", "mms"].includes(scheme);
}

export function canEditAudioTags(path: string) {
  const trimmed = path.trim();
  if (!trimmed || isRemoteMediaPath(trimmed)) {
    return false;
  }
  const ext = trimmed.split(".").pop()?.toLowerCase() ?? "";
  return TAG_WRITABLE_AUDIO_EXTENSIONS.includes(ext as (typeof TAG_WRITABLE_AUDIO_EXTENSIONS)[number]);
}
