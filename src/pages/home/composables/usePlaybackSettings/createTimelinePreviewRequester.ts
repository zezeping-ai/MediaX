import {
  DEFAULT_PREVIEW_FRAME_MAX_HEIGHT,
  DEFAULT_PREVIEW_FRAME_MAX_WIDTH,
} from "@/modules/media-player";
import type { PreviewFrame } from "@/modules/media-types";

type RequestPreviewFrame = (
  positionSeconds: number,
  maxWidth?: number,
  maxHeight?: number,
) => Promise<PreviewFrame | null>;

export function createTimelinePreviewRequester(requestPreviewFrame: RequestPreviewFrame) {
  function requestTimelinePreview(
    positionSeconds: number,
    maxWidth = DEFAULT_PREVIEW_FRAME_MAX_WIDTH,
    maxHeight = DEFAULT_PREVIEW_FRAME_MAX_HEIGHT,
  ) {
    return requestPreviewFrame(positionSeconds, maxWidth, maxHeight);
  }

  return {
    requestTimelinePreview,
  };
}
