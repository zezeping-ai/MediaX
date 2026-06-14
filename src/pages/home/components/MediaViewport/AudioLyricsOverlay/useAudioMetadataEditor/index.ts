import { computed, ref, watch } from "vue";
import { message } from "ant-design-vue";
import { canEditAudioTags } from "@/modules/audio-tag-editing";
import { formatLyricsToLrc } from "@/modules/lyrics";
import { playbackWriteAudioMetadata } from "@/modules/media-player";
import type { MediaLyricLine, MediaSnapshot } from "@/modules/media-types";
import { toUserMediaErrorMessage } from "@/pages/home/composables/useMediaErrorMap";

type UseAudioMetadataEditorOptions = {
  sourcePath: () => string;
  title: () => string;
  artist: () => string;
  album: () => string;
  durationSeconds: () => number;
  lyrics: () => MediaLyricLine[];
  lyricsSource: () => string | null;
  updatePlaybackSnapshot: (snapshot: MediaSnapshot) => void;
};

export function useAudioMetadataEditor(options: UseAudioMetadataEditorOptions) {
  const open = ref(false);
  const saving = ref(false);
  const lyricsSelectKey = ref(0);
  const formTitle = ref("");
  const formArtist = ref("");
  const formAlbum = ref("");
  const formLyricsLrc = ref("");
  const embedLyrics = ref(true);

  const canEdit = computed(() => canEditAudioTags(options.sourcePath()));

  function resetForm() {
    formTitle.value = options.title();
    formArtist.value = options.artist();
    formAlbum.value = options.album();
    const initialLrc = formatLyricsToLrc(options.lyrics());
    formLyricsLrc.value = initialLrc;
    // 无歌词时默认只改元数据，避免保存被「嵌入歌词」校验静默拦住。
    embedLyrics.value = Boolean(initialLrc.trim());
    lyricsSelectKey.value += 1;
  }

  function showEditor() {
    if (!canEdit.value) {
      message.warning("当前音频格式不支持写入标签");
      return;
    }
    resetForm();
    open.value = true;
  }

  function closeEditor() {
    open.value = false;
  }

  async function saveEditor(resolvedLyricsLrc?: string) {
    const path = options.sourcePath().trim();
    if (!path || !canEditAudioTags(path)) {
      message.warning("当前音频不支持保存标签");
      return;
    }

    const resolved = resolvedLyricsLrc?.trim() ?? "";
    const lyricsLrc = embedLyrics.value
      ? (resolved || formLyricsLrc.value.trim())
      : "";
    if (resolved) {
      formLyricsLrc.value = resolved;
    }
    if (embedLyrics.value && !lyricsLrc) {
      message.warning("请先检索并选择歌词，或在下方文本框填写歌词后再保存");
      return;
    }

    saving.value = true;
    try {
      const snapshot = await playbackWriteAudioMetadata({
        path,
        title: formTitle.value,
        artist: formArtist.value,
        album: formAlbum.value,
        lyricsLrc,
        embedLyrics: embedLyrics.value,
      });
      options.updatePlaybackSnapshot(snapshot);
      message.success(embedLyrics.value && lyricsLrc ? "歌曲信息与歌词已保存" : "歌曲信息已保存");
      open.value = false;
    } catch (error) {
      message.error(toUserMediaErrorMessage(error));
    } finally {
      saving.value = false;
    }
  }

  watch(
    () => options.sourcePath(),
    () => {
      if (!canEdit.value) {
        open.value = false;
      }
    },
  );

  return {
    album: formAlbum,
    artist: formArtist,
    canEdit,
    closeEditor,
    durationSeconds: computed(() => options.durationSeconds()),
    embedLyrics,
    lyricsLrc: formLyricsLrc,
    lyricsSelectKey,
    lyricsSource: computed(() => options.lyricsSource()),
    open,
    saveEditor,
    saving,
    showEditor,
    title: formTitle,
  };
}
