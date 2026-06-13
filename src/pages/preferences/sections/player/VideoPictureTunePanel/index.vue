<script setup lang="ts">
import { computed } from "vue";
import {
  normalizeVideoPictureTune,
  resolveVideoPictureTunePresetId,
  VIDEO_PICTURE_TUNE_CUSTOM_PRESET_ID,
  VIDEO_PICTURE_TUNE_FIELDS,
  VIDEO_PICTURE_TUNE_PRESET_OPTIONS,
  VIDEO_PICTURE_TUNE_PRESETS,
  type VideoPictureTune,
  type VideoPictureTuneKey,
} from "@/modules/video-picture-tune";
import PictureTuneSliderRow from "./PictureTuneSliderRow.vue";

const props = defineProps<{
  tune?: VideoPictureTune | null;
}>();

const emit = defineEmits<{
  "update:tune": [value: VideoPictureTune];
  change: [];
}>();

const resolvedTune = computed(() => normalizeVideoPictureTune(props.tune));

const selectedPresetId = computed({
  get: () => resolveVideoPictureTunePresetId(resolvedTune.value),
  set: (presetId: string) => {
    if (presetId === VIDEO_PICTURE_TUNE_CUSTOM_PRESET_ID) {
      return;
    }
    const preset = VIDEO_PICTURE_TUNE_PRESETS.find((item) => item.id === presetId);
    if (!preset) {
      return;
    }
    emit("update:tune", normalizeVideoPictureTune(preset.tune));
    emit("change");
  },
});

function updateField(key: VideoPictureTuneKey, value: number) {
  emit("update:tune", { ...resolvedTune.value, [key]: value });
  emit("change");
}
</script>

<template>
  <div class="flex flex-col gap-2.5">
    <div class="text-xs text-black/55 dark:text-white/55">
      0 为标准还原；仅影响播放画面，不修改源文件。
    </div>
    <a-radio-group
      v-model:value="selectedPresetId"
      button-style="solid"
      size="small"
      class="picture-tune-presets"
    >
      <a-radio-button
        v-for="option in VIDEO_PICTURE_TUNE_PRESET_OPTIONS"
        :key="option.id"
        :value="option.id"
        :class="
          option.id === VIDEO_PICTURE_TUNE_CUSTOM_PRESET_ID
            ? 'picture-tune-presets__custom'
            : undefined
        "
      >
        {{ option.label }}
      </a-radio-button>
    </a-radio-group>
    <div class="flex flex-col gap-1">
      <PictureTuneSliderRow
        v-for="field in VIDEO_PICTURE_TUNE_FIELDS"
        :key="field.key"
        :label="field.label"
        :value="resolvedTune[field.key]"
        @update:value="(value) => updateField(field.key, value)"
        @change="emit('change')"
      />
    </div>
  </div>
</template>

<style scoped>
.picture-tune-presets {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.picture-tune-presets :deep(.ant-radio-button-wrapper) {
  border-inline-start-width: 1px;
}

.picture-tune-presets :deep(.ant-radio-button-wrapper:not(:first-child)::before) {
  display: none;
}

.picture-tune-presets__custom {
  pointer-events: none;
}
</style>
