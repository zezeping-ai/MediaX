<script setup lang="ts">
import { computed, ref } from "vue";
import { message } from "ant-design-vue";
import LyricsSelect from "@/components/selects/LyricsSelect/index.vue";
import { useAppSurfaceTheme } from "@/pages/home/composables/useAppSurfaceTheme";

const props = defineProps<{
  open: boolean;
  title: string;
  artist: string;
  album: string;
  durationSeconds: number;
  lyricsLrc: string;
  lyricsSource: string | null;
  embedLyrics: boolean;
  saving: boolean;
  lyricsSelectKey: number;
  onSave: (lyricsLrc: string) => Promise<void>;
}>();

const emit = defineEmits<{
  "update:open": [boolean];
  "update:title": [string];
  "update:artist": [string];
  "update:album": [string];
  "update:lyricsLrc": [string];
  "update:embedLyrics": [boolean];
}>();

const { insetPanel, sectionSubtitle, sectionTitle } = useAppSurfaceTheme();
const lyricsSelectRef = ref<InstanceType<typeof LyricsSelect> | null>(null);

const lyricsLrcModel = computed({
  get: () => props.lyricsLrc,
  set: (value: string) => emit("update:lyricsLrc", value),
});

const embedLyricsModel = computed({
  get: () => props.embedLyrics,
  set: (value: boolean) => emit("update:embedLyrics", value),
});

const lyricsPickerLocked = computed(() => props.saving || !props.embedLyrics);

function handleOpenChange(next: boolean) {
  if (props.saving && !next) {
    return;
  }
  emit("update:open", next);
}

function handleCancel() {
  if (props.saving) {
    return;
  }
  emit("update:open", false);
}

function resolveLyricsForSave() {
  if (!props.embedLyrics) {
    return "";
  }
  const fromSelect = lyricsSelectRef.value?.resolveSelectedLyrics?.()?.trim() ?? "";
  const fromForm = props.lyricsLrc.trim();
  return fromSelect || fromForm;
}

async function handleOk() {
  const lyricsLrc = resolveLyricsForSave();
  if (props.embedLyrics && !lyricsLrc) {
    message.warning("请先检索并选择歌词，或在下方文本框填写歌词后再保存");
    return;
  }
  if (props.embedLyrics) {
    emit("update:lyricsLrc", lyricsLrc);
  }
  await props.onSave(lyricsLrc);
}
</script>

<template>
  <a-modal
    :open="open"
    title="编辑歌曲信息"
    ok-text="保存"
    cancel-text="取消"
    :width="560"
    :z-index="4000"
    :confirm-loading="saving"
    :mask-closable="!saving"
    @ok="handleOk"
    @cancel="handleCancel"
    @update:open="handleOpenChange"
  >
    <div class="space-y-4">
      <div class="space-y-1">
        <div :class="sectionTitle">歌曲名称</div>
        <a-input
          :value="title"
          placeholder="歌曲名称"
          allow-clear
          @update:value="(value: string) => emit('update:title', value)"
        />
      </div>

      <div class="space-y-1">
        <div :class="sectionTitle">作者</div>
        <a-input
          :value="artist"
          placeholder="作者 / 艺术家"
          allow-clear
          @update:value="(value: string) => emit('update:artist', value)"
        />
      </div>

      <div class="space-y-1">
        <div :class="sectionTitle">专辑</div>
        <a-input
          :value="album"
          placeholder="专辑名称"
          allow-clear
          @update:value="(value: string) => emit('update:album', value)"
        />
      </div>

      <div :class="[insetPanel, 'space-y-3']">
        <div class="flex items-center justify-between gap-3">
          <div class="min-w-0">
            <div :class="sectionTitle">嵌入歌词到文件</div>
            <div :class="sectionSubtitle">
              {{ embedLyrics ? "检索并选择要写入文件的歌词" : "开启后可检索并选择歌词" }}
            </div>
          </div>
          <a-switch
            v-model:checked="embedLyricsModel"
            :disabled="saving"
            checked-children="开"
            un-checked-children="关"
          />
        </div>

        <template v-if="embedLyrics">
          <LyricsSelect
            ref="lyricsSelectRef"
            :key="lyricsSelectKey"
            v-model:lyrics-lrc="lyricsLrcModel"
            mode="search"
            :title="title"
            :artist="artist"
            :album="album"
            :duration-seconds="durationSeconds"
            :lyrics-source="lyricsSource"
            :disabled="lyricsPickerLocked"
            :search-disabled="lyricsPickerLocked"
          />

          <a-textarea
            v-model:value="lyricsLrcModel"
            :rows="8"
            :disabled="saving"
            placeholder="[00:12.50]歌词行..."
          />
        </template>
      </div>
    </div>
  </a-modal>
</template>
