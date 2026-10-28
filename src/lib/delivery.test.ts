import { describe, it, expect } from "vitest";
import {
  applyDelivery,
  deliveryErrorText,
  requestNotice,
  serverErrorText,
  withPendingDelivery,
  wsNotice,
  type LoggedScan,
} from "./delivery";
import { i18n } from "../i18n";
import type { ConfigError, DeliveryError, ImportError, ScanRecord, WsStatus } from "./tauri";

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

describe("serverErrorText", () => {
  const t = i18n.global.t;
  const errors: Array<ConfigError | ImportError> = [
    { kind: "invalid_json", message: "expected value at line 1" },
    { kind: "wrong_format" },
    { kind: "unsupported_version", version: 7 },
    { kind: "invalid_http_url" },
    { kind: "invalid_ws_url" },
    { kind: "invalid_http_token" },
    { kind: "invalid_ws_token" },
    { kind: "invalid_link" },
  ];

  it.each(errors)("translates $kind without leaking key names", (e) => {
    const text = serverErrorText(t, e);
    expect(text).not.toContain("server.error");
    expect(text.length).toBeGreaterThan(0);
  });

  it("never shows the raw invalid_json message", () => {
    const text = serverErrorText(t, { kind: "invalid_json", message: "expected value at line 1" });
    expect(text).not.toContain("expected value");
  });

  it("interpolates the unsupported version", () => {
    expect(serverErrorText(t, { kind: "unsupported_version", version: 7 })).toContain("7");
  });
});

describe("wsNotice", () => {
  const status = (s: WsStatus["state"], extra: object = {}): WsStatus =>
    ({ state: s, requestPending: false, ...extra }) as WsStatus;
  const reconnecting = status("reconnecting", { error: { kind: "timeout" }, lost: true });

  it("flags connected to reconnecting with the error", () => {
    expect(wsNotice(status("connected"), reconnecting)).toEqual({
      kind: "reconnecting",
      error: { kind: "timeout" },
    });
  });

  it("stays quiet for drops the backend does not mark as lost", () => {
    const repeated = status("reconnecting", { error: { kind: "timeout" }, lost: false });
    expect(wsNotice(status("connected"), repeated)).toBeNull();
  });

  it("ignores reconnecting while already reconnecting or connecting", () => {
    expect(wsNotice(reconnecting, reconnecting)).toBeNull();
    expect(wsNotice(status("connecting"), reconnecting)).toBeNull();
    expect(wsNotice(null, reconnecting)).toBeNull();
  });

  it("flags token_rejected from any state once", () => {
    expect(wsNotice(status("connecting"), status("token_rejected"))).toEqual({
      kind: "token_rejected",
    });
    expect(wsNotice(null, status("token_rejected"))).toEqual({ kind: "token_rejected" });
    expect(wsNotice(status("token_rejected"), status("token_rejected", { requestPending: true }))).toBeNull();
  });

  it("is quiet for other transitions", () => {
    expect(wsNotice(status("connecting"), status("connected"))).toBeNull();
    expect(wsNotice(status("connected"), status("off"))).toBeNull();
  });
});

describe("requestNotice", () => {
  it("notifies for a new id and remembers it", () => {
    expect(requestNotice(null, "r1")).toEqual({ notify: true, lastNotified: "r1" });
  });

  it("stays quiet for the same id twice", () => {
    const first = requestNotice(null, "r1");
    expect(requestNotice(first.lastNotified, "r1").notify).toBe(false);
  });

  it("notifies again for a different id", () => {
    expect(requestNotice("r1", "r2")).toEqual({ notify: true, lastNotified: "r2" });
  });

  it("notifies again for the same id once the caller cleared the memory", () => {
    expect(requestNotice(null, "r1").notify).toBe(true);
  });
});
