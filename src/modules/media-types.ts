export type PlaybackStatus = "idle" | "playing" | "paused" | "stopped";
export type HardwareDecodeMode = "auto" | "on" | "off";

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
