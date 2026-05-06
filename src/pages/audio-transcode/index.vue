<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { Icon } from "@iconify/vue";
import { open } from "@tauri-apps/plugin-dialog";
import { revealFileInSystem, transcodeAudioEnqueue } from "@/modules/transcodeCommands";
import { useTranscodeQueue } from "@/pages/transcode/composables/useTranscodeQueue";
import { useWindowSourceDrop } from "@/pages/transcode/composables/useWindowSourceDrop";
import { formatBytes } from "@/pages/transcode/utils";

const sourcePath = ref("");
const outputDir = ref("");
const format = ref("m4a/aac");
const playbackRate = ref(1);
const errorMessage = ref("");
const queue = useTranscodeQueue();

function deriveOutputDirFromSource(path: string) {
  const normalized = path.trim();
  if (!normalized) {
    return "";
  }
  const slashIndex = Math.max(normalized.lastIndexOf("/"), normalized.lastIndexOf("\\"));
  if (slashIndex <= 0) {
    return "";
  }
  return normalized.slice(0, slashIndex);
}

const jobs = computed(() => queue.jobs.value.filter((job) => job.kind === "audio"));
const statusTextMap = {
  queued: "排队",
  running: "进行中",
  success: "完成",
  skipped: "跳过",
  failed: "失败",
  canceled: "已取消",
} as const;
const { dropActive } = useWindowSourceDrop({
  onDropPaths: async (paths) => {
    const [firstPath] = paths;
    if (firstPath) {
      sourcePath.value = firstPath;
    }
  },
});

async function pickSource() {
  const selected = await open({
    title: "选择音频源",
    multiple: false,
    filters: [{ name: "音频文件", extensions: ["mp3", "aac", "m4a", "wav", "flac", "ogg"] }],
  });
  if (selected && !Array.isArray(selected)) {
    sourcePath.value = String(selected);
  }
}

async function pickOutputDir() {
  const selected = await open({
    title: "选择输出目录",
    directory: true,
    multiple: false,
  });
  if (selected && !Array.isArray(selected)) {
    outputDir.value = String(selected);
  }
}

async function submit() {
  errorMessage.value = "";
  if (!sourcePath.value || !outputDir.value) {
    errorMessage.value = "请先选择音频源和输出目录";
    return;
  }
  try {
    queue.applySnapshot(await transcodeAudioEnqueue({
      source_path: sourcePath.value,
      output_dir: outputDir.value,
      format: format.value,
      playback_rate: playbackRate.value || 1,
    }));
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
}

async function revealOutputPath(path: string) {
  errorMessage.value = "";
  try {
    await revealFileInSystem(path);
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
}

onMounted(async () => {
  await queue.refreshSnapshot();
  await queue.registerEvents();
});

watch(sourcePath, () => {
  outputDir.value = deriveOutputDirFromSource(sourcePath.value);
});
</script>

<template>
  <a-layout class="relative h-screen bg-[#0b1220] text-white">
    <a-layout-content class="flex h-full min-h-0 flex-col p-4">
      <div class="mb-3 flex items-center justify-between">
        <a-typography-title :level="4" class="mb-0! text-white!">音频转码</a-typography-title>
        <div class="text-xs text-white/70">
          进行中 {{ queue.runningJobs }} · 排队 {{ queue.queuedJobs }}
        </div>
      </div>

      <a-card class="mb-3 border-white/10 bg-white/5" :body-style="{ padding: '12px' }">
        <div class="flex flex-wrap items-end gap-3">
          <div class="min-w-[280px] flex-1">
            <div class="mb-1 text-xs text-white/70">音频源</div>
            <a-space>
              <a-input :value="sourcePath" readonly placeholder="请选择音频源" class="w-[360px]" />
              <a-button type="primary" @click="pickSource">选择</a-button>
            </a-space>
          </div>
          <div class="min-w-[280px] flex-1">
            <div class="mb-1 text-xs text-white/70">输出目录</div>
            <a-space>
              <a-input :value="outputDir" readonly placeholder="默认与输入源同目录" class="w-[360px]" />
              <a-button type="primary" @click="pickOutputDir">选择</a-button>
            </a-space>
          </div>
          <div class="w-[170px]">
            <div class="mb-1 text-xs text-white/70">格式</div>
            <a-select v-model:value="format" class="w-full">
              <a-select-option value="m4a/aac">M4A / AAC</a-select-option>
              <a-select-option value="mp3/mp3">MP3 / MP3</a-select-option>
            </a-select>
          </div>
          <div class="w-[120px]">
            <div class="mb-1 text-xs text-white/70">倍速</div>
            <a-input-number v-model:value="playbackRate" :min="0.5" :max="4" :step="0.1" class="w-full" />
          </div>
          <div class="ml-auto">
            <a-button type="primary" @click="submit">加入队列</a-button>
          </div>
        </div>
        <div class="mt-2 text-xs text-rose-300">{{ errorMessage }}</div>
      </a-card>

      <a-card class="min-h-0 flex-1 border-white/10 bg-white/5" :body-style="{ padding: '12px', height: '100%' }">
        <div class="h-full overflow-auto space-y-2 pr-1">
          <div
            v-for="job in jobs"
            :key="job.id"
            class="rounded-lg border border-white/10 bg-black/10 p-3"
          >
            <div class="mb-2 flex items-center justify-between">
              <div class="text-sm font-medium text-white/90">
                #{{ job.id }} · {{ statusTextMap[job.status] }}
              </div>
              <div class="text-xs text-white/70">
                输入 {{ formatBytes(job.input_size_bytes) }} · 输出 {{ formatBytes(job.output_size_bytes) }}
              </div>
            </div>
            <a-progress
              v-if="job.status === 'running' || job.status === 'queued'"
              :percent="Number(job.progress_percent.toFixed(1))"
              size="small"
              :show-info="false"
              status="normal"
            />
            <a-progress
              v-else
              :percent="100"
              size="small"
              :show-info="true"
              :status="job.status === 'failed' ? 'exception' : job.status === 'success' ? 'success' : 'normal'"
            />
            <div class="mt-2 flex items-center justify-between gap-3">
              <div class="min-w-0 text-xs text-white/60">
                <div class="flex items-center gap-1">
                  <span class="truncate" :title="job.output_path">{{ job.output_path }}</span>
                  <a-tooltip title="打开所在位置">
                    <a-button
                      type="text"
                      size="small"
                      class="h-[20px] min-w-[20px] p-0 text-white/70 hover:text-white!"
                      @click="revealOutputPath(job.output_path)"
                    >
                      <Icon icon="mdi:folder-open-outline" />
                    </a-button>
                  </a-tooltip>
                </div>
              </div>
              <a-space>
                <a-button size="small" @click="queue.cancelJob(job.id)">取消</a-button>
                <a-button size="small" danger @click="queue.removeJob(job.id)">移除</a-button>
              </a-space>
            </div>
          </div>
          <div v-if="jobs.length === 0" class="flex h-full min-h-[240px] items-center justify-center">
            <a-empty description="暂无任务" />
          </div>
        </div>
      </a-card>
    </a-layout-content>
    <div
      v-if="dropActive"
      class="pointer-events-none absolute inset-4 z-20 flex items-center justify-center rounded-2xl border border-dashed border-emerald-300/75 bg-black/45 text-sm text-white/90 backdrop-blur-sm"
    >
      拖拽音频文件到此处，自动填入输入源
    </div>
  </a-layout>
</template>
