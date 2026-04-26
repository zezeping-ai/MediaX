export const STATIC_DEBUG_KEYS = [
  "open",
  "decoder_ready",
  "video_stream",
  "audio",
  "video_format",
  "video_codec_profile",
  "audio_format",
  "video_frame_format",
] as const;

export function formatMediaInfoLabel(key: string): string {
  switch (key) {
    case "source":
      return "来源";
    case "video_format":
      return "视频格式";
    case "video_codec_profile":
      return "编码配置";
    case "audio_format":
      return "音频格式";
    case "video_frame_format":
      return "帧格式/色彩";
    case "video_stream":
      return "视频流参数";
    case "audio":
      return "音频流";
    case "engine":
      return "引擎";
    case "duration":
      return "时长";
    case "resolution":
      return "分辨率";
    case "fps":
      return "帧率";
    case "quality":
      return "画质";
    default:
      return key;
  }
}
