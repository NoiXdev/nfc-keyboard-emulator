import type { ScanRecord } from "./tauri";

export function pushScan<T extends ScanRecord>(
  log: T[],
  record: T,
  capacity: number,
): T[] {
  const next = [record, ...log];
  return next.slice(0, Math.max(1, capacity));
}
