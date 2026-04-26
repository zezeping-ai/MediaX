export interface DecodeBannerState {
  isHardware: boolean;
  mode: string;
  modeLabel: string;
  backend: string;
  error: string | null;
}

export interface DebugRow {
  key: string;
  label: string;
  value: string;
}

export interface DebugGroup {
  id: string;
  title: string;
  rows: DebugRow[];
}

export interface ProcessStage {
  id: string;
  label: string;
  status: "pending" | "active" | "completed" | "error";
  message: string;
  atMs: number | null;
  sinceStartMs: number | null;
  sincePrevMs: number | null;
}

export interface CurrentFrameSection {
  id: string;
  title: string;
  rows: DebugRow[];
}

export interface DebugSection {
  id: string;
  title: string;
  rows: DebugRow[];
}

export interface HardwareDecisionEvent {
  stage: string;
  label: string;
  message: string;
  atMs: number;
  tone: "neutral" | "good" | "warn" | "error";
}
