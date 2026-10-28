import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import ImportPreviewDialog from "./ImportPreviewDialog.vue";
import { i18n } from "../i18n";
import type { ImportPreview } from "../lib/tauri";

const t = i18n.global.t;

const preview: ImportPreview = {
  id: "imp-1",
  source: "link",
  deviceName: "Reception",
  httpEnabled: true,
  httpHost: "api.example.com:8443",
  httpToken: "set",
  wsHost: "ws.example.com",
  wsStream: true,
  wsRequest: false,
  wsToken: "unchanged",
};

const render = (p: ImportPreview = preview) =>
  mount(ImportPreviewDialog, { props: { preview: p }, global: { plugins: [i18n] } });

describe("ImportPreviewDialog", () => {
  it("shows source, device name, hosts and mode states", () => {
    const text = render().text();
    expect(text).toContain(t("import.sourceLink"));
    expect(text).toContain("Reception");
    expect(text).toContain("api.example.com:8443");
    expect(text).toContain("ws.example.com");
    expect(text).toContain(t("import.on"));
    expect(text).toContain(t("import.off"));
  });

  it("shows the token change as text", () => {
    const text = render().text();
    expect(text).toContain(t("import.tokenSet"));
    expect(text).toContain(t("import.tokenUnchanged"));
    expect(render({ ...preview, wsToken: "cleared" }).text()).toContain(t("import.tokenCleared"));
  });

  it("shows a placeholder for a missing host and the file source", () => {
    const w = render({ ...preview, source: "file", httpHost: null, wsHost: null });
    expect(w.text()).toContain(t("import.sourceFile"));
    expect(w.text()).toContain(t("import.noHost"));
  });

  it("never renders a token field", () => {
    const leaky = { ...preview, token: "LEAK", httpTokenValue: "LEAK" } as ImportPreview;
    expect(render(leaky).html()).not.toContain("LEAK");
  });

  it("emits apply and cancel with the preview id", async () => {
    const w = render();
    await w.get('[data-test="apply"]').trigger("click");
    await w.get('[data-test="cancel"]').trigger("click");
    expect(w.emitted("apply")![0]).toEqual(["imp-1"]);
    expect(w.emitted("cancel")![0]).toEqual(["imp-1"]);
  });
});
