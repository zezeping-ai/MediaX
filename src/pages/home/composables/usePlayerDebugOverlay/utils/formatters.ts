import { DEBUG_LABELS } from "../constants";

export function formatBytesPerSecond(value: number): string {
  if (!Number.isFinite(value) || value <= 0) {
    return "0 B/s";
  }
  const units = ["B/s", "KB/s", "MB/s", "GB/s"];
  let size = value;
  let unitIndex = 0;
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }
  return `${size.toFixed(size >= 100 ? 0 : size >= 10 ? 1 : 2)} ${units[unitIndex]}`;
}

export function formatHardwareDecisionLabel(stage: string): string {
  switch (stage) {
    case "open":
      return "打开源";
    case "decoder_ready":
      return "解码器";
    case "hw_decode_decision":
      return "硬解决策";
    case "hw_decode_fallback":
      return "硬解回退";
    case "decode_error":
      return "解码错误";
    default:
      return stage;
  }
}

export function formatDebugLabel(key: string): string {
  return DEBUG_LABELS[key] || key;
}

export function formatHwModeLabel(mode: string): string {
  switch (mode) {
    case "auto":
      return "自动";
    case "on":
      return "硬解优先";
    case "off":
      return "仅软解";
    default:
      return mode;
  }
}

export function formatGroupTitle(groupId: string): string {
  switch (groupId) {
    case "input":
      return "输入/流";
    case "stream":
      return "流信息";
    case "decode":
      return "解码";
    case "video":
      return "视频";
    case "audio":
      return "音频";
    case "timing":
      return "时序/性能";
    case "error":
      return "异常";
    default:
      return "其他";
  }
}
