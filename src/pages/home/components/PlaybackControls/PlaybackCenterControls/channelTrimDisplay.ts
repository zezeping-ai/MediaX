import type { PlaybackChannelRouting } from "@/modules/media-types";

export function formatPercent(value: number) {
  const normalized = Number.isFinite(value) ? Math.max(0, Math.min(1, value)) : 0;
  return `${Math.round(normalized * 100)}%`;
}

export function channelRoutingLabel(routing: PlaybackChannelRouting) {
  switch (routing) {
    case "left_to_both":
      return "L->LR";
    case "right_to_both":
      return "R->LR";
    default:
      return "Stereo";
  }
}

export function channelStateLabel(muted: boolean) {
  return muted ? "Muted" : "Live";
}
