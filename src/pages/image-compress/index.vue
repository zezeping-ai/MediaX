<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { Icon } from "@iconify/vue";
import { open } from "@tauri-apps/plugin-dialog";
import { imageCompressEnqueue, imageCompressEstimate, revealFileInSystem } from "@/modules/transcodeCommands";
import { useTranscodeQueue } from "@/pages/transcode/composables/useTranscodeQueue";
import { useWindowSourceDrop } from "@/pages/transcode/composables/useWindowSourceDrop";
import { formatBytes } from "@/pages/transcode/utils";

const mode = ref<"lossless" | "lossy">("lossless");
const lossyFormat = ref<"jpeg" | "webp" | "png" | "gif" | "bmp">("jpeg");
const quality = ref(75);
const sourcePaths = ref<string[]>([]);
const errorMessage = ref("");
const estimating = ref(false);
const estimatedInputBytes = ref<number | null>(null);
const estimatedOutputBytes = ref<number | null>(null);
let estimateSeq = 0;
const queue = useTranscodeQueue();

const jobs = computed(() => {
  const targetKind = mode.value === "lossless" ? "image_lossless" : "image_lossy";
  return queue.jobs.value.filter((job) => job.kind === targetKind);
});
const listRows = computed(() =>
  jobs.value.map((job) => ({
    id: job.id,
    source_path: job.source_path,
    progress_percent: job.progress_percent,
    output_size_bytes: job.output_size_bytes,
    status: job.status,
    filename: sourceFilename(job.source_path),
    saved_percent: savedPercent(job.status, job.input_size_bytes, job.output_size_bytes),
    format: job.format || null,
    quality: typeof job.quality === "number" ? job.quality : null,
    pending: false,
  })),
);
const lossyPendingRows = computed(() =>
  mode.value === "lossy"
    ? sourcePaths.value.map((path, idx) => ({
      id: `pending-${idx}-${path}`,
      source_path: path,
      progress_percent: 0,
      output_size_bytes: null,
      status: "queued" as const,
      filename: sourceFilename(path),
      saved_percent: null,
      format: lossyFormat.value,
      quality: quality.value,
      pending: true,
    }))
    : [],
);
const displayRows = computed(() =>
  mode.value === "lossy" ? [...lossyPendingRows.value, ...listRows.value] : listRows.value,
);
const estimatedSavedBytes = computed(() => {
  if (typeof estimatedInputBytes.value !== "number" || typeof estimatedOutputBytes.value !== "number") {
    return null;
  }
  return Math.max(0, estimatedInputBytes.value - estimatedOutputBytes.value);
});
const { dropActive } = useWindowSourceDrop({
  onDropPaths: async (paths) => {
    if (mode.value === "lossless") {
      await enqueueImages(paths, "lossless");
      return;
    }
    sourcePaths.value = paths;
  },
});

watch(
  [mode, sourcePaths, lossyFormat, quality],
  () => {
    void refreshLossyEstimate();
  },
  { deep: true, immediate: true },
);

async function pickSources() {
  errorMessage.value = "";
  try {
    const selected = await open({
      title: "选择图片",
      multiple: true,
      filters: [{ name: "图片文件", extensions: ["png", "jpg", "jpeg", "bmp", "webp", "gif", "tif", "tiff"] }],
    });
    if (!selected) {
      return;
    }
    const paths = Array.isArray(selected) ? selected.map(String) : [String(selected)];
    if (mode.value === "lossless") {
      await enqueueImages(paths, "lossless");
      return;
    }
    sourcePaths.value = paths;
  } catch (error) {
    // Some platforms/windows may reject filtered picker; fallback to generic picker.
    try {
      const selected = await open({
        title: "选择图片",
        multiple: true,
      });
      if (!selected) {
        return;
      }
      const paths = Array.isArray(selected) ? selected.map(String) : [String(selected)];
      if (mode.value === "lossless") {
        await enqueueImages(paths, "lossless");
        return;
      }
      sourcePaths.value = paths;
    } catch (fallbackError) {
      errorMessage.value = fallbackError instanceof Error
        ? fallbackError.message
        : String(fallbackError || error);
    }
  }
}

async function submit() {
  errorMessage.value = "";
  if (!sourcePaths.value.length) {
    errorMessage.value = "请先选择图片";
    return;
  }
  await enqueueImages(sourcePaths.value, mode.value);
}

