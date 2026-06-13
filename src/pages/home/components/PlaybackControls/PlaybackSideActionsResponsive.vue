<script setup lang="ts">
import PlaybackSideActions from "./PlaybackSideActions.vue";
import type { SideActionEmitContract, SideActionViewProps } from "./bindings.contract";

const props = defineProps<SideActionViewProps & {
  variant?: "auto" | "dock-left" | "dock-right" | "stack";
}>();

const emit = defineEmits<SideActionEmitContract>();

const variant = props.variant ?? "auto";
</script>

<template>
  <div
    v-if="variant === 'auto' || variant === 'dock-left'"
    class="flex items-center"
    :class="variant === 'auto' ? 'status-indicators--dock' : 'justify-start'"
  >
    <span
      class="inline-flex h-6 items-center rounded-md border px-2 text-[10px] font-semibold tracking-[0.1px] leading-none transition-colors duration-150"
      :class="[
        props.decodeBadgeClass,
        'justify-center whitespace-nowrap',
        !props.showDecodeBadge && 'pointer-events-none invisible',
      ]"
      :title="props.showDecodeBadge ? props.decodeBadgeTitle : undefined"
      aria-hidden="true"
    >
      {{ props.decodeBadgeLabel }}
    </span>
  </div>

  <div
    v-if="variant === 'auto' || variant === 'dock-right'"
    class="flex items-center"
    :class="variant === 'auto' ? 'side-actions--dock' : 'justify-end'"
  >
    <PlaybackSideActions
      v-bind="props"
      @toggle-cache="emit('toggle-cache')"
      @toggle-lock="emit('toggle-lock')"
      @toggle-playlist="emit('toggle-playlist')"
      @export-audio="emit('export-audio')"
    />
  </div>

  <div
    v-if="variant === 'auto' || variant === 'stack'"
    class="side-actions--stack"
    :class="variant === 'stack' ? 'col-span-full mt-2' : ''"
  >
    <span
      class="inline-flex h-6 items-center rounded-md border px-2 text-[10px] font-semibold tracking-[0.1px] leading-none transition-colors duration-150"
      :class="[
        props.decodeBadgeClass,
        'justify-center whitespace-nowrap',
        !props.showDecodeBadge && 'pointer-events-none invisible',
      ]"
      :title="props.showDecodeBadge ? props.decodeBadgeTitle : undefined"
      aria-hidden="true"
    >
      {{ props.decodeBadgeLabel }}
    </span>
    <PlaybackSideActions
      v-bind="props"
      @toggle-cache="emit('toggle-cache')"
      @toggle-lock="emit('toggle-lock')"
      @toggle-playlist="emit('toggle-playlist')"
      @export-audio="emit('export-audio')"
    />
  </div>
</template>

<style scoped>
.side-actions--stack {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 0.5rem;
}

@media (min-width: 860px) {
  .status-indicators--dock {
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
  }

  .side-actions--dock {
    position: absolute;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
  }

  .side-actions--stack {
    display: none;
  }
}

@media (max-width: 859px) {
  .status-indicators--dock,
  .side-actions--dock {
    display: none;
  }
}
</style>
