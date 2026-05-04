/** Normalize paths from `dialog.open()` / shell for `playback_open_source`. */
export function normalizeLocalPlaybackPath(raw: string): string {
  const t = raw.trim();
  if (!t.startsWith("file:")) {
    return t;
  }
  try {
    const u = new URL(t);
    let p = u.pathname;
    // Windows: file:///C:/Users/... → pathname /C:/Users/...
    if (/^\/[A-Za-z]:\//.test(p)) {
      p = p.slice(1);
    }
    return decodeURIComponent(p);
  } catch {
    return t;
  }
}
