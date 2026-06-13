export const LOCAL_MEDIA_EXTENSIONS = [
  "mp4", "mkv", "mov", "avi", "webm", "flv", "m4v", "wmv", "mpeg", "mpg", "ts", "m2ts",
  "mts", "mxf", "rm", "rmvb", "3gp", "3g2", "ogv", "asf", "vob", "f4v", "divx",
  "mp3", "flac", "wav", "aac", "m4a", "ogg", "opus", "wma", "aif", "aiff", "ape",
  "alac", "amr", "ac3", "dts", "mp2", "mka",
] as const;

export const LOCAL_MEDIA_DIALOG_FILTERS = [
  {
    name: "媒体文件",
    extensions: [...LOCAL_MEDIA_EXTENSIONS],
  },
];

export function isLocalMediaPath(path: string) {
  const ext = path.trim().split(".").pop()?.toLowerCase() ?? "";
  return LOCAL_MEDIA_EXTENSIONS.includes(ext as (typeof LOCAL_MEDIA_EXTENSIONS)[number]);
}

export function filterLocalMediaPaths(paths: string[]) {
  return paths.filter(isLocalMediaPath);
}
