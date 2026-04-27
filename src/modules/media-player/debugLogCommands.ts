import { invokeMediaCommandValidated } from "../media-command";

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function isBoolean(value: unknown): value is boolean {
  return typeof value === "boolean";
}

export function playbackGetDebugLogPath() {
  return invokeMediaCommandValidated<string>(
    "playback_get_debug_log_path",
    isString,
  );
}

export function playbackClearDebugLog() {
  return invokeMediaCommandValidated<string>(
    "playback_clear_debug_log",
    isString,
  );
}

export function playbackSetDebugLogEnabled(enabled: boolean) {
  return invokeMediaCommandValidated<boolean>(
    "playback_set_debug_log_enabled",
    isBoolean,
    { enabled },
  );
}
