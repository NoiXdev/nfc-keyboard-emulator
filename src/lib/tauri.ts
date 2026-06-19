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

export interface Config {
  selectedReader: string | null;
  typingEnabledOnStart: boolean;
  format: FormatConfig;
  startMinimized: boolean;
  autostart: boolean;
  logRetention: number;
}

export type ScanStatus =
  | "ok"
  | "read_error"
  | "typing_disabled"
  | "no_accessibility"
  | "type_error";

export interface ScanRecord {
  timestamp: string;
  reader: string;
  uidHex: string;
  uidLength: number;
  typed: boolean;
  typedString: string;
  status: ScanStatus;
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
};

export const onScan = (cb: (r: ScanRecord) => void): Promise<UnlistenFn> =>
  listen<ScanRecord>("scan", (e) => cb(e.payload));
export const onReadersChanged = (cb: (r: string[]) => void): Promise<UnlistenFn> =>
  listen<string[]>("readers-changed", (e) => cb(e.payload));
export const onReaderStatus = (cb: (s: ReaderStatus) => void): Promise<UnlistenFn> =>
  listen<ReaderStatus>("reader-status", (e) => cb(e.payload));
