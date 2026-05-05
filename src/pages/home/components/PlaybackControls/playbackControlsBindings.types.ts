export type TimelineEventMap = {
  preview: (value: number | [number, number]) => void;
  commit: (value: number | [number, number]) => void;
};
