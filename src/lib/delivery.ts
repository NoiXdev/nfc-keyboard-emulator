import type { TemplateError } from "./template";
import type {
  ConfigError,
  DeliveryError,
  DeliveryResult,
  DeliveryTarget,
  HttpTarget,
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
    case "invalid_auth":
      return t(`delivery.error.invalid_auth_${e.field}`);
    default:
      return t(`delivery.error.${e.kind}`);
  }
}

/** Translated text for a template that cannot be applied, without a target prefix. */
export function templateErrorText(t: Translate, e: TemplateError): string {
  if (e.kind === "unknown_placeholder") {
    return t("server.error.template_unknown_placeholder", { name: e.name });
  }
  return t(`server.error.template_${e.kind}`);
}

/** Translated text for a failed save or import; never the raw backend message. */
export function serverErrorText(t: Translate, e: ConfigError | ImportError): string {
  switch (e.kind) {
    case "unsupported_version":
      return t("server.error.unsupported_version", { version: e.version });
    case "invalid_http_template":
    case "invalid_ws_template":
      return t(`server.error.${e.kind}`, { error: templateErrorText(t, e.error) });
    default:
      return t(`server.error.${e.kind}`);
  }
}

/** HTTP push only sends when it is on and has an address; the footer must say the same. */
export function httpPushActive(http: Pick<HttpTarget, "enabled" | "url"> | undefined): boolean {
  return !!http && http.enabled && http.url.trim() !== "";
}

export type WsNotice = { kind: "reconnecting"; error: DeliveryError } | { kind: "token_rejected" };

/**
 * The websocket transitions worth a notification; repeats of the same state are quiet. A drop
 * only counts when the backend marks it `lost`, so a server that keeps accepting and closing
 * the connection is announced once, not on every attempt.
 */
export function wsNotice(prev: WsStatus | null, next: WsStatus): WsNotice | null {
  if (next.state === "token_rejected" && prev?.state !== "token_rejected") {
    return { kind: "token_rejected" };
  }
  if (next.state === "reconnecting" && next.lost && prev?.state === "connected") {
    return { kind: "reconnecting", error: next.error };
  }
  return null;
}

/**
 * Whether a scan request deserves a notice. The backend announces every scan_request, also
 * a repeated id, so the id last notified is remembered until the request is no longer pending.
 */
export function requestNotice(
  lastNotified: string | null,
  requestId: string,
): { notify: boolean; lastNotified: string } {
  return { notify: requestId !== lastNotified, lastNotified: requestId };
}
