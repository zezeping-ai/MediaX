import { useLocalStorage } from "@vueuse/core";

export function createSourceHeightBaselineCache(storageKey: string) {
  const sourceHeightBaselineByPath = useLocalStorage<Record<string, number>>(
    storageKey,
    {},
  );

  function readCachedSourceHeightBaseline(path: string): number | null {
    if (!path) {
      return null;
    }
    const value = sourceHeightBaselineByPath.value[path];
    return typeof value === "number" && Number.isFinite(value) && value > 0 ? value : null;
  }

  function writeCachedSourceHeightBaseline(path: string, height: number) {
    if (!path || !Number.isFinite(height) || height <= 0) {
      return;
    }
    const nextHeight = Math.round(height);
    const prevHeight = sourceHeightBaselineByPath.value[path];
    sourceHeightBaselineByPath.value[path] =
      typeof prevHeight === "number" && Number.isFinite(prevHeight) && prevHeight > 0
        ? Math.max(prevHeight, nextHeight)
        : nextHeight;
  }

  return {
    readCachedSourceHeightBaseline,
    writeCachedSourceHeightBaseline,
  };
}
