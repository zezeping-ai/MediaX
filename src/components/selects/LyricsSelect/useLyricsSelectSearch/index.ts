import { computed, onMounted, ref, type Ref } from "vue";
import { message } from "ant-design-vue";
import { playbackSearchLyrics } from "@/modules/media-player";
import { toUserMediaErrorMessage } from "@/pages/home/composables/useMediaErrorMap";
import { fromLocalLyrics, fromSearchHit, isLocalLyricsOption } from "../normalize";
import type { LyricsSelectOption } from "../types";
import type { LyricsSelectQuery } from "./types";

export type { LyricsSelectQuery } from "./types";

type UseLyricsSelectSearchOptions = {
  query: () => LyricsSelectQuery;
  lyricsLrc: Ref<string>;
  localSource: () => string | null;
};

export function useLyricsSelectSearch(options: UseLyricsSelectSearchOptions) {
  const searching = ref(false);
  const optionsList = ref<LyricsSelectOption[]>([]);
  const selectedId = ref<string | null>(null);

  const hasResults = computed(() => optionsList.value.length > 0);
  const selectedOption = computed(() =>
    optionsList.value.find((item) => item.id === selectedId.value) ?? null,
  );

  function buildLocalOption() {
    const { title, artist, album, durationSeconds } = options.query();
    return fromLocalLyrics({
      lyricsLrc: options.lyricsLrc.value,
      source: options.localSource(),
      title,
      artist,
      album,
      durationSeconds,
    });
  }

  function seedLocalOption() {
    const localOption = buildLocalOption();
    if (!localOption) {
      return;
    }
    optionsList.value = [localOption];
    selectedId.value = localOption.id;
  }

  function reset() {
    searching.value = false;
    optionsList.value = [];
    selectedId.value = null;
    seedLocalOption();
  }

  function applyOption(option: LyricsSelectOption) {
    selectedId.value = option.id;
    if (option.lyrics_lrc) {
      options.lyricsLrc.value = option.lyrics_lrc;
    }
  }

  function selectOption(optionId: string) {
    const option = optionsList.value.find((item) => item.id === optionId);
    if (option) {
      applyOption(option);
    }
  }

  function mergeSearchResults(online: LyricsSelectOption[]) {
    const localOption = optionsList.value.find(isLocalLyricsOption) ?? buildLocalOption();
    if (!localOption) {
      optionsList.value = online;
      return online[0] ?? null;
    }
    optionsList.value = [
      localOption,
      ...online.filter((item) => item.id !== localOption.id),
    ];
    return localOption.provider_id === "embedded" ? localOption : optionsList.value[0] ?? null;
  }

  async function search() {
    const { title, artist, album, durationSeconds } = options.query();
    const trimmedTitle = title.trim();
    const trimmedArtist = artist.trim();
    if (!trimmedTitle && !trimmedArtist) {
      message.warning("请先输入歌曲名称或作者后再检索");
      return;
    }

    searching.value = true;
    try {
      const hits = await playbackSearchLyrics({
        title: trimmedTitle,
        artist: trimmedArtist || undefined,
        album: album.trim() || undefined,
        durationSeconds,
      });
      const online = hits.map(fromSearchHit);
      if (online.length === 0 && !buildLocalOption()) {
        selectedId.value = null;
        optionsList.value = [];
        message.info("未找到匹配歌词");
        return;
      }
      const preferred = mergeSearchResults(online);
      if (preferred) {
        applyOption(preferred);
      }
      if (online.length === 0 && preferred) {
        message.info("未找到新的在线歌词，已保留内嵌歌词");
      }
    } catch (error) {
      message.error(toUserMediaErrorMessage(error));
    } finally {
      searching.value = false;
    }
  }

  function resolveSelectedLyrics() {
    const selected = selectedOption.value;
    if (selected?.lyrics_lrc?.trim()) {
      options.lyricsLrc.value = selected.lyrics_lrc;
      return selected.lyrics_lrc.trim();
    }
    return options.lyricsLrc.value.trim();
  }

  onMounted(() => {
    seedLocalOption();
  });

  return {
    hasResults,
    options: optionsList,
    reset,
    search,
    searching,
    selectOption,
    selectedId,
    selectedOption,
    resolveSelectedLyrics,
  };
}
