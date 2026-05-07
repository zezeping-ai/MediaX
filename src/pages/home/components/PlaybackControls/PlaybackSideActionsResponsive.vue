<script setup lang="ts">
import PlaybackSideActions from "./PlaybackSideActions.vue";
import type { SideActionEmitContract, SideActionViewProps } from "./bindings.contract";

const props = defineProps<SideActionViewProps>();

const emit = defineEmits<SideActionEmitContract>();
</script>

<template>
  <div class="status-indicators status-indicators--dock">
    <div class="status-indicators__group">
      <span
        class="inline-flex h-6 items-center rounded-md border px-2 text-[10px] font-semibold tracking-[0.1px] leading-none transition-colors duration-150"
        :class="[props.decodeBadgeClass, 'justify-center whitespace-nowrap']"
        :title="props.decodeBadgeTitle"
      >
        {{ props.decodeBadgeLabel }}
      </span>
    </div>
  </div>
  <div class="side-actions side-actions--dock">
    <PlaybackSideActions
      v-bind="props"
      @toggle-cache="emit('toggle-cache')"
      @toggle-lock="emit('toggle-lock')"
      @export-audio="emit('export-audio')"
    />
  </div>
  <div class="side-actions side-actions--stack">
    <div class="status-indicators__group">
      <span
        class="inline-flex h-6 items-center rounded-md border px-2 text-[10px] font-semibold tracking-[0.1px] leading-none transition-colors duration-150"
        :class="[props.decodeBadgeClass, 'justify-center whitespace-nowrap']"
        :title="props.decodeBadgeTitle"
      >
        {{ props.decodeBadgeLabel }}
      </span>
    </div>
    <PlaybackSideActions
      v-bind="props"
      @toggle-cache="emit('toggle-cache')"
      @toggle-lock="emit('toggle-lock')"
      @export-audio="emit('export-audio')"
    />
  </div>
</template>

<style scoped>
.status-indicators--dock {
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  display: none;
}

.status-indicators__group {
  display: flex;
  align-items: center;
}

.side-actions--dock {
  position: absolute;
  right: 0;
  top: 50%;
  transform: translateY(-50%);
  display: none;
}

.side-actions--stack {
  margin-top: 0.5rem;
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 0.5rem;
}

@media (min-width: 860px) {
  .status-indicators--dock {
    display: block;
  }
  .side-actions--dock {
    display: block;
  }
  .side-actions--stack {
    display: none;
  }
}
</style>
