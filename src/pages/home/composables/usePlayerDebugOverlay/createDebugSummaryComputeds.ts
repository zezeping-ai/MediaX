import { computed, type Ref } from "vue";
import type { PlaybackState } from "@/modules/media-types";
import { DEBUG_GROUP_ORDER, PREFERRED_DEBUG_ORDER } from "./constants";
import type { DebugGroup, DebugRow, DecodeBannerState } from "./types";
import { detectDebugGroup, formatDebugLabel, formatGroupTitle, formatHwModeLabel } from "./utils";

export function createDebugSummaryComputeds(
  playback: Ref<PlaybackState | null>,
  debugSnapshot: Ref<Record<string, string>>,
) {
  const decodeBanner = computed((): DecodeBannerState | null => {
    const state = playback.value;
    if (!state) {
      return null;
    }
    const mode = state.hw_decode_mode || "auto";
    return {
      isHardware: state.hw_decode_active,
      mode,
      modeLabel: formatHwModeLabel(mode),
      backend: state.hw_decode_backend || "—",
      error: state.hw_decode_error,
    };
  });

  const resourceSummary = computed(() => debugSnapshot.value.telemetry_resources || "");

  const debugRows = computed(() => {
    const snapshot = debugSnapshot.value;
    const rows: DebugRow[] = [];
    for (const key of PREFERRED_DEBUG_ORDER) {
      const value = snapshot[key];
      if (!value) continue;
      rows.push({ key, label: formatDebugLabel(key), value });
    }
    for (const [key, value] of Object.entries(snapshot)) {
      if (key === "hw_decode" || key === "telemetry_resources" || !value) continue;
      if (PREFERRED_DEBUG_ORDER.includes(key as (typeof PREFERRED_DEBUG_ORDER)[number])) continue;
      rows.push({ key, label: formatDebugLabel(key), value });
    }
    if (!rows.length) {
      return [{ key: "empty", label: "status", value: "等待解析信息..." }];
    }
    return rows;
  });

  const debugGroups = computed((): DebugGroup[] => {
    const bucketMap = new Map<string, DebugRow[]>();
    for (const id of DEBUG_GROUP_ORDER) {
      bucketMap.set(id, []);
    }
    for (const row of debugRows.value) {
      const groupId = detectDebugGroup(row.key);
      bucketMap.get(groupId)?.push(row);
    }
    return DEBUG_GROUP_ORDER.map((id) => ({
      id,
      title: formatGroupTitle(id),
      rows: bucketMap.get(id) ?? [],
    })).filter((group) => group.rows.length > 0);
  });

  return {
    decodeBanner,
    resourceSummary,
    debugGroups,
  };
}
