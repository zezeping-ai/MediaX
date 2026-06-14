<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Icon } from "@iconify/vue";
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
  coverLoading: boolean;
  coverPreviewUrl: string | null;
  coverMarkedForRemoval: boolean;
  lyricsSelectKey: number;
  onSave: (lyricsLrc: string) => Promise<void>;
  onPickCover: () => void | Promise<void>;
  onRemoveCover: () => void;
}>();

const emit = defineEmits<{
  "update:open": [boolean];
  "update:title": [string];
  "update:artist": [string];
  "update:album": [string];
  "update:lyricsLrc": [string];
  "update:embedLyrics": [boolean];
}>();

const { insetPanel, isDark, sectionTitle } = useAppSurfaceTheme();
const lyricsSelectRef = ref<InstanceType<typeof LyricsSelect> | null>(null);
const coverEnlargeOpen = ref(false);

const fieldLabel = computed(() => (
  isDark.value ? "mb-1 block text-[11px] font-medium text-white/50" : "mb-1 block text-[11px] font-medium text-slate-500"
));
const sectionDivider = computed(() => (
  isDark.value ? "border-white/8" : "border-black/8"
));
const coverShellClass = computed(() => {
  const base = "group relative aspect-square w-full overflow-hidden rounded-xl transition";
  if (!props.coverPreviewUrl) {
    return isDark.value
      ? `${base} border border-dashed border-white/18 bg-white/2`
      : `${base} border border-dashed border-black/14 bg-black/2`;
  }
  return isDark.value
    ? `${base} border border-white/12 bg-black/35 shadow-[0_8px_24px_rgba(0,0,0,0.28)]`
    : `${base} border border-black/10 bg-white shadow-[0_8px_24px_rgba(15,23,42,0.08)]`;
});
const coverActionClass = computed(() => (
  "flex h-7 w-7 items-center justify-center rounded-md border backdrop-blur-sm transition"
));
const coverActionNeutralClass = computed(() => (
  isDark.value
    ? `${coverActionClass.value} border-white/12 bg-black/55 text-white/90 hover:bg-black/75`
    : `${coverActionClass.value} border-black/10 bg-white/88 text-slate-700 hover:bg-white`
));
const coverActionDangerClass = computed(() => (
  `${coverActionClass.value} border-red-300/30 bg-red-500/75 text-white hover:bg-red-500/90`
));

const lyricsLrcModel = computed({
  get: () => props.lyricsLrc,
  set: (value: string) => emit("update:lyricsLrc", value),
});

const embedLyricsModel = computed({
  get: () => props.embedLyrics,
  set: (value: boolean) => emit("update:embedLyrics", value),
});

const lyricsPickerLocked = computed(() => props.saving || !props.embedLyrics);
const coverActionsDisabled = computed(() => props.saving || props.coverLoading);
const coverPreviewInteractive = computed(
  () => Boolean(props.coverPreviewUrl) && !props.coverLoading && !props.saving,
);
const canRemoveCover = computed(
  () => Boolean(props.coverPreviewUrl) || props.coverMarkedForRemoval,
);

function handlePickCover(event: MouseEvent) {
  event.stopPropagation();
  if (coverActionsDisabled.value) {
    return;
  }
  void props.onPickCover();
}

function handleRemoveCover(event: MouseEvent) {
  event.stopPropagation();
  if (coverActionsDisabled.value || !canRemoveCover.value) {
    return;
  }
  props.onRemoveCover();
}

function handleCoverEnlarge(event: MouseEvent) {
  event.stopPropagation();
  openCoverEnlargePreview();
}

function openCoverEnlargePreview() {
  if (!coverPreviewInteractive.value) {
    return;
  }
  coverEnlargeOpen.value = true;
}

function closeCoverEnlargePreview() {
  coverEnlargeOpen.value = false;
}