async function enqueueImages(paths: string[], nextMode: "lossless" | "lossy") {
  if (!paths.length) {
    return;
  }
  try {
    queue.applySnapshot(await imageCompressEnqueue({
      source_paths: paths,
      mode: nextMode,
      format: nextMode === "lossy" ? lossyFormat.value : undefined,
      quality: nextMode === "lossy" ? quality.value : undefined,
    }));
    if (nextMode === "lossy") {
      sourcePaths.value = [];
    }
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
}

async function refreshLossyEstimate() {
  if (mode.value !== "lossy" || sourcePaths.value.length === 0) {
    estimating.value = false;
    estimatedInputBytes.value = null;
    estimatedOutputBytes.value = null;
    return;
  }
  const seq = ++estimateSeq;
  estimating.value = true;
  try {
    const result = await imageCompressEstimate({
      source_paths: sourcePaths.value,
      mode: "lossy",
      format: lossyFormat.value,
      quality: quality.value,
    });
    if (seq !== estimateSeq) {
      return;
    }
    estimatedInputBytes.value = result.total_input_size_bytes;
    estimatedOutputBytes.value = result.estimated_output_size_bytes;
  } catch {
    if (seq !== estimateSeq) {
      return;
    }
    estimatedInputBytes.value = null;
    estimatedOutputBytes.value = null;
  } finally {
    if (seq === estimateSeq) {
      estimating.value = false;
    }
  }
}

function savedPercent(
  status?: "queued" | "running" | "success" | "skipped" | "failed" | "canceled",
  input?: number | null,
  output?: number | null,
) {
  if (status !== "success") {
    return null;
  }
  if (typeof input !== "number" || typeof output !== "number") {
    return null;
  }
  if (!Number.isFinite(input) || !Number.isFinite(output) || input <= 0 || output < 0) {
    return null;
  }
  const ratio = ((input - output) / input) * 100;
  return Math.max(0, ratio);
}

function sourceFilename(path: string) {
  const normalized = (path || "").split("\\").join("/");
  const segment = normalized.split("/").pop();
  return segment || path || "-";
}

async function revealSourcePath(path: string) {
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
</script>

<template>
  <a-layout class="relative h-screen bg-[#0b1220] text-white">
    <a-layout-content class="flex h-full min-h-0 flex-col p-4">
      <div class="mb-2 flex items-center justify-between">
        <a-typography-title :level="4" class="mb-0! text-white!">图片压缩</a-typography-title>
        <div class="text-xs text-white/70">
          进行中 {{ queue.runningJobs }} · 排队 {{ queue.queuedJobs }}
        </div>
      </div>

      <a-card class="mb-3 border-white/10 bg-white/5" :body-style="{ padding: '12px' }">
        <a-tabs v-model:activeKey="mode" size="small" class="mb-2">
          <a-tab-pane key="lossless" tab="无损压缩" />
          <a-tab-pane key="lossy" tab="有损压缩" />
        </a-tabs>
        <div
          class="mx-auto w-full cursor-pointer rounded-xl border border-dashed border-white/20 bg-black/15 p-4 text-center transition-colors hover:border-white/35"
          role="button"
          tabindex="0"
          @click="pickSources"
          @keydown.enter.prevent="pickSources"
          @keydown.space.prevent="pickSources"
        >
          <a-typography-title :level="5" class="mb-1! text-white!">
            {{ mode === "lossless" ? "无损压缩（同目录输出）" : "有损压缩（同目录输出）" }}
          </a-typography-title>
          <div class="mb-3 text-xs text-white/65">
            输出默认与原图同目录，文件名自动追加 `-compressed`
          </div>
          <div class="mx-auto mb-2 text-2xl leading-none text-white/85">
            +
          </div>
          <div class="text-sm text-white/80">点击或拖拽图片到这里</div>
          <div v-if="mode === 'lossy'" class="mt-3 flex justify-center" @click.stop @keydown.stop>
            <a-form layout="inline" class="flex flex-col items-center gap-1">
              <a-form-item label="格式" class="mb-0!">
                <a-select
                  v-model:value="lossyFormat"
                  :options="[
                    { label: 'JPEG (.jpg)', value: 'jpeg' },
                    { label: 'WebP (.webp)', value: 'webp' },
                    { label: 'PNG (.png)', value: 'png' },
                    { label: 'GIF (.gif)', value: 'gif' },
                    { label: 'BMP (.bmp)', value: 'bmp' },
                  ]"
                  style="width: 140px"
                />
              </a-form-item>
              <a-form-item label="质量" class="mb-0!">
                <div class="flex items-center">
                  <div class="w-[220px]">
                    <a-slider v-model:value="quality" :min="1" :max="100" :step="1" class="my-1!" />
                  </div>
                  <span class="ml-2 w-[32px] text-left text-xs leading-none text-white/80">{{ quality }}</span>
                </div>
              </a-form-item>
            </a-form>
          </div>
          <div v-if="mode === 'lossy'" class="mt-2 text-xs text-white/70">
            <template v-if="sourcePaths.length > 0">
              预计输入 {{ formatBytes(estimatedInputBytes) }}
              · 预计输出 {{ estimating ? '计算中...' : formatBytes(estimatedOutputBytes) }}
              · 预计节省 {{ estimating ? '计算中...' : formatBytes(estimatedSavedBytes) }}
            </template>
            <template v-else>
              选择图片后可实时预估压缩结果
            </template>
          </div>
          <div class="mt-2 text-xs text-rose-300">{{ errorMessage }}</div>
        </div>
      </a-card>

      <a-card class="min-h-0 flex-1 border-white/10 bg-white/5" :body-style="{ padding: '12px', height: '100%' }">
        <div class="h-full min-h-0 overflow-y-auto overflow-x-hidden">
          <div v-if="mode === 'lossy'" class="mb-2 flex items-center justify-end">
            <a-button type="primary" :disabled="sourcePaths.length === 0" @click="submit">
              开始执行
            </a-button>
          </div>
          <div class="flex items-center gap-2 border-b border-white/10 px-2 pb-2 text-xs text-white/55">
            <div class="flex-2 min-w-0">文件名</div>
            <div class="flex-2 min-w-0">进度</div>
            <div v-if="mode === 'lossy'" class="w-[88px] text-right">输出格式</div>
            <div v-if="mode === 'lossy'" class="w-[72px] text-right">质量</div>
            <div class="flex-1 min-w-0 text-right">节省比例</div>
            <div class="flex-1 min-w-0 text-right">压缩后大小</div>
          </div>
          <div v-if="displayRows.length === 0" class="flex h-full min-h-[220px] items-center justify-center">
            <a-empty description="暂无任务" />
          </div>
          <div v-else class="space-y-1 pt-2">
            <div
              v-for="record in displayRows"
              :key="record.id"
              class="flex items-center gap-2 rounded-md px-2 py-2 text-xs text-white/80 hover:bg-white/5"
            >
              <div class="flex-2 min-w-0 text-white/85" :title="record.source_path">
                <div class="flex h-6 items-center gap-1">
                  <span class="truncate">{{ record.filename }}</span>
                  <a-tooltip title="打开所在位置">
                    <a-button
                      type="text"
                      size="small"
                      class="h-[20px] min-w-[20px] p-0 text-white/70 hover:text-white!"
                      @click="revealSourcePath(record.source_path)"
                    >
                      <Icon icon="mdi:folder-open-outline" />
                    </a-button>
                  </a-tooltip>
                </div>
              </div>
              <div class="flex-2 min-w-0">
                <div class="flex h-6 items-center gap-2">
                  <span v-if="record.pending" class="text-xs text-amber-300">待开始</span>
                  <a-progress
                    v-else-if="record.status === 'running' || record.status === 'queued'"
                    class="w-[120px] mb-0!"
                    :percent="Number(record.progress_percent.toFixed(1))"
                    size="small"
                    :show-info="false"
                    status="normal"
                  />
                  <a-progress
                    v-else
                    class="w-[120px] mb-0!"
                    :percent="100"
                    size="small"
                    :show-info="true"
                    :status="record.status === 'failed' ? 'exception' : record.status === 'success' ? 'success' : 'normal'"
                  />
                </div>
              </div>
              <div v-if="mode === 'lossy'" class="w-[88px] text-right tabular-nums text-white/75">
                {{ (record.format || "-").toUpperCase() }}
              </div>
              <div v-if="mode === 'lossy'" class="w-[72px] text-right tabular-nums text-white/75">
                {{ typeof record.quality === "number" ? record.quality : "-" }}
              </div>
              <div class="flex-1 min-w-0 text-right tabular-nums text-white/75">
                {{ typeof record.saved_percent === "number" ? `${record.saved_percent.toFixed(1)}%` : "-" }}
              </div>
              <div class="flex-1 min-w-0 text-right tabular-nums text-white/75">
                {{ formatBytes(record.output_size_bytes) }}
              </div>
            </div>
          </div>
        </div>
      </a-card>
    </a-layout-content>
    <div
      v-if="dropActive"
      class="pointer-events-none absolute inset-4 z-20 flex items-center justify-center rounded-2xl border border-dashed border-violet-300/75 bg-black/45 text-sm text-white/90 backdrop-blur-sm"
    >
      拖拽图片到此处，自动加入压缩列表
    </div>
  </a-layout>
</template>
