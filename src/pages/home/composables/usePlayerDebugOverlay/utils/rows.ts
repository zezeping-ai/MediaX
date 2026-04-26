import type { DebugRow } from "../types";

export function pushSnapshotRow(
  rows: DebugRow[],
  snapshot: Record<string, string>,
  key: string,
  label = key,
) {
  const value = snapshot[key];
  if (!value) {
    return;
  }
  rows.push({ key, label, value });
}
