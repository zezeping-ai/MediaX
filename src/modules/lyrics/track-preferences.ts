import { computed, type Ref } from "vue";
import { useStorage } from "@vueuse/core";

export type LyricsTrackPreferences = {
  hidden?: boolean;
  offsetSeconds?: number;
};

const OFFSET_STEP_SECONDS = 0.5;
const FINE_OFFSET_STEP_SECONDS = 0.1;
const MAX_OFFSET_SECONDS = 30;

const trackPreferences = useStorage<Record<string, LyricsTrackPreferences>>(
  "mediax.lyrics.track-preferences",
  {},
  localStorage,
  { mergeDefaults: true },
);

function readTrackPreferences(sourcePath: string): LyricsTrackPreferences {
  if (!sourcePath) {
    return {};
  }
  return trackPreferences.value[sourcePath] ?? {};
}

function writeTrackPreferences(sourcePath: string, next: LyricsTrackPreferences) {
  if (!sourcePath) {
    return;
  }
  const hidden = next.hidden ?? false;
  const offsetSeconds = next.offsetSeconds ?? 0;
  if (!hidden && Math.abs(offsetSeconds) < 0.001) {
    const { [sourcePath]: _, ...rest } = trackPreferences.value;
    trackPreferences.value = rest;
    return;
  }
  trackPreferences.value = {
    ...trackPreferences.value,
    [sourcePath]: {
      hidden: hidden || undefined,
      offsetSeconds: Math.abs(offsetSeconds) >= 0.001 ? offsetSeconds : undefined,
    },
  };
}

function clampOffset(value: number) {
  return Math.max(-MAX_OFFSET_SECONDS, Math.min(MAX_OFFSET_SECONDS, value));
}

export function useLyricsTrackPreferences(sourcePath: Readonly<Ref<string>>) {
  const hidden = computed({
    get: () => readTrackPreferences(sourcePath.value).hidden ?? false,
    set: (value: boolean) => {
      const current = readTrackPreferences(sourcePath.value);
      writeTrackPreferences(sourcePath.value, { ...current, hidden: value });
    },
  });

  const offsetSeconds = computed({
    get: () => readTrackPreferences(sourcePath.value).offsetSeconds ?? 0,
    set: (value: number) => {
      const current = readTrackPreferences(sourcePath.value);
      writeTrackPreferences(sourcePath.value, {
        ...current,
        offsetSeconds: clampOffset(value),
      });
    },
  });

  function adjustOffset(deltaSeconds: number) {
    offsetSeconds.value = clampOffset(offsetSeconds.value + deltaSeconds);
  }

  function resetOffset() {
    const current = readTrackPreferences(sourcePath.value);
    writeTrackPreferences(sourcePath.value, { ...current, offsetSeconds: 0 });
  }

  return {
    hidden,
    offsetSeconds,
    adjustOffset,
    resetOffset,
    offsetStepSeconds: OFFSET_STEP_SECONDS,
    fineOffsetStepSeconds: FINE_OFFSET_STEP_SECONDS,
  };
}
