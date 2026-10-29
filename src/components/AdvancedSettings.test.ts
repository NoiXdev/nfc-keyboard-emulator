import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import AdvancedSettings from "./AdvancedSettings.vue";
import { i18n } from "../i18n";
import { defaultAuth, defaultPayload, defaultTemplate } from "../lib/template";
import type { AuthConfig, PayloadConfig } from "../lib/tauri";

const t = i18n.global.t;

// The model props are fed back from the emitted updates, like v-model does in a parent.
function render(
  opts: { target?: "http" | "ws"; auth?: Partial<AuthConfig>; payload?: Partial<PayloadConfig> } = {},
) {
  return mount(AdvancedSettings, {
    props: {
      target: opts.target ?? "http",
      device: "Reception",
      auth: { ...defaultAuth(), ...opts.auth },
      payload: { ...defaultPayload(), ...opts.payload },
      "onUpdate:auth": (a: AuthConfig) => w.setProps({ auth: a }),
      "onUpdate:payload": (p: PayloadConfig) => w.setProps({ payload: p }),
    },
    global: { plugins: [i18n] },
  });
  // `w` is assigned below, before any handler can run.
}
let w: ReturnType<typeof render>;
const mountIt = (opts?: Parameters<typeof render>[0]) => (w = render(opts));

const el = (id: string) => w.find(`[data-test="${id}"]`);
const text = (id: string) => el(id).text();

