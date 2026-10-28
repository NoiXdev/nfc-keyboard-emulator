use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatus {
    Ok,
    ReadError,
    TypingDisabled,
    NoAccessibility,
    TypeError,
    /// The scan answered a server request and was deliberately not typed.
    ServerRequest,
}

// Variants are constructed by the delivery pipeline added in a later task.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryTarget {
    Http,
    Ws,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRecord {
    pub timestamp: String,
    pub reader: String,
    pub uid_hex: String,
    pub uid_length: usize,
    pub typed: bool,
    pub typed_string: String,
    pub status: ScanStatus,
    pub scan_id: String,
    pub uid_formatted: String,
    pub request_id: Option<String>,
    pub delivery_targets: Vec<DeliveryTarget>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_camelcase_and_snakecase_status() {
        let r = ScanRecord {
            timestamp: "2026-06-19T20:15:00Z".into(),
            reader: "ACR122U".into(),
            uid_hex: "04A1B2C3".into(),
            uid_length: 4,
            typed: true,
            typed_string: "04A1B2C3\n".into(),
            status: ScanStatus::Ok,
            scan_id: "id".into(),
            uid_formatted: "04A1B2C3".into(),
            request_id: None,
            delivery_targets: vec![],
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"uidHex\":\"04A1B2C3\""));
        assert!(json.contains("\"uidLength\":4"));
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"scanId\":\"id\""));
        assert!(json.contains("\"uidFormatted\":\"04A1B2C3\""));
        assert!(json.contains("\"requestId\":null"));
        assert!(json.contains("\"deliveryTargets\":[]"));
    }

    #[test]
    fn no_accessibility_status_is_snake_case() {
        let json = serde_json::to_string(&ScanStatus::NoAccessibility).unwrap();
        assert_eq!(json, "\"no_accessibility\"");
    }

    #[test]
    fn server_request_status_is_snake_case() {
        let json = serde_json::to_string(&ScanStatus::ServerRequest).unwrap();
        assert_eq!(json, "\"server_request\"");
    }

    #[test]
    fn delivery_target_is_snake_case() {
        let json = serde_json::to_string(&[DeliveryTarget::Http, DeliveryTarget::Ws]).unwrap();
        assert_eq!(json, "[\"http\",\"ws\"]");
    }
}
