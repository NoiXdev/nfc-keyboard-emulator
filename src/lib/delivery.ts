import type {
  ConfigError,
  DeliveryError,
  DeliveryResult,
  DeliveryTarget,
  ImportError,
  ScanRecord,
  WsStatus,
} from "./tauri";

export type DeliveryState =
  | { state: "pending" }
  | { state: "ok" }
  | { state: "failed"; error: DeliveryError };

export interface LoggedScan extends ScanRecord {
  delivery: Partial<Record<DeliveryTarget, DeliveryState>>;
}

type Translate = (key: string, named?: Record<string, unknown>) => string;

export function withPendingDelivery(r: ScanRecord): LoggedScan {
  const delivery: LoggedScan["delivery"] = {};
  for (const target of r.deliveryTargets) delivery[target] = { state: "pending" };
  return { ...r, delivery };
}

export function applyDelivery(log: LoggedScan[], result: DeliveryResult): LoggedScan[] {
  const state: DeliveryState = result.ok
    ? { state: "ok" }
    : { state: "failed", error: result.error ?? { kind: "unreachable", message: "" } };
  return log.map((row) =>
    row.scanId === result.scanId
      ? { ...row, delivery: { ...row.delivery, [result.target]: state } }
      : row,
  );
}

export function deliveryErrorText(t: Translate, e: DeliveryError): string {
  switch (e.kind) {
    case "unreachable":
      return t("delivery.error.unreachable", { message: e.message });
    case "token_rejected":
    case "http_status":
      return t(`delivery.error.${e.kind}`, { status: e.status });
    default:
      return t(`delivery.error.${e.kind}`);
  }
}

/** Translated text for a failed save or import; never the raw backend message. */
export function serverErrorText(t: Translate, e: ConfigError | ImportError): string {
  if (e.kind === "unsupported_version") {
    return t("server.error.unsupported_version", { version: e.version });
  }
  return t(`server.error.${e.kind}`);
}

export type WsNotice = { kind: "reconnecting"; error: DeliveryError } | { kind: "token_rejected" };

/** The websocket transitions worth a notification; repeats of the same state are quiet. */
export function wsNotice(prev: WsStatus | null, next: WsStatus): WsNotice | null {
  if (next.state === "token_rejected" && prev?.state !== "token_rejected") {
    return { kind: "token_rejected" };
  }
  if (next.state === "reconnecting" && prev?.state === "connected") {
    return { kind: "reconnecting", error: next.error };
  }
  return null;
}
