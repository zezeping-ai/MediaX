import { invokeMediaCommand } from "@/modules/media-command";
import type { TranscodeQueueSnapshot } from "@/modules/media-types";

export interface VideoTranscodePayload {
  source_path: string;
  output_dir: string;
  format: string;
  resolution: string;
  playback_rate: number;
}

export interface AudioTranscodePayload {
  source_path: string;
  output_dir: string;
  format: string;
  playback_rate: number;
}

export interface ImageCompressPayload {
  source_paths: string[];
  output_dir?: string;
  mode: "lossless" | "lossy";
  format?: "jpeg" | "webp" | "png" | "gif" | "bmp";
  quality?: number;
}

export interface ImageCompressEstimatePayload {
  source_paths: string[];
  mode: "lossless" | "lossy";
  format?: "jpeg" | "webp" | "png" | "gif" | "bmp";
  quality?: number;
}

export interface ImageCompressEstimateResult {
  total_input_size_bytes: number;
  estimated_output_size_bytes: number;
}

export function transcodeQueueSnapshot() {
  return invokeMediaCommand<TranscodeQueueSnapshot>("transcode_queue_snapshot");
}

export function transcodeVideoEnqueue(payload: VideoTranscodePayload) {
  return invokeMediaCommand<TranscodeQueueSnapshot>("transcode_video_enqueue", { payload });
}

export function transcodeAudioEnqueue(payload: AudioTranscodePayload) {
  return invokeMediaCommand<TranscodeQueueSnapshot>("transcode_audio_enqueue", { payload });
}

export function imageCompressEnqueue(payload: ImageCompressPayload) {
  return invokeMediaCommand<TranscodeQueueSnapshot>("image_compress_enqueue", { payload });
}

export function imageCompressEstimate(payload: ImageCompressEstimatePayload) {
  return invokeMediaCommand<ImageCompressEstimateResult>("image_compress_estimate", { payload });
}

export function transcodeJobCancel(jobId: number) {
  return invokeMediaCommand<TranscodeQueueSnapshot>("transcode_job_cancel", { jobId });
}

export function transcodeJobRemove(jobId: number) {
  return invokeMediaCommand<TranscodeQueueSnapshot>("transcode_job_remove", { jobId });
}

export function revealFileInSystem(path: string) {
  return invokeMediaCommand<void>("window_reveal_file", { path });
}
