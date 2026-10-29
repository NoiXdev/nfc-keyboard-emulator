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
  httpHostBefore: "https://api.example.com:8443",
  httpHostAfter: "https://api.example.com:8443",
  httpToken: "set",
  httpTokenToNewHost: false,
  httpDowngrade: false,
  httpAuth: "bearer",
  httpAuthChanged: false,
  httpPayload: "standard",
  httpPayloadChanged: false,
  wsHostBefore: "wss://ws.example.com",
  wsHostAfter: "wss://ws.example.com",
  wsStream: true,
  wsRequest: false,
  wsToken: "unchanged",
  wsTokenToNewHost: false,
  wsDowngrade: false,
  wsAuth: "bearer",
  wsAuthChanged: false,
  wsPayload: "standard",
  wsPayloadChanged: false,
};

const render = (p: ImportPreview = preview) =>
  mount(ImportPreviewDialog, { props: { preview: p }, global: { plugins: [i18n] } });

describe("ImportPreviewDialog", () => {
  it("shows source, device name, hosts and mode states", () => {
    const text = render().text();
    expect(text).toContain(t("import.sourceLink"));
    expect(text).toContain("Reception");
    expect(text).toContain("https://api.example.com:8443");
    expect(text).toContain("wss://ws.example.com");
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
    const w = render({
      ...preview,
      source: "file",
      httpHostBefore: null,
      httpHostAfter: null,
      wsHostBefore: null,
      wsHostAfter: null,
    });
    expect(w.text()).toContain(t("import.sourceFile"));
    expect(w.text()).toContain(t("import.noHost"));
  });

  it("shows no warning and no before value while hosts stay the same", () => {
    const w = render();
    expect(w.find('[data-test="warnings"]').exists()).toBe(false);
    expect(w.find(".before").exists()).toBe(false);
    expect(w.get('[data-test="http-host"]').classes()).not.toContain("changed");
  });

  it("shows a changed host as before and after", () => {
    const w = render({ ...preview, wsHostAfter: "wss://new.example", wsToken: "set" });
    const host = w.get('[data-test="ws-host"]');
    expect(host.classes()).toContain("changed");
    expect(host.get(".before").text()).toBe("wss://ws.example.com");
    expect(host.text()).toContain("wss://new.example");
    expect(w.get('[data-test="ws-token"]').classes()).toContain("changed");
    expect(w.find('[data-test="warnings"]').exists()).toBe(false);
  });

  it("warns when the stored token goes to a new host", () => {
    const w = render({
      ...preview,
      wsHostAfter: "wss://evil.example",
      wsTokenToNewHost: true,
    });
    expect(w.get('[data-test="warnings"]').text()).toContain(
      t("import.warnTokenToNewHost", { target: t("import.targetWs"), host: "wss://evil.example" }),
    );
  });

  it("warns about a downgrade to an unencrypted connection", () => {
    const w = render({ ...preview, httpHostAfter: "http://api.example.com:8443", httpDowngrade: true });
    const warnings = w.get('[data-test="warnings"]');
    expect(warnings.attributes("role")).toBe("alert");
    expect(warnings.text()).toContain(t("import.warnDowngrade", { target: t("import.targetHttp") }));
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
