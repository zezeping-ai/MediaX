export type PlaylistItemKind = "local" | "url";

/** 曲目结束后的自动续播策略 */
export type PlaybackAdvanceMode = "sequential" | "shuffle" | "repeat-one" | "stop-after-current";

export const PLAYBACK_ADVANCE_MODE_OPTIONS: ReadonlyArray<{
  value: PlaybackAdvanceMode;
  label: string;
  icon: string;
  title: string;
}> = [
  { value: "sequential", label: "顺序", icon: "lucide:list-ordered", title: "按列表顺序播放下一首" },
  { value: "shuffle", label: "随机", icon: "lucide:shuffle", title: "随机播放列表中的下一首" },
  { value: "repeat-one", label: "单曲", icon: "lucide:repeat-1", title: "单曲循环" },
  {
    value: "stop-after-current",
    label: "停止",
    icon: "lucide:square",
    title: "播放完当前曲目后自动停止",
  },
];

export type PlaylistItem = {
  id: string;
  source: string;
  kind: PlaylistItemKind;
  title: string;
  addedAt: number;
  lastPlayedAt: number | null;
};

export type UrlPlaylistItem = {
  url: string;
  lastOpenedAt: number;
};