watch(
  () => props.open,
  (next) => {
    if (!next) {
      closeCoverEnlargePreview();
    }
  },
);

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
    :width="600"
    :z-index="4000"
    :confirm-loading="saving"
    :mask-closable="!saving"
    :body-style="{ paddingTop: '8px' }"
    @ok="handleOk"
    @cancel="handleCancel"
    @update:open="handleOpenChange"
  >
    <div class="space-y-5">
      <section :class="[insetPanel, 'p-4']">
        <div class="grid grid-cols-[7.5rem_minmax(0,1fr)] items-start gap-x-4 gap-y-3">
          <div :class="[coverShellClass, 'row-span-3']">
            <button
              v-if="!coverPreviewUrl"
              type="button"
              class="flex h-full w-full flex-col items-center justify-center gap-1.5 transition"
              :class="[
                coverActionsDisabled
                  ? 'cursor-not-allowed opacity-60'
                  : (isDark ? 'text-white/45 hover:text-white/72' : 'text-slate-400 hover:text-slate-600'),
              ]"
              :disabled="coverActionsDisabled"
              :title="coverMarkedForRemoval ? '保存后移除' : '选择封皮'"
              aria-label="选择专辑封皮"
              @click="handlePickCover"
            >
              <Icon icon="mdi:image-plus-outline" class="text-2xl" />
            </button>

            <img
              v-else
              :src="coverPreviewUrl"
              alt="专辑封皮预览"
              class="h-full w-full object-cover"
            >

            <div
              v-if="coverPreviewUrl && !coverLoading"
              class="absolute inset-0 flex items-center justify-center gap-1.5 bg-black/52 opacity-0 transition group-hover:opacity-100"
              :class="coverActionsDisabled ? 'pointer-events-none' : ''"
            >
              <button
                type="button"
                :class="coverActionNeutralClass"
                :disabled="!coverPreviewInteractive"
                aria-label="放大预览"
                @click="handleCoverEnlarge"
              >
                <Icon icon="mdi:magnify-plus-outline" class="text-sm" />
              </button>
              <button
                type="button"
                :class="coverActionNeutralClass"
                :disabled="coverActionsDisabled"
                aria-label="更换图片"
                @click="handlePickCover"
              >
                <Icon icon="mdi:image-edit-outline" class="text-sm" />
              </button>
              <button
                type="button"
                :class="coverActionDangerClass"
                :disabled="coverActionsDisabled || !canRemoveCover"
                aria-label="移除封面"
                @click="handleRemoveCover"
              >
                <Icon icon="mdi:delete-outline" class="text-sm" />
              </button>
            </div>

            <div
              v-if="coverLoading"
              class="absolute inset-0 z-10 flex items-center justify-center bg-black/45 backdrop-blur-[1px]"
            >
              <Icon icon="mdi:loading" class="animate-spin text-xl text-white/88" />
            </div>
          </div>

          <div>
            <label :class="fieldLabel">歌曲名称</label>
            <a-input
              :value="title"
              size="small"
              placeholder="歌曲名称"
              allow-clear
              @update:value="(value: string) => emit('update:title', value)"
            />
          </div>

          <div>
            <label :class="fieldLabel">作者</label>
            <a-input
              :value="artist"
              size="small"
              placeholder="作者 / 艺术家"
              allow-clear
              @update:value="(value: string) => emit('update:artist', value)"
            />
          </div>

          <div>
            <label :class="fieldLabel">专辑</label>
            <a-input
              :value="album"
              size="small"
              placeholder="专辑名称"
              allow-clear
              @update:value="(value: string) => emit('update:album', value)"
            />
          </div>
        </div>
      </section>

      <section class="space-y-3">
        <div
          class="flex items-center justify-between gap-3 border-b pb-3"
          :class="sectionDivider"
        >
          <span :class="sectionTitle">嵌入歌词到文件</span>
          <a-switch
            v-model:checked="embedLyricsModel"
            size="small"
            :disabled="saving"
          />
        </div>

        <div v-if="embedLyrics" class="space-y-3">
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
            :is-dark="isDark"
            :disabled="lyricsPickerLocked"
            :search-disabled="lyricsPickerLocked"
          />

          <a-textarea
            v-model:value="lyricsLrcModel"
            :rows="5"
            :disabled="saving"
            placeholder="[00:12.50]歌词行..."
            class="font-mono text-xs"
          />
        </div>
      </section>
    </div>
  </a-modal>

  <Teleport to="body">
    <div
      v-if="coverEnlargeOpen && coverPreviewUrl"
      class="fixed inset-0 z-[5200] flex items-center justify-center bg-black/78 p-6"
      @click.self="closeCoverEnlargePreview"
    >
      <button
        type="button"
        class="absolute right-5 top-5 flex h-9 w-9 items-center justify-center rounded-full border border-white/18 bg-black/45 text-white/88 transition hover:bg-black/65"
        aria-label="关闭封面预览"
        @click="closeCoverEnlargePreview"
      >
        <Icon icon="mdi:close" class="text-lg" />
      </button>
      <img
        :src="coverPreviewUrl"
        alt="专辑封皮大图预览"
        class="max-h-[85vh] max-w-[min(90vw,720px)] object-contain shadow-2xl"
      >
    </div>
  </Teleport>
</template>
