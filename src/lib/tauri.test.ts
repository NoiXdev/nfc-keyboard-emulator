import { describe, it, expect, vi, beforeEach } from "vitest";
import type { ImportError, ImportPreview } from "./tauri";

const invoke = vi.fn();
const listen = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: (...a: unknown[]) => listen(...a) }));

const { onImportPreview, onImportError } = await import("./tauri");

const preview: ImportPreview = {
  id: "id-1",
  source: "link",
  deviceName: "Reception",
  httpEnabled: true,
  httpHostBefore: null,
  httpHostAfter: "https://example.com",
  httpToken: "set",
  httpTokenToNewHost: false,
  httpTokenToQuery: false,
  httpDowngrade: false,
  httpAuth: "bearer",
  httpAuthChanged: false,
  httpPayload: "standard",
  httpPayloadChanged: false,
  wsHostBefore: null,
  wsHostAfter: null,
  wsStream: false,
  wsRequest: false,
  wsToken: "unchanged",
  wsTokenToNewHost: false,
  wsTokenToQuery: false,
  wsDowngrade: false,
  wsAuth: "bearer",
  wsAuthChanged: false,
  wsPayload: "standard",
  wsPayloadChanged: false,
};

describe("onImportPreview", () => {
  beforeEach(() => {
    invoke.mockReset();
    listen.mockReset().mockResolvedValue(() => {});
  });

  it("replays an import that was pending before subscribing", async () => {
    invoke.mockResolvedValue(preview);
    const cb = vi.fn();
    await onImportPreview(cb);
    expect(listen).toHaveBeenCalledWith("import-preview", expect.any(Function));
    expect(invoke).toHaveBeenCalledWith("get_pending_import");
    expect(cb).toHaveBeenCalledWith(preview);
  });

  it("calls nothing when no import is pending", async () => {
    invoke.mockResolvedValue(null);
    const cb = vi.fn();
    await onImportPreview(cb);
    expect(cb).not.toHaveBeenCalled();
  });

  it("forwards events", async () => {
    invoke.mockResolvedValue(null);
    const cb = vi.fn();
    await onImportPreview(cb);
    const handler = listen.mock.calls[0][1] as (e: { payload: ImportPreview }) => void;
    handler({ payload: preview });
    expect(cb).toHaveBeenCalledWith(preview);
  });

  it("keeps the listener when the replay fetch fails", async () => {
    const unlisten = vi.fn();
    listen.mockResolvedValue(unlisten);
    invoke.mockRejectedValue(new Error("ipc down"));
    const cb = vi.fn();
    await expect(onImportPreview(cb)).resolves.toBe(unlisten);
    expect(cb).not.toHaveBeenCalled();
    expect(unlisten).not.toHaveBeenCalled();
  });
});

describe("onImportError", () => {
  const error: ImportError = { kind: "invalid_link" };

  beforeEach(() => {
    invoke.mockReset();
    listen.mockReset().mockResolvedValue(() => {});
  });

  it("replays the error of a link that arrived before subscribing", async () => {
    invoke.mockResolvedValue(error);
    const cb = vi.fn();
    await onImportError(cb);
    expect(listen).toHaveBeenCalledWith("import-error", expect.any(Function));
    expect(invoke).toHaveBeenCalledWith("take_pending_import_error");
    expect(cb).toHaveBeenCalledWith(error);
  });

  it("calls nothing when no error is pending", async () => {
    invoke.mockResolvedValue(null);
    const cb = vi.fn();
    await onImportError(cb);
    expect(cb).not.toHaveBeenCalled();
  });

  it("forwards events", async () => {
    invoke.mockResolvedValue(null);
    const cb = vi.fn();
    await onImportError(cb);
    const handler = listen.mock.calls[0][1] as (e: { payload: ImportError }) => void;
    handler({ payload: error });
    expect(cb).toHaveBeenCalledWith(error);
  });

  it("keeps the listener when the replay fetch fails", async () => {
    const unlisten = vi.fn();
    listen.mockResolvedValue(unlisten);
    invoke.mockRejectedValue(new Error("ipc down"));
    const cb = vi.fn();
    await expect(onImportError(cb)).resolves.toBe(unlisten);
    expect(cb).not.toHaveBeenCalled();
    expect(unlisten).not.toHaveBeenCalled();
  });
});
