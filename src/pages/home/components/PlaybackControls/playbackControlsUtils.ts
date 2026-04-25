export function formatSeconds(value: number) {
  const safeValue = Math.max(0, Math.floor(value || 0));
  const minutes = Math.floor(safeValue / 60);
  const seconds = safeValue % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

export interface PlaybackQualityOption {
  key: string;
  label: string;
}

function sourceQualityLabel(sourceHeight: number | null) {
  const normalized =
    typeof sourceHeight === "number" && Number.isFinite(sourceHeight) && sourceHeight > 0
      ? Math.round(sourceHeight)
      : null;
  if (!normalized) {
    return "原画";
  }
  return `${normalized}P（原画）`;
}

export function buildPlaybackQualityOptions(
  sourceHeight: number | null,
  downgradeLevels: readonly number[],
  supportsAdaptive: boolean,
  selectedQuality?: string,
): PlaybackQualityOption[] {
  const normalizedHeight =
    typeof sourceHeight === "number" && Number.isFinite(sourceHeight) && sourceHeight > 0
      ? sourceHeight
      : null;
  const sourceLabel = sourceQualityLabel(normalizedHeight);
  const options: PlaybackQualityOption[] = [{ key: "source", label: sourceLabel }];

  if (supportsAdaptive) {
    options.push({ key: "auto", label: "自动" });
  }

  if (normalizedHeight !== null) {
    for (const level of downgradeLevels) {
      if (level < normalizedHeight) {
        options.push({ key: `${level}p`, label: `${level}P` });
      }
    }
  }

  // Keep currently selected item visible even when runtime metadata reflects a downscaled output.
  if (selectedQuality && !options.some((option) => option.key === selectedQuality)) {
    if (selectedQuality === "source") {
      options.unshift({ key: "source", label: sourceLabel });
    } else if (selectedQuality === "auto") {
      options.push({ key: "auto", label: "自动" });
    } else {
      const match = selectedQuality.match(/^(\d+)p$/i);
      if (match) {
        options.push({ key: selectedQuality, label: `${match[1]}P` });
      }
    }
  }

  return options;
}

