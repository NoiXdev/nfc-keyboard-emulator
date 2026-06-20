import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import ScanLog from "./ScanLog.vue";
import { i18n } from "../i18n";
import type { ScanRecord } from "../lib/tauri";

const rec: ScanRecord = {
  timestamp: "2026-06-19T20:15:00Z",
  reader: "ACR122U",
  uidHex: "04A1B2C3",
  uidLength: 4,
  typed: true,
  typedString: "04A1B2C3\n",
  status: "ok",
};

describe("ScanLog", () => {
  it("renders a row per scan and emits export", async () => {
    const w = mount(ScanLog, {
      props: { scans: [rec] },
      global: { plugins: [i18n] },
    });
    expect(w.text()).toContain("04A1B2C3");
    await w.get('[data-test="export"]').trigger("click");
    expect(w.emitted("export")).toHaveLength(1);
  });
});
