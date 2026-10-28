import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Case = "upper" | "lower";
export type ByteOrder = "normal" | "reversed";
export type Suffix = "enter" | "tab" | "none";

export interface FormatConfig {
  case: Case;
  separator: string;
  byteOrder: ByteOrder;
  prefix: string;
  suffix: Suffix;
}

export interface HttpTarget {
  enabled: boolean;
  url: string;
  token: string;
}

export interface WsTarget {
  url: string;
  token: string;
  stream: boolean;
  request: boolean;
}

export interface IntegrationConfig {
  deviceName: string;
  http: HttpTarget;
  ws: WsTarget;
}

export interface Config {
  selectedReader: string | null;
  typingEnabledOnStart: boolean;
  format: FormatConfig;
  startMinimized: boolean;
  autostart: boolean;
  logRetention: number;
  integration: IntegrationConfig;
}

export type ScanStatus =
  | "ok"
  | "read_error"
  | "typing_disabled"
  | "no_accessibility"
  | "type_error"
  | "server_request";

export interface ScanRecord {
  timestamp: string;
  reader: string;
  uidHex: string;
  uidLength: number;
  typed: boolean;
  typedString: string;
  status: ScanStatus;
  scanId: string;
  uidFormatted: string;
  requestId: string | null;
  deliveryTargets: DeliveryTarget[];
}

export type DeliveryTarget = "http" | "ws";

export type DeliveryError =
  | { kind: "invalid_url" }
  | { kind: "unreachable"; message: string }
  | { kind: "timeout" }
  | { kind: "token_rejected"; status: number }
  | { kind: "http_status"; status: number }
  | { kind: "not_connected" };

export interface DeliveryResult {
  scanId: string;
  target: DeliveryTarget;
  ok: boolean;
  error: DeliveryError | null;
}

export type WsState =
  | { state: "off" }
  | { state: "connecting" }
  | { state: "connected" }
  /** `lost` marks the drop worth a notice; repeated drops of an unstable server are quiet. */
  | { state: "reconnecting"; error: DeliveryError; lost: boolean }
  | { state: "token_rejected" };

export type WsStatus = WsState & { requestPending: boolean };

export type ConfigError = { kind: "invalid_http_url" } | { kind: "invalid_ws_url" };

export type ImportError =
  | { kind: "invalid_json"; message: string }
  | { kind: "wrong_format" }
  | { kind: "unsupported_version"; version: number }
  | { kind: "invalid_http_url" }
  | { kind: "invalid_ws_url" }
  | { kind: "invalid_link" };

export type ImportSource = "file" | "link";

export type TokenChange = "unchanged" | "set" | "cleared";

/** What applying an import changes; tokens appear only as a TokenChange. */
export interface ImportPreview {
  id: string;
  source: ImportSource;
  deviceName: string;
  httpEnabled: boolean;
  httpHost: string | null;
  httpToken: TokenChange;
  wsHost: string | null;
  wsStream: boolean;
  wsRequest: boolean;
  wsToken: TokenChange;
}

export type ReaderStatus =
  | { kind: "connected" }
  | { kind: "disconnected" }
  | { kind: "error"; message: string };

export const api = {
  getConfig: () => invoke<Config>("get_config"),
  listReaders: () => invoke<string[]>("list_readers"),
  rescanReaders: () => invoke<void>("rescan_readers"),
  selectReader: (name: string | null) => invoke<void>("select_reader", { name }),
  setTypingEnabled: (enabled: boolean) => invoke<void>("set_typing_enabled", { enabled }),
  updateFormat: (format: FormatConfig) => invoke<void>("update_format", { format }),
  formatPreview: (format: FormatConfig) => invoke<string>("format_preview", { format }),
  setStartMinimized: (value: boolean) => invoke<void>("set_start_minimized", { value }),
  exportLogCsv: () => invoke<boolean>("export_log_csv"),
  checkAccessibility: () => invoke<boolean>("check_accessibility"),
  openAccessibilitySettings: () => invoke<void>("open_accessibility_settings"),
  updateIntegration: (integration: IntegrationConfig) =>
    invoke<Config>("update_integration", { integration }),
  testHttp: (http: HttpTarget, deviceName: string) =>
    invoke<void>("test_http", { http, deviceName }),
  testWs: (ws: WsTarget, deviceName: string) => invoke<void>("test_ws", { ws, deviceName }),
  wsReconnect: () => invoke<void>("ws_reconnect"),
  getWsStatus: () => invoke<WsStatus>("get_ws_status"),
  exportServerConfig: () => invoke<boolean>("export_server_config"),
  importServerConfig: () => invoke<ImportPreview | null>("import_server_config"),
  getPendingImport: () => invoke<ImportPreview | null>("get_pending_import"),
  takePendingImportError: () => invoke<ImportError | null>("take_pending_import_error"),
  applyImport: (id: string) => invoke<Config>("apply_import", { id }),
  discardImport: (id: string) => invoke<void>("discard_import", { id }),
};

export const onScan = (cb: (r: ScanRecord) => void): Promise<UnlistenFn> =>
  listen<ScanRecord>("scan", (e) => cb(e.payload));
export const onReadersChanged = (cb: (r: string[]) => void): Promise<UnlistenFn> =>
  listen<string[]>("readers-changed", (e) => cb(e.payload));
export const onReaderStatus = (cb: (s: ReaderStatus) => void): Promise<UnlistenFn> =>
  listen<ReaderStatus>("reader-status", (e) => cb(e.payload));
export const onDelivery = (cb: (r: DeliveryResult) => void): Promise<UnlistenFn> =>
  listen<DeliveryResult>("scan-delivery", (e) => cb(e.payload));
export const onWsStatus = (cb: (s: WsStatus) => void): Promise<UnlistenFn> =>
  listen<WsStatus>("ws-status", (e) => cb(e.payload));
export const onWsRequest = (cb: (requestId: string) => void): Promise<UnlistenFn> =>
  listen<{ requestId: string }>("ws-request", (e) => cb(e.payload.requestId));
// A link that started the app arrives before anyone listens, so what it left behind is
// fetched once after subscribing as well. A failed fetch only skips that replay.
const listenWithReplay = async <T>(
  event: string,
  pending: () => Promise<T | null>,
  cb: (payload: T) => void,
): Promise<UnlistenFn> => {
  const unlisten = await listen<T>(event, (e) => cb(e.payload));
  const missed = await pending().catch(() => null);
  if (missed) cb(missed);
  return unlisten;
};
export const onImportPreview = (cb: (p: ImportPreview) => void): Promise<UnlistenFn> =>
  listenWithReplay("import-preview", api.getPendingImport, cb);
export const onImportError = (cb: (e: ImportError) => void): Promise<UnlistenFn> =>
  listenWithReplay("import-error", api.takePendingImportError, cb);
