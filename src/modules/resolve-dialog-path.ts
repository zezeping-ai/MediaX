/** Normalize dialog `open()` results into a local filesystem path for FFmpeg/Rust. */
export function resolveDialogPath(selection: unknown): string | null {
  if (selection == null) {
    return null;
  }
  if (Array.isArray(selection)) {
    return resolveDialogPath(selection[0]);
  }
  if (typeof selection === "object") {
    const record = selection as Record<string, unknown>;
    if (typeof record.path === "string") {
      return resolveDialogPath(record.path);
    }
  }
  const raw = String(selection).trim();
  if (!raw) {
    return null;
  }
  if (raw.startsWith("file://")) {
    try {
      return decodeURIComponent(new URL(raw).pathname);
    } catch {
      return raw;
    }
  }
  return raw;
}
