import { onBeforeUnmount, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  MEDIA_TRANSCODE_ESTIMATE_EVENT,
  MEDIA_TRANSCODE_PROGRESS_EVENT,
  type TranscodeJob,
  type TranscodeQueueSnapshot,
} from "@/modules/media-types";
import {
  transcodeJobCancel,
  transcodeJobRemove,
  transcodeQueueSnapshot,
} from "@/modules/transcodeCommands";

interface ProgressPayload {
  job_id: number;
  progress_percent: number;
  status: TranscodeJob["status"];
  output_size_bytes?: number | null;
  error_message?: string | null;
}

interface EstimatePayload {
  job_id: number;
  estimated_output_size_bytes: number;
}

export function useTranscodeQueue() {
  const jobs = ref<TranscodeJob[]>([]);
  const runningJobs = ref(0);
  const queuedJobs = ref(0);
  const unlisteners: UnlistenFn[] = [];

  function applySnapshot(snapshot: TranscodeQueueSnapshot) {
    jobs.value = snapshot.jobs;
    runningJobs.value = snapshot.running_jobs;
    queuedJobs.value = snapshot.queued_jobs;
  }

  async function refreshSnapshot() {
    applySnapshot(await transcodeQueueSnapshot());
  }

  async function registerEvents() {
    unlisteners.push(
      await listen<ProgressPayload>(MEDIA_TRANSCODE_PROGRESS_EVENT, (event) => {
        const payload = event.payload;
        const idx = jobs.value.findIndex((job) => job.id === payload.job_id);
        if (idx < 0) {
          void refreshSnapshot();
          return;
        }
        const current = jobs.value[idx];
        jobs.value[idx] = {
          ...current,
          progress_percent: payload.progress_percent,
          status: payload.status,
          output_size_bytes: payload.output_size_bytes ?? current.output_size_bytes,
          error_message: payload.error_message ?? current.error_message,
        };
      }),
    );
    unlisteners.push(
      await listen<EstimatePayload>(MEDIA_TRANSCODE_ESTIMATE_EVENT, (event) => {
        const payload = event.payload;
        const idx = jobs.value.findIndex((job) => job.id === payload.job_id);
        if (idx >= 0) {
          jobs.value[idx] = {
            ...jobs.value[idx],
            estimated_output_size_bytes: payload.estimated_output_size_bytes,
          };
        }
      }),
    );
  }

  async function cancelJob(jobId: number) {
    applySnapshot(await transcodeJobCancel(jobId));
  }

  async function removeJob(jobId: number) {
    applySnapshot(await transcodeJobRemove(jobId));
  }

  onBeforeUnmount(() => {
    unlisteners.forEach((dispose) => dispose());
    unlisteners.length = 0;
  });

  return {
    jobs,
    runningJobs,
    queuedJobs,
    applySnapshot,
    refreshSnapshot,
    registerEvents,
    cancelJob,
    removeJob,
  };
}
