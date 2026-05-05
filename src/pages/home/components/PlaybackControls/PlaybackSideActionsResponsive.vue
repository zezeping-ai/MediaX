<script setup lang="ts">
import PlaybackSideActions from "./PlaybackSideActions.vue";
import type { SideActionEmitContract, SideActionViewProps } from "./bindings.contract";

const props = defineProps<SideActionViewProps>();

const emit = defineEmits<SideActionEmitContract>();
</script>

<template>
  <div class="side-actions side-actions--dock">
    <PlaybackSideActions
      v-bind="props"
      @toggle-cache="emit('toggle-cache')"
      @toggle-lock="emit('toggle-lock')"
      @export-audio="emit('export-audio')"
    />
  </div>
  <div class="side-actions side-actions--stack">
    <PlaybackSideActions
      v-bind="props"
      @toggle-cache="emit('toggle-cache')"
      @toggle-lock="emit('toggle-lock')"
      @export-audio="emit('export-audio')"
    />
  </div>
</template>

<style scoped>
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
}

@media (min-width: 860px) {
  .side-actions--dock {
    display: block;
  }
  .side-actions--stack {
    display: none;
  }
}
</style>
