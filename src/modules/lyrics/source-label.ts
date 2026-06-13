export function formatLyricsSourceLabel(source: string | null) {
  switch (source) {
    case "lrclib":
      return "LRCLIB";
    case "lrcapi":
    case "lrcapi_jsonapi":
      return "LrcApi";
    case "kugou":
      return "酷狗音乐";
    case "netease":
      return "网易云音乐";
    case "cache":
      return "Cache";
    case "sidecar":
      return "Local LRC";
    case "embedded":
      return "Embedded";
    default:
      return "";
  }
}
