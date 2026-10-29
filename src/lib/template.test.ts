import { describe, it, expect } from "vitest";
import {
  MAX_TEMPLATE_BYTES,
  PLACEHOLDERS,
  defaultAuth,
  defaultPayload,
  defaultTemplate,
  examplePayload,
  renderTemplate,
  type ScanPayload,
  type TemplateError,
} from "./template";
import vectors from "./templateVectors.json";

interface Vector {
  name: string;
  template: string;
  payload: ScanPayload;
  expected?: unknown;
  error?: TemplateError;
}

describe("shared vectors", () => {
  it("has vectors", () => {
    expect(vectors.length).toBeGreaterThan(0);
  });

  it.each(vectors as Vector[])("$name", (v) => {
    const result = renderTemplate(v.template, v.payload);
    if (v.error) {
      expect(result).toEqual({ ok: false, error: v.error });
    } else {
      expect(result.ok).toBe(true);
      // Serialized, so key order counts.
      expect(JSON.stringify(result.ok && result.value)).toBe(JSON.stringify(v.expected));
    }
  });
});

describe("constants", () => {
  it("matches the backend", () => {
    expect(MAX_TEMPLATE_BYTES).toBe(8192);
    expect(PLACEHOLDERS).toEqual([
      "uid",
      "uidFormatted",
      "uidLength",
      "scanId",
      "device",
      "reader",
      "timestamp",
    ]);
  });
});

describe("defaults", () => {
  it("auth keeps today's bearer behaviour", () => {
    expect(defaultAuth()).toEqual({
      mode: "bearer",
      headerName: "X-API-Key",
      username: "",
      queryParam: "token",
    });
  });

  it("payload is standard with an empty template", () => {
    expect(defaultPayload()).toEqual({ mode: "standard", template: "" });
  });
});

describe("examplePayload", () => {
  it("has the fixed fields", () => {
    const p = examplePayload("Kasse 1", new Date("2026-10-29T10:15:30.123Z"));
    expect(p).toEqual({
      scanId: "test",
      device: "Kasse 1",
      reader: "Example Reader",
      uid: "04A1B2C3D4E5F6",
      uidFormatted: "04A1B2C3D4E5F6",
      uidLength: 7,
      timestamp: "2026-10-29T10:15:30.123Z",
    });
  });

  it("defaults the timestamp to now", () => {
    expect(examplePayload("d").timestamp).toMatch(/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}Z$/);
  });
});

describe("defaultTemplate", () => {
  it("renders to the standard payload fields", () => {
    const payload = examplePayload("Kasse 1");
    const result = renderTemplate(defaultTemplate(), payload);
    expect(result.ok).toBe(true);
    expect(JSON.stringify(result.ok && result.value)).toBe(
      JSON.stringify({
        scanId: payload.scanId,
        device: payload.device,
        reader: payload.reader,
        uid: payload.uid,
        uidFormatted: payload.uidFormatted,
        uidLength: payload.uidLength,
        timestamp: payload.timestamp,
      }),
    );
  });

  it("is pretty-printed", () => {
    expect(defaultTemplate()).toContain("\n  ");
  });
});

describe("renderTemplate", () => {
  const payload = examplePayload("Kasse 1");

  it("checks the size before the syntax", () => {
    const big = "x".repeat(MAX_TEMPLATE_BYTES + 1);
    expect(renderTemplate(big, payload)).toEqual({ ok: false, error: { kind: "too_large" } });
  });

  it("does not pollute prototypes through a __proto__ key", () => {
    const result = renderTemplate('{"__proto__": {"x": "{{uid}}"}}', payload);
    expect(result.ok).toBe(true);
    expect(({} as Record<string, unknown>).x).toBeUndefined();
    expect(JSON.stringify(result.ok && result.value)).toBe('{"__proto__":{"x":"04A1B2C3D4E5F6"}}');
  });
});
