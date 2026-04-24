import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type MediaProcessingTaskType = "transcode" | "concat";

export type MediaProcessingTaskStatus = "queued" | "running" | "completed" | "failed" | "canceled";

export type MediaProcessingTaskPayload = {
  type: MediaProcessingTaskType;
  inputPaths: string[];
  outputPath: string;
  options?: Record<string, unknown>;
};

export type MediaProcessingTask = {
  taskId: string;
  type: MediaProcessingTaskType;
  status: MediaProcessingTaskStatus;
  progress: number;
  error: string | null;
};

export type MediaProcessingTaskUpdateEvent = {
  task: MediaProcessingTask;
};

export const MEDIA_PROCESSING_TASK_UPDATED_EVENT = "media-processing://task-updated";

/**
 * 统一媒体处理能力入口，便于后续将转码/拼接扩展为完整任务系统。
 * 当前前端仅定义协议层，后端实现可按命令逐步接入。
 */
export function createMediaProcessingTask(payload: MediaProcessingTaskPayload) {
  return invoke<MediaProcessingTask>("media_processing_create_task", { payload });
}

export function getMediaProcessingTask(taskId: string) {
  return invoke<MediaProcessingTask>("media_processing_get_task", { taskId });
}

export function cancelMediaProcessingTask(taskId: string) {
  return invoke<MediaProcessingTask>("media_processing_cancel_task", { taskId });
}

/**
 * 任务进度事件订阅：用于转码/拼接进度条、失败提示、历史任务列表同步。
 */
export function onMediaProcessingTaskUpdated(
  handler: (event: MediaProcessingTaskUpdateEvent) => void,
): Promise<UnlistenFn> {
  return listen<MediaProcessingTaskUpdateEvent>(MEDIA_PROCESSING_TASK_UPDATED_EVENT, (event) => {
    handler(event.payload);
  });
}
