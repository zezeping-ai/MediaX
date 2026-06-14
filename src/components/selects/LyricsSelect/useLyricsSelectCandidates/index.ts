import { computed, ref } from "vue";
import { playbackSelectLyricsCandidate } from "@/modules/media-player/playbackCommands";
import type { LyricsCandidateSummary, MediaSnapshot } from "@/modules/media-types";
import { fromCandidate } from "../normalize";

type UseLyricsSelectCandidatesOptions = {
  candidates: () => LyricsCandidateSummary[];
  selectedId: () => string | null;
  updatePlaybackSnapshot: (snapshot: MediaSnapshot) => void;
};

export function useLyricsSelectCandidates(options: UseLyricsSelectCandidatesOptions) {
  const switching = ref(false);

  const optionsList = computed(() => options.candidates().map(fromCandidate));
  const selectedOption = computed(() =>
    optionsList.value.find((item) => item.id === options.selectedId())
    ?? optionsList.value[0]
    ?? null,
  );

  async function selectOption(nextId: string) {
    if (!nextId || nextId === options.selectedId() || switching.value) {
      return;
    }
    switching.value = true;
    try {
      const snapshot = await playbackSelectLyricsCandidate(nextId);
      options.updatePlaybackSnapshot(snapshot);
    } finally {
      switching.value = false;
    }
  }

  return {
    options: optionsList,
    selectOption,
    selectedOption,
    switching,
  };
}
