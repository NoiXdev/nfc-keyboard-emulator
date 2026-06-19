import { describe, it, expect } from "vitest";
import { pushScan } from "./scanLog";
import type { ScanRecord } from "./tauri";

const rec = (uid: string): ScanRecord => ({
  timestamp: "2026-06-19T20:15:00Z",
  reader: "ACR122U",
  uidHex: uid,
  uidLength: 4,
  typed: true,
  typedString: `${uid}\n`,
  status: "ok",
});

describe("pushScan", () => {
  it("prepends newest first", () => {
    let log: ScanRecord[] = [];
    log = pushScan(log, rec("AA"), 10);
    log = pushScan(log, rec("BB"), 10);
    expect(log.map((r) => r.uidHex)).toEqual(["BB", "AA"]);
  });

  it("trims to capacity", () => {
    let log: ScanRecord[] = [];
    for (let i = 0; i < 12; i++) log = pushScan(log, rec(String(i)), 5);
    expect(log).toHaveLength(5);
    expect(log[0].uidHex).toBe("11");
  });
});
