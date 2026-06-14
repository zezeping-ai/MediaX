import { computed, ref, watch } from "vue";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { message } from "ant-design-vue";
import { canEditAudioTags } from "@/modules/audio-tag-editing";
import { formatLyricsToLrc } from "@/modules/lyrics";
import {
  coverArtPayloadToDataUrl,
  playbackReadAudioCoverArt,
  playbackReadImageFileForCover,
  playbackWriteAudioMetadata,
} from "@/modules/media-player";
import type { MediaLyricLine, MediaSnapshot } from "@/modules/media-types";
import { resolveDialogPath } from "@/modules/resolve-dialog-path";
import { toUserMediaErrorMessage } from "@/pages/home/composables/useMediaErrorMap";

type CoverArtChange = "none" | "replace" | "remove";

type UseAudioMetadataEditorOptions = {
  sourcePath: () => string;
  title: () => string;
  artist: () => string;
  album: () => string;
  durationSeconds: () => number;
  lyrics: () => MediaLyricLine[];
  lyricsSource: () => string | null;
  hasCoverArt: () => boolean;
  updatePlaybackSnapshot: (snapshot: MediaSnapshot) => void;
};

function buildSaveSuccessMessage(options: {
  embedLyrics: boolean;
  hasLyrics: boolean;
  coverArtChange: CoverArtChange;
}) {
  const parts: string[] = ["歌曲信息"];
  if (options.embedLyrics && options.hasLyrics) {
    parts.push("歌词");
  }
  if (options.coverArtChange !== "none") {
    parts.push("封面");
  }
  return `${parts.join("与")}已保存`;
}

export function useAudioMetadataEditor(options: UseAudioMetadataEditorOptions) {
  const editorOpen = ref(false);
  const saving = ref(false);
  const coverLoading = ref(false);
  const lyricsSelectKey = ref(0);
  const formTitle = ref("");
  const formArtist = ref("");
  const formAlbum = ref("");
  const formLyricsLrc = ref("");
  const embedLyrics = ref(true);
  const coverPreviewUrl = ref<string | null>(null);
  const coverArtChange = ref<CoverArtChange>("none");
  const coverArtBase64 = ref<string | null>(null);
  const coverArtMimeType = ref<string | null>(null);
  const hadInitialCover = ref(false);

  const canEdit = computed(() => canEditAudioTags(options.sourcePath()));
  const coverMarkedForRemoval = computed(
    () => coverArtChange.value === "remove" && (hadInitialCover.value || Boolean(coverPreviewUrl.value)),
  );

  async function loadCurrentCoverPreview() {
    const path = options.sourcePath().trim();
    coverPreviewUrl.value = null;
    coverArtChange.value = "none";
    coverArtBase64.value = null;
    coverArtMimeType.value = null;
    hadInitialCover.value = options.hasCoverArt();
    if (!path || !canEditAudioTags(path)) {
      return;
    }

    coverLoading.value = true;
    try {
      const payload = await playbackReadAudioCoverArt(path);
      if (payload) {
        coverPreviewUrl.value = coverArtPayloadToDataUrl(payload);
        hadInitialCover.value = true;
      }
    } catch (error) {
      message.error(toUserMediaErrorMessage(error));
    } finally {
      coverLoading.value = false;
    }
  }

  function resetForm() {
    formTitle.value = options.title();
    formArtist.value = options.artist();
    formAlbum.value = options.album();
    const initialLrc = formatLyricsToLrc(options.lyrics());
    formLyricsLrc.value = initialLrc;
    embedLyrics.value = Boolean(initialLrc.trim());
    lyricsSelectKey.value += 1;
    void loadCurrentCoverPreview();
  }

  function showEditor() {
    if (!canEdit.value) {
      message.warning("当前音频格式不支持写入标签");
      return;
    }
    resetForm();
    editorOpen.value = true;
  }

  function closeEditor() {
    editorOpen.value = false;
  }

  async function pickCoverImage() {
    if (saving.value) {
      return;
    }
    const selected = await openFileDialog({
      multiple: false,
      filters: [
        {
          name: "图片",
          extensions: ["jpg", "jpeg", "png", "gif", "bmp"],
        },
      ],
    });
    const imagePath = resolveDialogPath(selected);
    if (!imagePath) {
      return;
    }

    coverLoading.value = true;
    try {
      const payload = await playbackReadImageFileForCover(imagePath);
      coverPreviewUrl.value = coverArtPayloadToDataUrl(payload);
      coverArtChange.value = "replace";
      coverArtBase64.value = payload.data_base64;
      coverArtMimeType.value = payload.mime_type;
    } catch (error) {
      message.error(toUserMediaErrorMessage(error));
    } finally {
      coverLoading.value = false;
    }
  }

  function removeCoverImage() {
    if (saving.value) {
      return;
    }
    coverPreviewUrl.value = null;
    coverArtChange.value = "remove";
    coverArtBase64.value = null;
    coverArtMimeType.value = null;
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
    if (coverArtChange.value === "replace" && !coverArtBase64.value) {
      message.warning("请先选择封面图片后再保存");
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
        coverArtChange: coverArtChange.value,
        coverArtDataBase64: coverArtBase64.value ?? undefined,
        coverArtMimeType: coverArtMimeType.value ?? undefined,
      });
      options.updatePlaybackSnapshot(snapshot);
      message.success(
        buildSaveSuccessMessage({
          embedLyrics: embedLyrics.value,
          hasLyrics: Boolean(lyricsLrc),
          coverArtChange: coverArtChange.value,
        }),
      );
      editorOpen.value = false;
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
        editorOpen.value = false;
      }
    },
  );

  return {
    album: formAlbum,
    artist: formArtist,
    canEdit,
    closeEditor,
    coverArtChange,
    coverLoading,
    coverMarkedForRemoval,
    coverPreviewUrl,
    durationSeconds: computed(() => options.durationSeconds()),
    embedLyrics,
    lyricsLrc: formLyricsLrc,
    lyricsSelectKey,
    lyricsSource: computed(() => options.lyricsSource()),
    open: editorOpen,
    pickCoverImage,
    removeCoverImage,
    saveEditor,
    saving,
    showEditor,
    title: formTitle,
  };
}
