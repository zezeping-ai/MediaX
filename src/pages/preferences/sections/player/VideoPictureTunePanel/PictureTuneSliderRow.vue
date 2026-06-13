<script setup lang="ts">
import { computed } from "vue";
import {
  formatVideoPictureTuneValue,
  VIDEO_PICTURE_TUNE_MAX,
  VIDEO_PICTURE_TUNE_MIN,
} from "@/modules/video-picture-tune";

const props = defineProps<{
  label: string;
  value: number;
}>();

const emit = defineEmits<{
  "update:value": [value: number];
  change: [];
}>();

const displayValue = computed(() => formatVideoPictureTuneValue(props.value));
const isNeutral = computed(() => props.value === 0);

function onSliderChange(next: number | [number, number]) {
  const value = Array.isArray(next) ? next[0] : next;
  emit("update:value", value);
  emit("change");
}
</script>

<template>
  <div class="picture-tune-row">
    <span class="picture-tune-row__label">{{ label }}</span>
    <a-slider
      class="picture-tune-row__slider"
      :value="value"
      :min="VIDEO_PICTURE_TUNE_MIN"
      :max="VIDEO_PICTURE_TUNE_MAX"
      :step="1"
      :tooltip-open="false"
      @change="onSliderChange"
    />
    <span
      class="picture-tune-row__value"
      :class="isNeutral ? 'picture-tune-row__value--neutral' : ''"
      :aria-label="`${label} ${displayValue}`"
    >
      {{ displayValue }}
    </span>
  </div>
</template>

<style scoped>
.picture-tune-row {
  display: grid;
  grid-template-columns: 3.25rem minmax(0, 1fr) 2.75rem;
  align-items: center;
  gap: 8px;
}

.picture-tune-row__label {
  font-size: 13px;
  font-weight: 600;
}

.picture-tune-row__slider {
  margin: 0 !important;
}

.picture-tune-row__slider :deep(.ant-slider) {
  margin: 0;
}

.picture-tune-row__value {
  text-align: right;
  font-size: 12px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  color: rgba(15, 23, 42, 0.88);
}

:global([data-theme="dark"] .picture-tune-row__value) {
  color: rgba(255, 255, 255, 0.9);
}

.picture-tune-row__value--neutral {
  color: rgba(15, 23, 42, 0.45);
}

:global([data-theme="dark"] .picture-tune-row__value--neutral) {
  color: rgba(255, 255, 255, 0.45);
}
</style>
