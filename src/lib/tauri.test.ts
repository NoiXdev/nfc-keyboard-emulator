import { describe, it, expect, vi, beforeEach } from "vitest";
import type { ImportPreview } from "./tauri";

const invoke = vi.fn();
const listen = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: (...a: unknown[]) => listen(...a) }));

const { onImportPreview } = await import("./tauri");

const preview: ImportPreview = {
  id: "id-1",
  source: "link",
  deviceName: "Reception",
  httpEnabled: true,
  httpHost: "example.com",
  httpToken: "set",
  wsHost: null,
  wsStream: false,
  wsRequest: false,
  wsToken: "unchanged",
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
});
