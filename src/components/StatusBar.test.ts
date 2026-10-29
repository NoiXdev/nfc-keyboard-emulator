import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import StatusBar from "./StatusBar.vue";
import { i18n } from "../i18n";
import type { WsStatus } from "../lib/tauri";

const t = i18n.global.t;

function render(ws: WsStatus | null, httpEnabled = false) {
  return mount(StatusBar, {
    props: { status: { kind: "connected" }, typing: true, ws, httpEnabled },
    global: { plugins: [i18n] },
  });
}

describe("StatusBar server item", () => {
  it("shows off without a status", () => {
    expect(render(null).get('[data-test="server"]').text()).toContain(t("footer.off"));
  });

  it("shows off when everything is off", () => {
    const w = render({ state: "off", requestPending: false });
    expect(w.get('[data-test="server"]').text()).toContain(t("footer.off"));
  });

  it("shows http push when only http is on", () => {
    const w = render({ state: "off", requestPending: false }, true);
    expect(w.get('[data-test="server"]').text()).toContain(t("footer.serverHttp"));
  });

  it("prefers the websocket state over http", () => {
    const w = render({ state: "connected", requestPending: false }, true);
    const text = w.get('[data-test="server"]').text();
    expect(text).toContain(t("footer.serverConnected"));
    expect(text).not.toContain(t("footer.serverHttp"));
  });

  it("shows the translated token rejected text", () => {
    const w = render({ state: "token_rejected", requestPending: false });
    expect(w.get('[data-test="server"]').text()).toContain(t("footer.serverTokenRejected"));
  });

  it("shows waiting for scan while a request is pending", () => {
    const w = render({ state: "connected", requestPending: true });
    const text = w.get('[data-test="server"]').text();
    expect(text).toContain(t("footer.serverConnected"));
    expect(text).toContain(t("footer.serverWaiting"));
  });

  it("does not show waiting when no request is pending", () => {
    const w = render({ state: "connected", requestPending: false });
    expect(w.get('[data-test="server"]').text()).not.toContain(t("footer.serverWaiting"));
  });

  it("shows connecting", () => {
    const w = render({ state: "connecting", requestPending: false });
    expect(w.get('[data-test="server"]').text()).toContain(t("footer.serverConnecting"));
  });

  it("shows the translated error while reconnecting", () => {
    const w = render({
      state: "reconnecting",
      error: { kind: "unreachable", message: "refused" },
      lost: true,
      requestPending: false,
    });
    const text = w.get('[data-test="server"]').text();
    expect(text).toContain(t("delivery.error.unreachable", { message: "refused" }));
  });
});
