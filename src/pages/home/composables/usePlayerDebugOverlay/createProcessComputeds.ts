import { computed, type Ref } from "vue";
import type { HardwareDecisionEvent, ProcessStage } from "./types";
import { formatHardwareDecisionLabel, resolveHardwareDecisionTone } from "./utils";

type DebugTimelineEntry = { stage: string; message: string; at_ms: number };
type DebugStageSnapshotEntry = { message: string; at_ms: number };

export function createProcessComputeds(
  debugTimeline?: Ref<DebugTimelineEntry[]>,
  debugStageSnapshot?: Ref<Record<string, DebugStageSnapshotEntry>>,
) {
  const hardwareDecisionTimeline = computed((): HardwareDecisionEvent[] => {
    const timeline = debugTimeline?.value ?? [];
    const interestingStages = new Set([
      "open",
      "decoder_ready",
      "hw_decode_decision",
      "hw_decode_fallback",
      "decode_error",
    ]);
    return timeline
      .filter((item) => interestingStages.has(item.stage))
      .slice(-16)
      .map((item) => ({
        stage: item.stage,
        label: formatHardwareDecisionLabel(item.stage),
        message: item.message,
        atMs: item.at_ms,
        tone: resolveHardwareDecisionTone(item.stage, item.message),
      }));
  });

  const processStages = computed((): ProcessStage[] => {
    const latestByStage = new Map<string, { message: string; atMs: number }>();
    const stageSnapshot = debugStageSnapshot?.value ?? {};
    let hasError = false;
    for (const [stage, entry] of Object.entries(stageSnapshot)) {
      latestByStage.set(stage, { message: entry.message, atMs: entry.at_ms });
      if (stage.includes("error")) {
        hasError = true;
      }
    }
    if (!latestByStage.size) {
      const timeline = debugTimeline?.value ?? [];
      for (const item of timeline) {
        latestByStage.set(item.stage, { message: item.message, atMs: item.at_ms });
        if (item.stage.includes("error")) {
          hasError = true;
        }
      }
    }
    const stageDefs = [
      { id: "open", label: "打开源", aliases: ["open"] },
      { id: "stream_start", label: "启动流", aliases: ["stream_start"] },
      { id: "metadata_ready", label: "元数据", aliases: ["metadata_ready", "video_format", "video_stream"] },
      { id: "audio_pipeline_ready", label: "音频管线", aliases: ["audio", "audio_format", "audio_pipeline_ready"] },
      { id: "decoder_ready", label: "解码器", aliases: ["decoder_ready", "video_codec_profile"] },
      { id: "running", label: "进入播放", aliases: ["running"] },
      { id: "first_frame", label: "首帧输出", aliases: ["first_frame", "video_frame_format"] },
    ] as const;

    let activeAssigned = false;
    const hasFirstFrame = stageDefs
      .find((stage) => stage.id === "first_frame")
      ?.aliases.some((alias) => latestByStage.has(alias)) ?? false;

    const stages = stageDefs.map((stage) => {
      const matched = stage.aliases
        .map((alias) => latestByStage.get(alias))
        .filter((value): value is { message: string; atMs: number } => Boolean(value))
        .sort((a, b) => b.atMs - a.atMs)[0];
      if (matched) {
        return {
          id: stage.id,
          label: stage.label,
          status: hasError && stage.id === "first_frame" && !hasFirstFrame ? "error" : "completed",
          message: matched.message,
          atMs: matched.atMs,
          sinceStartMs: null,
          sincePrevMs: null,
        } satisfies ProcessStage;
      }
      if (!activeAssigned && !hasFirstFrame) {
        activeAssigned = true;
        return {
          id: stage.id,
          label: stage.label,
          status: hasError ? "error" : "active",
          message: hasError ? "流程中断或等待恢复" : "等待执行",
          atMs: null,
          sinceStartMs: null,
          sincePrevMs: null,
        } satisfies ProcessStage;
      }
      return {
        id: stage.id,
        label: stage.label,
        status: "pending",
        message: "等待执行",
        atMs: null,
        sinceStartMs: null,
        sincePrevMs: null,
      } satisfies ProcessStage;
    });
    const visibleStages = stages.filter((stage) => stage.status !== "pending");
    const firstAtMs = visibleStages.find((stage) => stage.atMs !== null)?.atMs ?? null;
    let prevAtMs: number | null = null;
    const stagesWithTiming = visibleStages.map((stage) => {
      const sinceStartMs =
        firstAtMs !== null && stage.atMs !== null ? Math.max(0, stage.atMs - firstAtMs) : null;
      const sincePrevMs =
        prevAtMs !== null && stage.atMs !== null ? Math.max(0, stage.atMs - prevAtMs) : null;
      if (stage.atMs !== null) {
        prevAtMs = stage.atMs;
      }
      return {
        ...stage,
        sinceStartMs,
        sincePrevMs,
      } satisfies ProcessStage;
    });
    return stagesWithTiming.length ? stagesWithTiming : stages.slice(0, 1);
  });

  return {
    hardwareDecisionTimeline,
    processStages,
  };
}
