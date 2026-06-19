import type { ScanRecord } from "./tauri";

export function pushScan(
  log: ScanRecord[],
  record: ScanRecord,
  capacity: number,
): ScanRecord[] {
  const next = [record, ...log];
  return next.slice(0, Math.max(1, capacity));
}
