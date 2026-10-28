import type { DeliveryError, DeliveryResult, DeliveryTarget, ScanRecord } from "./tauri";

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
