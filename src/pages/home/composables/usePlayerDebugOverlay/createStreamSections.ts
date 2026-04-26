import type { DebugRow, DebugSection } from "./types";
import { pushSnapshotRow } from "./utils";

export function createStreamSections(snapshot: Record<string, string>): DebugSection[] {
  const inputRows: DebugRow[] = [];
  const videoRows: DebugRow[] = [];
  const audioRows: DebugRow[] = [];

  pushSnapshotRow(inputRows, snapshot, "open", "source");
  pushSnapshotRow(inputRows, snapshot, "video_demux", "demux");
  pushSnapshotRow(inputRows, snapshot, "video_gop", "gop");

  pushSnapshotRow(videoRows, snapshot, "video_format", "container");
  pushSnapshotRow(videoRows, snapshot, "video_codec_profile", "codec");
  pushSnapshotRow(videoRows, snapshot, "video_stream", "stream");
  pushSnapshotRow(videoRows, snapshot, "video_frame_format", "frame fmt");
  pushSnapshotRow(videoRows, snapshot, "decoder_ready", "decoder");
  pushSnapshotRow(videoRows, snapshot, "color_profile", "color");

  pushSnapshotRow(audioRows, snapshot, "audio", "stream");
  pushSnapshotRow(audioRows, snapshot, "audio_format", "format");
  pushSnapshotRow(audioRows, snapshot, "audio_pipeline_ready", "pipeline");
  pushSnapshotRow(audioRows, snapshot, "audio_output", "output");

  return [
    { id: "input", title: "输入与探测", rows: inputRows },
    { id: "video-chain", title: "视频链路", rows: videoRows },
    { id: "audio-chain", title: "音频链路", rows: audioRows },
  ].filter((section) => section.rows.length > 0);
}
