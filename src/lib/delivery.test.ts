import { describe, it, expect } from "vitest";
import { applyDelivery, deliveryErrorText, withPendingDelivery, type LoggedScan } from "./delivery";
import { i18n } from "../i18n";
import type { DeliveryError, ScanRecord } from "./tauri";

const rec = (scanId: string, deliveryTargets: ScanRecord["deliveryTargets"]): ScanRecord => ({
  timestamp: "2026-06-19T20:15:00Z",
  reader: "ACR122U",
  uidHex: "04A1B2C3",
  uidLength: 4,
  typed: true,
  typedString: "04A1B2C3\n",
  status: "ok",
  scanId,
  uidFormatted: "04A1B2C3",
  requestId: null,
  deliveryTargets,
});

describe("withPendingDelivery", () => {
  it("creates one pending entry per delivery target", () => {
    const row = withPendingDelivery(rec("a", ["http", "ws"]));
    expect(row.delivery).toEqual({ http: { state: "pending" }, ws: { state: "pending" } });
    expect(row.scanId).toBe("a");
  });

  it("creates no entries without targets", () => {
    expect(withPendingDelivery(rec("a", [])).delivery).toEqual({});
  });
});

describe("applyDelivery", () => {
  const log = (): LoggedScan[] => [
    withPendingDelivery(rec("a", ["http", "ws"])),
    withPendingDelivery(rec("b", ["http"])),
  ];

  it("sets ok on the matching row and target only", () => {
    const next = applyDelivery(log(), { scanId: "b", target: "http", ok: true, error: null });
    expect(next[1].delivery.http).toEqual({ state: "ok" });
    expect(next[0]).toEqual(log()[0]);
  });

  it("sets failed with the error", () => {
    const error: DeliveryError = { kind: "http_status", status: 500 };
    const next = applyDelivery(log(), { scanId: "a", target: "ws", ok: false, error });
    expect(next[0].delivery.ws).toEqual({ state: "failed", error });
    expect(next[0].delivery.http).toEqual({ state: "pending" });
    expect(next[1]).toEqual(log()[1]);
  });

  it("returns an equal list for an unknown scanId", () => {
    const next = applyDelivery(log(), { scanId: "gone", target: "http", ok: true, error: null });
    expect(next).toEqual(log());
  });

  it("does not mutate the input list", () => {
    const before = log();
    applyDelivery(before, { scanId: "a", target: "http", ok: true, error: null });
    expect(before).toEqual(log());
  });
});

describe("deliveryErrorText", () => {
  const t = i18n.global.t;
  const kinds: DeliveryError[] = [
    { kind: "invalid_url" },
    { kind: "unreachable", message: "connection refused" },
    { kind: "timeout" },
    { kind: "token_rejected", status: 401 },
    { kind: "http_status", status: 503 },
    { kind: "not_connected" },
  ];

  it.each(kinds)("translates $kind without leaking key names", (e) => {
    const text = deliveryErrorText(t, e);
    expect(text).not.toContain("delivery.error");
    expect(text.length).toBeGreaterThan(0);
  });

  it("interpolates status and message", () => {
    expect(deliveryErrorText(t, { kind: "token_rejected", status: 401 })).toContain("401");
    expect(deliveryErrorText(t, { kind: "http_status", status: 503 })).toContain("503");
    expect(deliveryErrorText(t, { kind: "unreachable", message: "connection refused" })).toContain(
      "connection refused",
    );
  });
});
