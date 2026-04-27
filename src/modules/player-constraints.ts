export const MIN_PLAYBACK_RATE = 0.25;
export const MAX_PLAYBACK_RATE = 4;
export const MIN_PREVIEW_EDGE = 32;
export const MAX_PREVIEW_EDGE = 4096;
const PLAYBACK_RATE_PRECISION = 100;

export function normalizeNonNegative(value: number, field: string) {
  if (!Number.isFinite(value)) {
    throw new Error(`${field} must be a finite number`);
  }
  return Math.max(value, 0);
}

export function normalizePlaybackRate(playbackRate: number) {
  if (!Number.isFinite(playbackRate)) {
    throw new Error("playbackRate must be a finite number");
  }
  const clamped = Math.min(MAX_PLAYBACK_RATE, Math.max(MIN_PLAYBACK_RATE, playbackRate));
  return Math.round(clamped * PLAYBACK_RATE_PRECISION) / PLAYBACK_RATE_PRECISION;
}

export function formatPlaybackRate(playbackRate: number) {
  const normalized = normalizePlaybackRate(playbackRate);
  return `${normalized.toFixed(2).replace(/\.?0+$/, "")}x`;
}

export function normalizeUnitInterval(value: number, field: string) {
  if (!Number.isFinite(value)) {
    throw new Error(`${field} must be a finite number`);
  }
  return Math.min(1, Math.max(0, value));
}

export function normalizePreviewEdge(value: number) {
  if (!Number.isFinite(value)) {
    throw new Error("preview edge must be a finite number");
  }
  return Math.min(MAX_PREVIEW_EDGE, Math.max(MIN_PREVIEW_EDGE, Math.round(value)));
}
