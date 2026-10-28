import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import StatusBar from "./StatusBar.vue";
import { i18n } from "../i18n";
import type { WsStatus } from "../lib/tauri";

const t = i18n.global.t;

function render(ws: WsStatus | null) {
  return mount(StatusBar, {
    props: { status: { kind: "connected" }, typing: true, ws },
    global: { plugins: [i18n] },
  });
}

describe("StatusBar server item", () => {
  it("is hidden without a status", () => {
    expect(render(null).find('[data-test="server"]').exists()).toBe(false);
  });

  it("is hidden when the websocket is off", () => {
    const w = render({ state: "off", requestPending: false });
    expect(w.find('[data-test="server"]').exists()).toBe(false);
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
