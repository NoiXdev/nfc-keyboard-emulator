import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { flushPromises } from "@vue/test-utils";
import ServerSettings from "./ServerSettings.vue";
import { i18n } from "../i18n";
import { api, type ConfigError, type IntegrationConfig, type WsStatus } from "../lib/tauri";

vi.mock("../lib/tauri", () => ({ api: { testHttp: vi.fn(), testWs: vi.fn() } }));

const t = i18n.global.t;

const integration: IntegrationConfig = {
  deviceName: "Reception",
  http: { enabled: true, url: "https://example.com/api", token: "http-secret" },
  ws: { url: "wss://example.com/ws", token: "ws-secret", stream: true, request: false },
};

function render(
  props: { ws?: WsStatus | null; saveError?: ConfigError | null; integration?: IntegrationConfig } = {},
) {
  return mount(ServerSettings, {
    props: { integration, ws: null, saveError: null, ...props },
    global: { plugins: [i18n] },
  });
}

beforeEach(() => {
  vi.mocked(api.testHttp).mockReset();
  vi.mocked(api.testWs).mockReset();
});

describe("ServerSettings", () => {
  it("emits save with the edited config", async () => {
    const w = render();
    await w.get('[data-test="http-url"]').setValue("https://other.example/hook");
    await w.get('[data-test="save"]').trigger("click");
    const [saved] = w.emitted("save")![0] as [IntegrationConfig];
    expect(saved.http.url).toBe("https://other.example/hook");
    expect(saved.ws).toEqual(integration.ws);
    expect(saved.deviceName).toBe("Reception");
  });

  it("only enables save when something changed", async () => {
    const w = render();
    expect(w.get('[data-test="save"]').attributes("disabled")).toBeDefined();
    await w.get('[data-test="device-name"]').setValue("Desk");
    expect(w.get('[data-test="save"]').attributes("disabled")).toBeUndefined();
  });

  it("takes over a new integration from the parent", async () => {
    const w = render();
    await w.setProps({
      integration: { ...integration, ws: { ...integration.ws, url: "wss://new.example/ws" } },
    });
    expect((w.get('[data-test="ws-url"]').element as HTMLInputElement).value).toBe(
      "wss://new.example/ws",
    );
  });

  it("masks tokens until reveal is clicked", async () => {
    const w = render();
    const http = () => w.get('[data-test="http-token"]');
    const ws = () => w.get('[data-test="ws-token"]');
    expect(http().attributes("type")).toBe("password");
    expect(ws().attributes("type")).toBe("password");
    await w.get('[data-test="http-token-reveal"]').trigger("click");
    expect(http().attributes("type")).toBe("text");
    expect(ws().attributes("type")).toBe("password");
    await w.get('[data-test="http-token-reveal"]').trigger("click");
    expect(http().attributes("type")).toBe("password");
  });

  it("renders the translated save error", () => {
    const w = render({ saveError: { kind: "invalid_ws_url" } });
    expect(w.get('[data-test="save-error"]').text()).toBe(t("server.error.invalid_ws_url"));
  });

  it("shows no error element without a save error", () => {
    expect(render().find('[data-test="save-error"]').exists()).toBe(false);
  });

  it.each([
    [{ state: "off", requestPending: false }, false],
    [{ state: "connecting", requestPending: false }, false],
    [{ state: "connected", requestPending: false }, false],
    [{ state: "token_rejected", requestPending: false }, true],
    [{ state: "reconnecting", error: { kind: "timeout" }, requestPending: false }, true],
  ] as Array<[WsStatus, boolean]>)("reconnect visibility for %j", async (ws, visible) => {
    const w = render({ ws });
    const btn = w.find('[data-test="ws-reconnect"]');
    expect(btn.exists()).toBe(visible);
    if (visible) {
      await btn.trigger("click");
      expect(w.emitted("reconnect")).toHaveLength(1);
    }
  });

  it("shows the connection state", () => {
    const w = render({ ws: { state: "connected", requestPending: false } });
    expect(w.get('[data-test="ws-state"]').text()).toContain(t("server.state.connected"));
  });

  it("tests HTTP with the form values and shows OK", async () => {
    vi.mocked(api.testHttp).mockResolvedValue(undefined);
    const w = render();
    await w.get('[data-test="http-url"]').setValue("https://typed.example/x");
    await w.get('[data-test="http-test"]').trigger("click");
    await flushPromises();
    const [http, device] = vi.mocked(api.testHttp).mock.calls[0];
    expect(http.url).toBe("https://typed.example/x");
    expect(device).toBe("Reception");
    expect(w.get('[data-test="http-test-result"]').text()).toBe(t("server.testOk"));
  });

  it("shows the translated error when the HTTP test fails", async () => {
    vi.mocked(api.testHttp).mockRejectedValue({ kind: "token_rejected", status: 401 });
    const w = render();
    await w.get('[data-test="http-test"]').trigger("click");
    await flushPromises();
    expect(w.get('[data-test="http-test-result"]').text()).toContain("401");
  });

  it("disables the test button while a test runs", async () => {
    let finish!: () => void;
    vi.mocked(api.testWs).mockReturnValue(new Promise<void>((r) => (finish = r)));
    const w = render();
    await w.get('[data-test="ws-test"]').trigger("click");
    expect(w.get('[data-test="ws-test"]').attributes("disabled")).toBeDefined();
    finish();
    await flushPromises();
    expect(w.get('[data-test="ws-test"]').attributes("disabled")).toBeUndefined();
    expect(w.get('[data-test="ws-test-result"]').text()).toBe(t("server.testOk"));
  });

  it("clears an old test result when the form is edited", async () => {
    vi.mocked(api.testHttp).mockResolvedValue(undefined);
    const w = render();
    await w.get('[data-test="http-test"]').trigger("click");
    await flushPromises();
    await w.get('[data-test="http-url"]').setValue("https://changed.example");
    expect(w.find('[data-test="http-test-result"]').exists()).toBe(false);
  });

  it("asks for confirmation before exporting", async () => {
    const w = render();
    expect(w.find('[data-test="export-warning"]').exists()).toBe(false);
    await w.get('[data-test="export"]').trigger("click");
    expect(w.emitted("export")).toBeUndefined();
    expect(w.get('[data-test="export-warning"]').text()).toContain(t("server.exportWarning"));
    await w.get('[data-test="export-confirm"]').trigger("click");
    expect(w.emitted("export")).toHaveLength(1);
    expect(w.find('[data-test="export-warning"]').exists()).toBe(false);
  });

  it("cancelling the export confirmation emits nothing", async () => {
    const w = render();
    await w.get('[data-test="export"]').trigger("click");
    await w.get('[data-test="export-cancel"]').trigger("click");
    expect(w.emitted("export")).toBeUndefined();
    expect(w.find('[data-test="export-warning"]').exists()).toBe(false);
  });

  it("emits import", async () => {
    const w = render();
    await w.get('[data-test="import"]').trigger("click");
    expect(w.emitted("import")).toHaveLength(1);
  });
});
