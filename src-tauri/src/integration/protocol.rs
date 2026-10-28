use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::scan::{DeliveryTarget, ScanRecord};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanPayload {
    pub scan_id: String,
    pub device: String,
    pub reader: String,
    pub uid: String,
    pub uid_formatted: String,
    pub uid_length: usize,
    pub timestamp: String,
}

pub fn scan_payload(rec: &ScanRecord, device: &str) -> ScanPayload {
    ScanPayload {
        scan_id: rec.scan_id.clone(),
        device: device.to_string(),
        reader: rec.reader.clone(),
        uid: rec.uid_hex.clone(),
        uid_formatted: rec.uid_formatted.clone(),
        uid_length: rec.uid_length,
        timestamp: wire_timestamp(&rec.timestamp),
    }
}

/// The protocol promises RFC 3339 in UTC with millisecond precision and a `Z` suffix; the
/// record's own timestamp keeps its finer local format. An unparsable value is passed through.
fn wire_timestamp(timestamp: &str) -> String {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|t| {
            t.with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Millis, true)
        })
        .unwrap_or_else(|_| timestamp.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Modes {
    pub stream: bool,
    pub request: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    Timeout,
    Cancelled,
    Superseded,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ClientMessage {
    Hello {
        protocol_version: u32,
        device: String,
        app_version: String,
        modes: Modes,
    },
    Scan {
        protocol_version: u32,
        #[serde(flatten)]
        payload: ScanPayload,
    },
    ScanResponse {
        protocol_version: u32,
        request_id: String,
        #[serde(flatten)]
        payload: ScanPayload,
    },
    ScanRequestEnded {
        request_id: String,
        reason: EndReason,
    },
    Test {
        protocol_version: u32,
        device: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ServerMessage {
    ScanRequest {
        request_id: String,
        timeout_ms: Option<u64>,
    },
    ScanRequestCancel {
        request_id: String,
    },
    #[serde(other)]
    Unknown,
}

/// Parses a text frame from the server. Anything that is not a JSON object with
/// a usable `type` (including known types with missing fields) yields `None`.
pub fn parse_server_message(text: &str) -> Option<ServerMessage> {
    serde_json::from_str(text).ok()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeliveryError {
    InvalidUrl,
    Unreachable { message: String },
    Timeout,
    TokenRejected { status: u16 },
    HttpStatus { status: u16 },
    NotConnected,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryResult {
    pub scan_id: String,
    pub target: DeliveryTarget,
    pub ok: bool,
    pub error: Option<DeliveryError>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::ScanStatus;
    use serde_json::{json, Value};

    fn record() -> ScanRecord {
        ScanRecord {
            timestamp: "2026-10-28T13:45:21.123Z".into(),
            reader: "ACS ACR122U PICC Interface".into(),
            uid_hex: "04A2B3C4D5E6F7".into(),
            uid_length: 7,
            typed: true,
            typed_string: "04A2B3C4D5E6F7\n".into(),
            status: ScanStatus::Ok,
            scan_id: "6f1c9a52-6c1e-4a59-9a43-1d1f5b0f2b7e".into(),
            uid_formatted: "04:A2:B3:C4:D5:E6:F7".into(),
            request_id: None,
            delivery_targets: vec![],
        }
    }

    fn to_value(msg: &ClientMessage) -> Value {
        serde_json::to_value(msg).unwrap()
    }

    #[test]
    fn scan_message_matches_spec() {
        let msg = ClientMessage::Scan {
            protocol_version: PROTOCOL_VERSION,
            payload: scan_payload(&record(), "Reception"),
        };
        assert_eq!(
            to_value(&msg),
            json!({
                "type": "scan",
                "protocolVersion": 1,
                "scanId": "6f1c9a52-6c1e-4a59-9a43-1d1f5b0f2b7e",
                "device": "Reception",
                "reader": "ACS ACR122U PICC Interface",
                "uid": "04A2B3C4D5E6F7",
                "uidFormatted": "04:A2:B3:C4:D5:E6:F7",
                "uidLength": 7,
                "timestamp": "2026-10-28T13:45:21.123Z"
            })
        );
    }

    #[test]
    fn payload_timestamp_is_utc_with_milliseconds() {
        let mut rec = record();
        rec.timestamp = "2026-10-28T13:45:21.123456789+00:00".into();
        assert_eq!(
            scan_payload(&rec, "Reception").timestamp,
            "2026-10-28T13:45:21.123Z"
        );
        // A non-UTC offset is converted, not just relabelled.
        rec.timestamp = "2026-10-28T15:45:21.5+02:00".into();
        assert_eq!(
            scan_payload(&rec, "Reception").timestamp,
            "2026-10-28T13:45:21.500Z"
        );
        // Unparsable values pass through unchanged.
        rec.timestamp = "yesterday".into();
        assert_eq!(scan_payload(&rec, "Reception").timestamp, "yesterday");
    }

    #[test]
    fn scan_response_carries_request_id_and_payload() {
        let msg = ClientMessage::ScanResponse {
            protocol_version: PROTOCOL_VERSION,
            request_id: "abc".into(),
            payload: scan_payload(&record(), "Reception"),
        };
        let v = to_value(&msg);
        assert_eq!(v["type"], "scan_response");
        assert_eq!(v["protocolVersion"], 1);
        assert_eq!(v["requestId"], "abc");
        assert_eq!(v["uid"], "04A2B3C4D5E6F7");
        assert_eq!(v["scanId"], "6f1c9a52-6c1e-4a59-9a43-1d1f5b0f2b7e");
    }

    #[test]
    fn hello_matches_spec() {
        let msg = ClientMessage::Hello {
            protocol_version: PROTOCOL_VERSION,
            device: "Reception".into(),
            app_version: "1.1.0".into(),
            modes: Modes {
                stream: true,
                request: true,
            },
        };
        assert_eq!(
            to_value(&msg),
            json!({
                "type": "hello",
                "protocolVersion": 1,
                "device": "Reception",
                "appVersion": "1.1.0",
                "modes": {"stream": true, "request": true}
            })
        );
    }

    #[test]
    fn test_message_shape() {
        let msg = ClientMessage::Test {
            protocol_version: PROTOCOL_VERSION,
            device: "Reception".into(),
        };
        assert_eq!(
            to_value(&msg),
            json!({"type": "test", "protocolVersion": 1, "device": "Reception"})
        );
    }

    #[test]
    fn request_ended_reasons() {
        for (reason, expected) in [
            (EndReason::Timeout, "timeout"),
            (EndReason::Cancelled, "cancelled"),
            (EndReason::Superseded, "superseded"),
            (EndReason::Disabled, "disabled"),
        ] {
            let msg = ClientMessage::ScanRequestEnded {
                request_id: "abc".into(),
                reason,
            };
            assert_eq!(
                to_value(&msg),
                json!({"type": "scan_request_ended", "requestId": "abc", "reason": expected})
            );
        }
    }

    #[test]
    fn parses_scan_request_with_and_without_timeout() {
        assert_eq!(
            parse_server_message(r#"{"type":"scan_request","requestId":"r1","timeoutMs":5000}"#),
            Some(ServerMessage::ScanRequest {
                request_id: "r1".into(),
                timeout_ms: Some(5000)
            })
        );
        assert_eq!(
            parse_server_message(r#"{"type":"scan_request","requestId":"r2"}"#),
            Some(ServerMessage::ScanRequest {
                request_id: "r2".into(),
                timeout_ms: None
            })
        );
        assert_eq!(
            parse_server_message(r#"{"type":"scan_request_cancel","requestId":"r2"}"#),
            Some(ServerMessage::ScanRequestCancel {
                request_id: "r2".into()
            })
        );
    }

    #[test]
    fn unknown_type_is_unknown_not_error() {
        assert_eq!(
            parse_server_message(r#"{"type":"welcome"}"#),
            Some(ServerMessage::Unknown)
        );
    }

    #[test]
    fn garbage_is_ignored() {
        assert_eq!(parse_server_message("not json"), None);
        assert_eq!(parse_server_message(r#"{"type":"scan_request"}"#), None);
        assert_eq!(parse_server_message("[]"), None);
        assert_eq!(parse_server_message(""), None);
        assert_eq!(parse_server_message("{}"), None);
    }

    #[test]
    fn delivery_error_shape() {
        let v = serde_json::to_value(DeliveryError::TokenRejected { status: 401 }).unwrap();
        assert_eq!(v, json!({"kind": "token_rejected", "status": 401}));
        let v = serde_json::to_value(DeliveryError::Unreachable {
            message: "boom".into(),
        })
        .unwrap();
        assert_eq!(v, json!({"kind": "unreachable", "message": "boom"}));
        let v = serde_json::to_value(DeliveryError::NotConnected).unwrap();
        assert_eq!(v, json!({"kind": "not_connected"}));
    }

    #[test]
    fn delivery_result_shape() {
        let r = DeliveryResult {
            scan_id: "s".into(),
            target: DeliveryTarget::Ws,
            ok: false,
            error: Some(DeliveryError::Timeout),
        };
        assert_eq!(
            serde_json::to_value(r).unwrap(),
            json!({"scanId": "s", "target": "ws", "ok": false, "error": {"kind": "timeout"}})
        );
    }
}
