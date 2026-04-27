export type PlaybackStatus = "idle" | "playing" | "paused" | "stopped";
export type HardwareDecodeMode = "auto" | "on" | "off";
export type PlaybackQualityMode = "source" | "auto" | "1080p" | "720p" | "480p" | "320p";
export type PlaybackMediaKind = "video" | "audio";
export type PlaybackChannelRouting = "stereo" | "left_to_both" | "right_to_both";
export const MEDIA_PLAYBACK_STATE_EVENT = "media://playback/state";
export const MEDIA_PLAYBACK_METADATA_EVENT = "media://playback/metadata";
export const MEDIA_PLAYBACK_ERROR_EVENT = "media://playback/error";
export const MEDIA_PLAYBACK_DEBUG_EVENT = "media://playback/debug";
export const MEDIA_PLAYBACK_TELEMETRY_EVENT = "media://playback/telemetry";
export const MEDIA_PLAYBACK_AUDIO_METER_EVENT = "media://playback/audio-meter";
export const MEDIA_MENU_EVENT = "media://menu-action";

export interface PlaybackState {
  engine: string;
  status: PlaybackStatus;
  media_kind: PlaybackMediaKind;
  current_path: string | null;
  position_seconds: number;
  duration_seconds: number;
  playback_rate: number;
  error: string | null;
  hw_decode_mode: HardwareDecodeMode;
  hw_decode_active: boolean;
  hw_decode_backend: string | null;
  hw_decode_error: string | null;
  quality_mode: PlaybackQualityMode;
  adaptive_quality_supported: boolean;
  volume: number;
  muted: boolean;
  left_channel_volume: number;
  right_channel_volume: number;
  left_channel_muted: boolean;
  right_channel_muted: boolean;
  channel_routing: PlaybackChannelRouting;
}

export interface MediaItem {
  id: string;
  path: string;
  name: string;
  extension: string;
  size_bytes: number;
  last_played_at: number | null;
  last_position_seconds: number;
}

export interface MediaLibraryState {
  roots: string[];
  items: MediaItem[];
}

export interface MediaSnapshot {
  playback: PlaybackState;
  library: MediaLibraryState;
}

export interface PreviewFrame {
  mime_type: string;
  data_base64: string;
  width: number;
  height: number;
  position_seconds: number;
}

export interface MediaLyricLine {
  time_seconds: number;
  text: string;
}

export interface MediaAudioMeterPayload {
  sample_rate: number;
  channels: number;
  left_peak: number;
  right_peak: number;
  left_spectrum: number[];
  right_spectrum: number[];
}

export interface CacheRecordingStatus {
  recording: boolean;
  source: string | null;
  output_path: string | null;
  finalized_output_path: string | null;
  output_size_bytes?: number | null;
  started_at_ms: number | null;
  error_message?: string | null;
  fallback_transcoding?: boolean | null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function isMediaSnapshot(value: unknown): value is MediaSnapshot {
  if (!isRecord(value)) {
    return false;
  }
  const playback = value.playback;
  const library = value.library;
  if (!isRecord(playback) || !isRecord(library)) {
    return false;
  }
  return (
    typeof playback.engine === "string" &&
    typeof playback.status === "string" &&
    typeof playback.position_seconds === "number" &&
    typeof playback.duration_seconds === "number" &&
    Array.isArray(library.roots) &&
    Array.isArray(library.items)
  );
}

export function isPreviewFrame(value: unknown): value is PreviewFrame {
  if (!isRecord(value)) {
    return false;
  }
  return (
    typeof value.mime_type === "string" &&
    typeof value.data_base64 === "string" &&
    typeof value.width === "number" &&
    typeof value.height === "number" &&
    typeof value.position_seconds === "number"
  );
}

export interface MediaEventEnvelope<T> {
  protocol_version: number;
  event_type: string;
  request_id: string | null;
  emitted_at_ms: number;
  payload: T;
}

export interface MediaVideoTimestampStats {
  samples: number;
  pts_missing_ratio_percent: number;
  pts_backtrack_count: number;
  jitter_avg_ms: number;
  jitter_max_ms: number;
}

export interface MediaFrameTypeStats {
  sample_count: number;
  i_ratio_percent: number;
  p_ratio_percent: number;
  b_ratio_percent: number;
  other_ratio_percent: number;
}

export interface MediaDecodeQuantileStats {
  sample_count: number;
  avg_ms: number;
  max_ms: number;
  p50_ms: number;
  p95_ms: number;
  p99_ms: number;
}

export interface MediaVideoStageCostStats {
  sample_count: number;
  receive_avg_ms: number;
  receive_max_ms: number;
  hw_transfer_avg_ms: number;
  hw_transfer_max_ms: number;
  scale_avg_ms: number;
  scale_max_ms: number;
  submit_avg_ms: number;
  submit_max_ms: number;
  total_avg_ms: number;
  total_max_ms: number;
}

export interface MediaTelemetryPayload {
  source_fps: number;
  render_fps: number;
  queue_depth: number;
  audio_queue_depth_sources?: number | null;
  clock_seconds: number;
  current_video_pts_seconds?: number | null;
  current_presented_video_pts_seconds?: number | null;
  current_submitted_video_pts_seconds?: number | null;
  current_audio_clock_seconds?: number | null;
  current_frame_type?: string | null;
  current_frame_width?: number | null;
  current_frame_height?: number | null;
  playback_rate?: number | null;
  network_read_bytes_per_second?: number | null;
  media_required_bytes_per_second?: number | null;
  network_sustain_ratio?: number | null;
  audio_drift_seconds: number | null;
  video_pts_gap_seconds: number | null;
  seek_settle_ms: number | null;
  decode_avg_frame_cost_ms: number | null;
  decode_max_frame_cost_ms: number | null;
  decode_samples: number | null;
  decode_quantiles?: MediaDecodeQuantileStats | null;
  video_stage_costs?: MediaVideoStageCostStats | null;
  video_timestamps?: MediaVideoTimestampStats | null;
  frame_types?: MediaFrameTypeStats | null;
  process_cpu_percent: number | null;
  process_memory_mb: number | null;
  gpu_queue_depth: number | null;
  gpu_queue_capacity: number | null;
  gpu_queue_utilization: number | null;
  render_estimated_cost_ms: number | null;
  render_present_lag_ms: number | null;
  video_submit_lead_ms?: number | null;
  video_packet_soft_error_count?: number | null;
  video_frame_drop_count?: number | null;
  video_hw_transfer_drop_count?: number | null;
  video_nv12_drop_count?: number | null;
  video_scale_drop_count?: number | null;
}

export interface MediaMetadataPayload {
  media_kind: PlaybackMediaKind;
  width: number;
  height: number;
  fps: number;
  duration_seconds: number;
  title?: string | null;
  artist?: string | null;
  album?: string | null;
  has_cover_art?: boolean;
  lyrics?: MediaLyricLine[];
}

export interface MediaErrorPayload {
  code: string;
  message: string;
}

export interface MediaDebugPayload {
  stage: string;
  message: string;
  at_ms: number;
}
