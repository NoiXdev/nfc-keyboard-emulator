use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatus {
    Ok,
    ReadError,
    TypingDisabled,
    NoAccessibility,
    TypeError,
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
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"uidHex\":\"04A1B2C3\""));
        assert!(json.contains("\"uidLength\":4"));
        assert!(json.contains("\"status\":\"ok\""));
    }

    #[test]
    fn no_accessibility_status_is_snake_case() {
        let json = serde_json::to_string(&ScanStatus::NoAccessibility).unwrap();
        assert_eq!(json, "\"no_accessibility\"");
    }
}
