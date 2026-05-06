<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { Icon } from "@iconify/vue";
import { open } from "@tauri-apps/plugin-dialog";
import {
  revealFileInSystem,
  transcodeVideoEnqueue,
  transcodeVideoProbe,
} from "@/modules/transcodeCommands";
import { useTranscodeQueue } from "@/pages/transcode/composables/useTranscodeQueue";
import { useWindowSourceDrop } from "@/pages/transcode/composables/useWindowSourceDrop";
import { formatBytes } from "@/pages/transcode/utils";

const sourcePath = ref("");
const outputDir = ref("");
const format = ref("mp4/h264+aac");
const resolution = ref("source");
const playbackRate = ref(1);
const errorMessage = ref("");
const queue = useTranscodeQueue();
const speedOptions = [0.5, 0.75, 1, 1.25, 1.5, 2, 3, 5, 10, 20, 30, 50, 100] as const;
const baseResolutionOptions = [
  { value: "1080p", label: "1080p" },
  { value: "720p", label: "720p" },
  { value: "480p", label: "480p" },
  { value: "360p", label: "360p" },
  { value: "240p", label: "240p" },
  { value: "120p", label: "120p" },
] as const;
const resolutionOptions = ref<{ value: string; label: string; height: number }[]>([
  { value: "source", label: "源分辨率", height: Number.POSITIVE_INFINITY },
]);

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

async function syncResolutionOptionsBySource() {
  if (!sourcePath.value) {
    resolutionOptions.value = [{ value: "source", label: "源分辨率", height: Number.POSITIVE_INFINITY }];
    resolution.value = "source";
    return;
  }
  try {
    const probe = await transcodeVideoProbe(sourcePath.value);
    const filtered = baseResolutionOptions
      .filter((option) => Number.parseInt(option.value, 10) <= probe.height)
      .map((option) => ({
        value: option.value,
        label: option.label,
        height: Number.parseInt(option.value, 10),
      }));
    resolutionOptions.value = [
      { value: "source", label: `源分辨率 (${probe.width}x${probe.height})`, height: probe.height },
      ...filtered,
    ];
    if (!resolutionOptions.value.some((option) => option.value === resolution.value)) {
      resolution.value = "source";
    }
  } catch {
    // Keep UX robust: if probe fails we still allow source transcoding.
    resolutionOptions.value = [{ value: "source", label: "源分辨率", height: Number.POSITIVE_INFINITY }];
    resolution.value = "source";
  }
}

