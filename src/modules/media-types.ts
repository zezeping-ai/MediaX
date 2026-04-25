export type PlaybackStatus = "idle" | "playing" | "paused" | "stopped";
export type HardwareDecodeMode = "auto" | "on" | "off";
export const MEDIA_PLAYBACK_STATE_EVENT = "media://playback/state";
export const MEDIA_PLAYBACK_METADATA_EVENT = "media://playback/metadata";
export const MEDIA_PLAYBACK_ERROR_EVENT = "media://playback/error";
export const MEDIA_PLAYBACK_DEBUG_EVENT = "media://playback/debug";
export const MEDIA_PLAYBACK_TELEMETRY_EVENT = "media://playback/telemetry";
export const MEDIA_STATE_EVENT = "media://state";
export const MEDIA_STATE_EVENT_V2 = "media://state/v2";
export const MEDIA_MENU_EVENT = "media://menu-action";
export const MEDIA_DEBUG_EVENT = "media://debug";
export const MEDIA_DEBUG_EVENT_V2 = "media://debug/v2";
export const MEDIA_TELEMETRY_EVENT_V2 = "media://telemetry/v2";
export const MEDIA_METADATA_EVENT = "media://metadata";
export const MEDIA_ERROR_EVENT = "media://error";

export interface PlaybackState {
  engine: string;
  status: PlaybackStatus;
  current_path: string | null;
  position_seconds: number;
  duration_seconds: number;
  playback_rate: number;
  error: string | null;
  hw_decode_mode: HardwareDecodeMode;
  hw_decode_active: boolean;
  hw_decode_backend: string | null;
  hw_decode_error: string | null;
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

export interface MediaEventEnvelope<T> {
  protocol_version: number;
  event_type: string;
  request_id: string | null;
  emitted_at_ms: number;
  payload: T;
}

export interface MediaTelemetryPayload {
  source_fps: number;
  render_fps: number;
  queue_depth: number;
  clock_seconds: number;
  audio_drift_seconds: number | null;
  video_pts_gap_seconds: number | null;
  seek_settle_ms: number | null;
}

export interface MediaMetadataPayload {
  width: number;
  height: number;
  fps: number;
  duration_seconds: number;
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
