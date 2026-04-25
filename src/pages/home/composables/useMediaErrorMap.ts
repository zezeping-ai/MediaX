const MEDIA_ERROR_TEXT: Record<string, string> = {
  INVALID_INPUT: "输入参数无效，请检查播放参数后重试。",
  INTERNAL: "播放器内部错误，请稍后重试。",
  INVALID_URL: "媒体地址无效，请检查 URL 或文件路径。",
  OPEN_FAILED: "媒体打开失败，请确认文件存在且格式受支持。",
  STREAM_START_FAILED: "媒体流启动失败，请重试或切换解码源。",
  STATE_POISONED: "播放器状态异常，请停止后重试。",
  DECODE_FAILED: "媒体解码失败，请检查媒体格式或尝试转码后播放。",
  UNSUPPORTED_FORMAT: "当前媒体格式暂不支持，请尝试转码后再播放。",
  NETWORK_ERROR: "网络连接异常，请检查网络状态后重试。",
  DECODE_ERROR: "媒体解码失败，可能是编码参数不兼容。",
  INTERNAL_ERROR: "播放器内部错误，请稍后重试。",
};

type ParsedMediaError = {
  code: string | null;
  detail: string;
};

function parseMediaError(error: unknown): ParsedMediaError {
  const rawMessage = error instanceof Error ? error.message : String(error);
  const normalized = rawMessage.trim();
  const separatorIndex = normalized.indexOf(":");
  if (separatorIndex > 0) {
    const code = normalized.slice(0, separatorIndex).trim().toUpperCase();
    const detail = normalized.slice(separatorIndex + 1).trim();
    return { code, detail };
  }
  return { code: null, detail: normalized };
}

export function toUserMediaErrorMessage(error: unknown) {
  const { code, detail } = parseMediaError(error);
  if (code && MEDIA_ERROR_TEXT[code]) {
    return detail ? `${MEDIA_ERROR_TEXT[code]}（${detail}）` : MEDIA_ERROR_TEXT[code];
  }
  if (/url|uri|协议|protocol/i.test(detail)) {
    return MEDIA_ERROR_TEXT.INVALID_URL;
  }
  if (/network|timeout|连接|dns|socket/i.test(detail)) {
    return MEDIA_ERROR_TEXT.NETWORK_ERROR;
  }
  if (/decode|codec|demux|parse/i.test(detail)) {
    return MEDIA_ERROR_TEXT.DECODE_ERROR;
  }
  if (/poisoned|lock poisoned|state lock/i.test(detail)) {
    return MEDIA_ERROR_TEXT.STATE_POISONED;
  }
  return detail || MEDIA_ERROR_TEXT.INTERNAL_ERROR;
}

export function useMediaErrorMap() {
  function toUserErrorMessage(error: unknown) {
    return toUserMediaErrorMessage(error);
  }
  return { toUserErrorMessage };
}