const jobs = computed(() => queue.jobs.value.filter((job) => job.kind === "video"));
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
    title: "选择视频源",
    multiple: false,
    filters: [{
      name: "视频文件",
      extensions: [
        "mp4", "mkv", "mov", "avi", "webm", "flv", "m4v",
        "wmv", "ts", "m2ts", "mpeg", "mpg", "3gp", "gif",
      ],
    }],
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
    errorMessage.value = "请先选择视频源和输出目录";
    return;
  }
  try {
    queue.applySnapshot(await transcodeVideoEnqueue({
      source_path: sourcePath.value,
      output_dir: outputDir.value,
      format: format.value,
      resolution: resolution.value,
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
  void syncResolutionOptionsBySource();
});
</script>

<template>
  <a-layout class="relative h-screen bg-[#0b1220] text-white">
    <a-layout-content class="flex h-full min-h-0 flex-col p-4">
      <div class="mb-3 flex items-center justify-between">
        <a-typography-title :level="4" class="mb-0! text-white!">视频转码</a-typography-title>
        <div class="text-xs text-white/70">
          进行中 {{ queue.runningJobs }} · 排队 {{ queue.queuedJobs }}
        </div>
      </div>

      <div class="mb-3 grid grid-cols-12 gap-3">
        <a-card class="col-span-8 border-white/10 bg-white/5" :body-style="{ padding: '12px' }">
          <a-form layout="vertical">
            <div class="grid grid-cols-12 gap-x-3 gap-y-2">
              <a-form-item class="col-span-12" label="视频源">
                <div class="flex w-full items-center gap-2">
                  <a-input :value="sourcePath" readonly placeholder="请选择视频源" class="min-w-0 flex-1" />
                  <a-button type="primary" @click="pickSource">选择</a-button>
                </div>
              </a-form-item>

              <a-form-item class="col-span-12" label="输出目录">
                <div class="flex w-full items-center gap-2">
                  <a-input :value="outputDir" readonly placeholder="默认与输入源同目录" class="min-w-0 flex-1" />
                  <a-button type="primary" @click="pickOutputDir">选择</a-button>
                </div>
              </a-form-item>

              <a-form-item class="col-span-6" label="转码格式">
                <a-select v-model:value="format" class="w-full">
                  <a-select-option value="mp4/h264+aac">MP4 / H264 + AAC</a-select-option>
                  <a-select-option value="mov/h264+aac">MOV / H264 + AAC</a-select-option>
                  <a-select-option value="mkv/h264+aac">MKV / H264 + AAC</a-select-option>
                  <a-select-option value="webm/vp9+opus">WebM / VP9 + Opus</a-select-option>
                  <a-select-option value="avi/mpeg4+mp3">AVI / MPEG4 + MP3</a-select-option>
                  <a-select-option value="flv/h264+aac">FLV / H264 + AAC</a-select-option>
                  <a-select-option value="m4v/h264+aac">M4V / H264 + AAC</a-select-option>
                  <a-select-option value="ts/h264+aac">TS / H264 + AAC</a-select-option>
                  <a-select-option value="m2ts/h264+aac">M2TS / H264 + AAC</a-select-option>
                  <a-select-option value="mpeg/mpeg2+mp2">MPEG / MPEG2 + MP2</a-select-option>
                  <a-select-option value="wmv/wmv2+wmav2">WMV / WMV2 + WMA</a-select-option>
                  <a-select-option value="gif/gif">GIF 动图</a-select-option>
                </a-select>
              </a-form-item>

              <a-form-item class="col-span-3" label="分辨率">
                <a-select v-model:value="resolution" class="w-full">
                  <a-select-option
                    v-for="option in resolutionOptions"
                    :key="option.value"
                    :value="option.value"
                  >
                    {{ option.label }}
                  </a-select-option>
                </a-select>
                <div class="mt-1 text-[11px] text-white/45">分辨率选项自动过滤</div>
              </a-form-item>

              <a-form-item class="col-span-3" label="倍速">
                <a-select v-model:value="playbackRate" class="w-full">
                  <a-select-option v-for="value in speedOptions" :key="String(value)" :value="value">
                    {{ value }}x
                  </a-select-option>
                </a-select>
              </a-form-item>

              <div class="col-span-12 flex items-center justify-between">
                <div class="text-xs text-rose-300">{{ errorMessage }}</div>
                <a-button type="primary" @click="submit">开始转码</a-button>
              </div>
            </div>
          </a-form>
        </a-card>

        <a-card class="col-span-4 border-white/10 bg-white/5" :body-style="{ padding: '12px' }">
          <a-typography-title :level="5" class="mb-2! text-white!">任务概览</a-typography-title>
          <div class="space-y-2 text-xs text-white/75">
            <div class="flex items-center justify-between"><span>运行中</span><span>{{ queue.runningJobs }}</span></div>
            <div class="flex items-center justify-between"><span>排队</span><span>{{ queue.queuedJobs }}</span></div>
            <div class="flex items-center justify-between"><span>总任务</span><span>{{ jobs.length }}</span></div>
            <div class="rounded bg-black/15 p-2 leading-5 text-white/65">
              侧重点：格式、分辨率与倍速参数组合。
            </div>
          </div>
        </a-card>
      </div>

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
      class="pointer-events-none absolute inset-4 z-20 flex items-center justify-center rounded-2xl border border-dashed border-sky-300/75 bg-black/45 text-sm text-white/90 backdrop-blur-sm"
    >
      拖拽视频文件到此处，自动填入输入源
    </div>
  </a-layout>
</template>
