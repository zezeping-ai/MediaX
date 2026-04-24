const MEDIA_ERROR_TEXT: Record<string, string> = {
  INVALID_URL: "媒体地址无效，请检查 URL 或文件路径。",
  OPEN_FAILED: "媒体打开失败，请确认文件存在且格式受支持。",
  STREAM_START_FAILED: "媒体流启动失败，请重试或切换解码源。",
  DECODE_FAILED: "媒体解码失败，请检查媒体格式或尝试转码后播放。",
  UNSUPPORTED_FORMAT: "当前媒体格式暂不支持，请尝试转码后再播放。",
  NETWORK_ERROR: "网络连接异常，请检查网络状态后重试。",
  DECODE_ERROR: "媒体解码失败，可能是编码参数不兼容。",
  INTERNAL_ERROR: "播放器内部错误，请稍后重试。",
};

export function useMediaErrorMap() {
  function toUserErrorMessage(error: unknown) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    const normalized = rawMessage.trim();
    const [codeCandidate, detailCandidate] = normalized.split(":");
    const code = codeCandidate?.trim().toUpperCase();
    if (code && MEDIA_ERROR_TEXT[code]) {
      const detail = detailCandidate?.trim();
      return detail ? `${MEDIA_ERROR_TEXT[code]}（${detail}）` : MEDIA_ERROR_TEXT[code];
    }
    if (/url|uri|协议|protocol/i.test(normalized)) {
      return MEDIA_ERROR_TEXT.INVALID_URL;
    }
    if (/network|timeout|连接|dns|socket/i.test(normalized)) {
      return MEDIA_ERROR_TEXT.NETWORK_ERROR;
    }
    if (/decode|codec|demux|parse/i.test(normalized)) {
      return MEDIA_ERROR_TEXT.DECODE_ERROR;
    }
    return normalized || MEDIA_ERROR_TEXT.INTERNAL_ERROR;
  }

  return { toUserErrorMessage };
}
