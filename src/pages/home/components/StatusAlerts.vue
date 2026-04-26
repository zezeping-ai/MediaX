<script setup lang="ts">
defineProps<{
  cacheFinalizedOutputPath: string;
  recordingNoticeMessage: string;
  displayErrorMessage: string;
}>();
</script>

<template>
  <div class="contents">
    <a-alert
      v-if="cacheFinalizedOutputPath"
      class="absolute left-1/2 top-20 z-40 w-[min(620px,calc(100vw-32px))] -translate-x-1/2"
      type="success"
      :message="`缓存已生成：${cacheFinalizedOutputPath}`"
      show-icon
      banner
    />
    <a-alert
      v-if="recordingNoticeMessage"
      class="absolute left-1/2 top-4 z-40 w-[min(620px,calc(100vw-32px))] -translate-x-1/2"
      type="warning"
      :message="recordingNoticeMessage"
      show-icon
      banner
    />
    <a-alert
      v-if="displayErrorMessage"
      class="absolute left-1/2 top-16 z-40 w-[min(620px,calc(100vw-32px))] -translate-x-1/2"
      type="error"
      :message="displayErrorMessage"
      show-icon
      banner
    />
  </div>
</template>
