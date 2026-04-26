import type { SliderValue } from "./types";

export function normalizeSliderValue(value: SliderValue) {
  return Array.isArray(value) ? Number(value[0]) : Number(value);
}