describe("AdvancedSettings", () => {
  it("is closed and quiet with the defaults", () => {
    mountIt();
    expect(el("http-advanced").attributes("open")).toBeUndefined();
    expect(el("http-advanced").text()).toContain(t("server.advanced.title"));
    expect(el("http-advanced-summary").exists()).toBe(false);
  });

  it("opens on mount with basic auth and summarises the non-default parts", () => {
    mountIt({ auth: { mode: "basic" }, payload: { mode: "template", template: "{}" } });
    expect(el("http-advanced").attributes("open")).toBeDefined();
    expect(text("http-advanced-summary")).toBe(
      `${t("server.advanced.auth.basic")} · ${t("server.advanced.payload.template")}`,
    );
  });

  it("opens on mount for a custom template alone", () => {
    mountIt({ payload: { mode: "template", template: "{}" } });
    expect(el("http-advanced").attributes("open")).toBeDefined();
    expect(text("http-advanced-summary")).toBe(t("server.advanced.payload.template"));
  });

  it("offers the five auth modes", () => {
    mountIt();
    const values = w.findAll('[data-test="http-auth-mode"] option').map((o) => o.attributes("value"));
    expect(values).toEqual(["bearer", "header", "basic", "query", "none"]);
  });

  it.each([
    ["bearer", []],
    ["header", ["http-header-name"]],
    ["basic", ["http-username"]],
    ["query", ["http-query-param"]],
    ["none", []],
  ])("shows only the fields of the %s mode", async (mode, visible) => {
    mountIt();
    await el("http-auth-mode").setValue(mode);
    for (const id of ["http-header-name", "http-username", "http-query-param"]) {
      expect(el(id).exists()).toBe(visible.includes(id));
    }
  });

  it("keeps the other modes' values when switching", async () => {
    mountIt({ auth: { mode: "header", headerName: "X-Custom" } });
    await el("http-auth-mode").setValue("basic");
    await el("http-auth-mode").setValue("header");
    expect((el("http-header-name").element as HTMLInputElement).value).toBe("X-Custom");
  });

  it("emits edited auth fields", async () => {
    mountIt({ auth: { mode: "basic" } });
    await el("http-username").setValue("reader");
    expect(w.props("auth")).toEqual({ ...defaultAuth(), mode: "basic", username: "reader" });
  });

  it("warns about the token in the address only for query", async () => {
    mountIt();
    expect(el("http-query-warning").exists()).toBe(false);
    await el("http-auth-mode").setValue("query");
    expect(text("http-query-warning")).toBe(t("server.advanced.queryWarning"));
    await el("http-auth-mode").setValue("none");
    expect(el("http-query-warning").exists()).toBe(false);
  });

  it("hides the template tools in standard mode", () => {
    mountIt();
    expect(el("http-template").exists()).toBe(false);
    expect(el("http-preview").exists()).toBe(false);
  });

  it("fills an empty template with the default on the first switch", async () => {
    mountIt();
    await el("http-payload-mode").setValue("template");
    expect(w.props("payload")).toEqual({ mode: "template", template: defaultTemplate() });
    expect((el("http-template").element as HTMLTextAreaElement).value).toBe(defaultTemplate());
  });

  it("keeps an existing template when switching to template mode", async () => {
    mountIt({ payload: { mode: "standard", template: '{"a":"{{uid}}"}' } });
    await el("http-payload-mode").setValue("template");
    expect(w.props("payload").template).toBe('{"a":"{{uid}}"}');
  });

  it("inserts a placeholder at the cursor", async () => {
    mountIt({ payload: { mode: "template", template: '{"a":""}' } });
    const ta = el("http-template").element as HTMLTextAreaElement;
    ta.setSelectionRange(6, 6);
    await el("http-placeholder-uid").trigger("click");
    expect(w.props("payload").template).toBe('{"a":"{{uid}}"}');
    expect(ta.selectionStart).toBe(6 + "{{uid}}".length);
  });

  it("replaces the selection with the placeholder", async () => {
    mountIt({ payload: { mode: "template", template: '{"a":"xx"}' } });
    (el("http-template").element as HTMLTextAreaElement).setSelectionRange(6, 8);
    await el("http-placeholder-reader").trigger("click");
    expect(w.props("payload").template).toBe('{"a":"{{reader}}"}');
  });

  it("offers one button per placeholder", () => {
    mountIt({ payload: { mode: "template", template: "{}" } });
    for (const n of ["uid", "uidFormatted", "uidLength", "scanId", "device", "reader", "timestamp"]) {
      expect(text(`http-placeholder-${n}`)).toBe(`{{${n}}}`);
    }
  });

  it("previews the rendered example and follows the input", async () => {
    mountIt({ payload: { mode: "template", template: '{"d":"{{device}}","n":"{{uidLength}}"}' } });
    expect(JSON.parse(text("http-preview"))).toEqual({ d: "Reception", n: 7 });
    await el("http-template").setValue('{"u":"{{uid}}"}');
    expect(JSON.parse(text("http-preview"))).toEqual({ u: "04A1B2C3D4E5F6" });
    expect(el("http-template-error").exists()).toBe(false);
  });

  it("shows the unknown placeholder instead of a preview", async () => {
    mountIt({ payload: { mode: "template", template: "{}" } });
    await el("http-template").setValue('{"x":"{{nope}}"}');
    expect(text("http-template-error")).toBe(
      t("server.error.template_unknown_placeholder", { name: "nope" }),
    );
    expect(el("http-preview").exists()).toBe(false);
  });

  it("shows a syntax error for invalid JSON", async () => {
    mountIt({ payload: { mode: "template", template: "{}" } });
    await el("http-template").setValue("{oops");
    expect(text("http-template-error")).toBe(t("server.error.template_not_json"));
  });

  it("notes that the test sends an example scan", () => {
    mountIt({ payload: { mode: "template", template: "{}" } });
    expect(text("http-test-note")).toBe(t("server.advanced.testNote"));
  });

  it("shows the request note only for the websocket", () => {
    mountIt({ target: "ws", payload: { mode: "template", template: "{}" } });
    expect(text("ws-request-note")).toBe(t("server.advanced.requestNote"));
    mountIt({ target: "http", payload: { mode: "template", template: "{}" } });
    expect(el("http-request-note").exists()).toBe(false);
  });

  it("prefixes every test id with the target", () => {
    mountIt({ target: "ws", auth: { mode: "query" }, payload: { mode: "template", template: "{}" } });
    for (const id of ["advanced", "auth-mode", "query-param", "query-warning", "payload-mode", "template", "preview"]) {
      expect(el(`ws-${id}`).exists()).toBe(true);
    }
  });
});
