export function formatSeconds(value: number) {
  const safeValue = Math.max(0, Math.floor(value || 0));
  const minutes = Math.floor(safeValue / 60);
  const seconds = safeValue % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}
