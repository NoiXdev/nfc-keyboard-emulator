import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import ScanLog from "./ScanLog.vue";
import { i18n } from "../i18n";
import { withPendingDelivery, type LoggedScan } from "../lib/delivery";
import type { ScanRecord } from "../lib/tauri";

const rec: ScanRecord = {
  timestamp: "2026-06-19T20:15:00Z",
  reader: "ACR122U",
  uidHex: "04A1B2C3",
  uidLength: 4,
  typed: true,
  typedString: "04A1B2C3\n",
  status: "ok",
  scanId: "id-1",
  uidFormatted: "04A1B2C3",
  requestId: null,
  deliveryTargets: [],
};

describe("ScanLog", () => {
  it("renders a row per scan and emits export", async () => {
    const w = mount(ScanLog, {
      props: { scans: [withPendingDelivery(rec)] },
      global: { plugins: [i18n] },
    });
    expect(w.text()).toContain("04A1B2C3");
    await w.get('[data-test="export"]').trigger("click");
    expect(w.emitted("export")).toHaveLength(1);
  });

  it("shows a dash in the server column when there are no targets", () => {
    const w = mount(ScanLog, {
      props: { scans: [withPendingDelivery(rec)] },
      global: { plugins: [i18n] },
    });
    expect(w.get('[data-test="server"]').text()).toBe("—");
  });

  it("shows a failed delivery with the translated error as title", () => {
    const scan: LoggedScan = {
      ...withPendingDelivery({ ...rec, deliveryTargets: ["http"] }),
      delivery: { http: { state: "failed", error: { kind: "unreachable", message: "refused" } } },
    };
    const w = mount(ScanLog, { props: { scans: [scan] }, global: { plugins: [i18n] } });
    const badge = w.get('[data-test="server"] .badge');
    expect(badge.text()).toContain("HTTP");
    expect(badge.text()).toContain("✗");
    expect(badge.attributes("title")).toBe(
      i18n.global.t("delivery.error.unreachable", { message: "refused" }),
    );
    expect(badge.attributes("title")).toContain("refused");
  });

  it("shows pending and ok marks per target", () => {
    const scan: LoggedScan = {
      ...withPendingDelivery({ ...rec, deliveryTargets: ["http", "ws"] }),
      delivery: { http: { state: "ok" }, ws: { state: "pending" } },
    };
    const w = mount(ScanLog, { props: { scans: [scan] }, global: { plugins: [i18n] } });
    const badges = w.findAll('[data-test="server"] .badge');
    expect(badges.map((b) => b.text())).toEqual(["HTTP ✓", "WS …"]);
  });
});
