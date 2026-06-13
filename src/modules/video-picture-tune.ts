export type VideoPictureTune = {
  brightness: number;
  contrast: number;
  saturation: number;
  gamma: number;
  hue: number;
};

export type VideoPictureTuneKey = keyof VideoPictureTune;

export const VIDEO_PICTURE_TUNE_MIN = -100;
export const VIDEO_PICTURE_TUNE_MAX = 100;
export const VIDEO_PICTURE_TUNE_CUSTOM_PRESET_ID = "custom";

export const DEFAULT_VIDEO_PICTURE_TUNE: VideoPictureTune = {
  brightness: 0,
  contrast: 0,
  saturation: 0,
  gamma: 0,
  hue: 0,
};

export type VideoPictureTunePreset = {
  id: string;
  label: string;
  tune: VideoPictureTune;
};

/** 市面播放器常见画面预设（mpv / VLC / IINA 同类调校量级） */
export const VIDEO_PICTURE_TUNE_PRESETS: ReadonlyArray<VideoPictureTunePreset> = [
  { id: "default", label: "默认", tune: { ...DEFAULT_VIDEO_PICTURE_TUNE } },
  {
    id: "bright",
    label: "明亮",
    tune: { brightness: 25, contrast: 10, saturation: 8, gamma: 5, hue: 0 },
  },
  {
    id: "cinema",
    label: "影院",
    tune: { brightness: -8, contrast: 18, saturation: -12, gamma: 10, hue: 0 },
  },
  {
    id: "vivid",
    label: "鲜艳",
    tune: { brightness: 12, contrast: 14, saturation: 30, gamma: 0, hue: 0 },
  },
  {
    id: "soft",
    label: "柔和",
    tune: { brightness: -5, contrast: -12, saturation: -10, gamma: -8, hue: 0 },
  },
];

export const VIDEO_PICTURE_TUNE_PRESET_OPTIONS: ReadonlyArray<{
  id: string;
  label: string;
}> = [
  ...VIDEO_PICTURE_TUNE_PRESETS.map(({ id, label }) => ({ id, label })),
  { id: VIDEO_PICTURE_TUNE_CUSTOM_PRESET_ID, label: "自定义" },
];

export const VIDEO_PICTURE_TUNE_FIELDS: ReadonlyArray<{
  key: VideoPictureTuneKey;
  label: string;
}> = [
  { key: "brightness", label: "亮度" },
  { key: "contrast", label: "对比度" },
  { key: "saturation", label: "饱和度" },
  { key: "gamma", label: "伽马" },
  { key: "hue", label: "色相" },
];

export function clampVideoPictureTuneValue(value: number) {
  return Math.round(
    Math.min(VIDEO_PICTURE_TUNE_MAX, Math.max(VIDEO_PICTURE_TUNE_MIN, value)),
  );
}

export function normalizeVideoPictureTune(
  tune: Partial<VideoPictureTune> | null | undefined,
): VideoPictureTune {
  return {
    brightness: clampVideoPictureTuneValue(tune?.brightness ?? 0),
    contrast: clampVideoPictureTuneValue(tune?.contrast ?? 0),
    saturation: clampVideoPictureTuneValue(tune?.saturation ?? 0),
    gamma: clampVideoPictureTuneValue(tune?.gamma ?? 0),
    hue: clampVideoPictureTuneValue(tune?.hue ?? 0),
  };
}

export function formatVideoPictureTuneValue(value: number) {
  const rounded = clampVideoPictureTuneValue(value);
  return rounded > 0 ? `+${rounded}` : `${rounded}`;
}

export function matchVideoPictureTunePreset(tune: VideoPictureTune | null | undefined): string | null {
  const normalized = normalizeVideoPictureTune(tune);
  for (const preset of VIDEO_PICTURE_TUNE_PRESETS) {
    if (VIDEO_PICTURE_TUNE_FIELDS.every(({ key }) => normalized[key] === preset.tune[key])) {
      return preset.id;
    }
  }
  return null;
}

export function resolveVideoPictureTunePresetId(tune: VideoPictureTune | null | undefined): string {
  return matchVideoPictureTunePreset(tune) ?? VIDEO_PICTURE_TUNE_CUSTOM_PRESET_ID;
}

export function isDefaultVideoPictureTune(tune: VideoPictureTune | null | undefined) {
  return resolveVideoPictureTunePresetId(tune) === "default";
}

export function resolveStoredVideoPictureTune(
  player: Partial<{
    videoPictureTune: Partial<VideoPictureTune>;
    videoPictureBrightness: number;
    videoPictureContrast: number;
  }> | null | undefined,
): VideoPictureTune {
  return normalizeVideoPictureTune({
    brightness: player?.videoPictureTune?.brightness ?? player?.videoPictureBrightness,
    contrast: player?.videoPictureTune?.contrast ?? player?.videoPictureContrast,
    saturation: player?.videoPictureTune?.saturation,
    gamma: player?.videoPictureTune?.gamma,
    hue: player?.videoPictureTune?.hue,
  });
}
