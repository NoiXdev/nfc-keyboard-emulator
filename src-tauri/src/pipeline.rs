use crate::config::{Config, Suffix};
use crate::formatter::format_uid;
use crate::scan::{ScanRecord, ScanStatus};
use crate::typer::{TypeError, Typer};
use uuid::Uuid;

fn uid_hex_upper(uid: &[u8]) -> String {
    uid.iter().map(|b| format!("{b:02X}")).collect()
}

pub fn handle_scan(
    uid: &[u8],
    reader: &str,
    timestamp: String,
    cfg: &Config,
    typing_enabled: bool,
    answering: Option<String>,
    typer: &mut dyn Typer,
) -> ScanRecord {
    let formatted = format_uid(uid, &cfg.format);
    let suffix_repr = match formatted.suffix {
        Suffix::Enter => "\n",
        Suffix::Tab => "\t",
        Suffix::None => "",
    };
    let typed_string = format!("{}{}", formatted.body, suffix_repr);

    let (typed, status) = if answering.is_some() {
        (false, ScanStatus::ServerRequest)
    } else if !typing_enabled {
        (false, ScanStatus::TypingDisabled)
    } else {
        match typer.type_scan(&formatted) {
            Ok(()) => (true, ScanStatus::Ok),
            Err(TypeError::NoAccessibility) => (false, ScanStatus::NoAccessibility),
            Err(TypeError::Other(_)) => (false, ScanStatus::TypeError),
        }
    };

    ScanRecord {
        timestamp,
        reader: reader.to_string(),
        uid_hex: uid_hex_upper(uid),
        uid_length: uid.len(),
        typed,
        typed_string,
        status,
        scan_id: Uuid::new_v4().to_string(),
        uid_formatted: formatted.body,
        request_id: answering,
        delivery_targets: Vec::new(),
    }
}

pub fn read_error_record(reader: &str, timestamp: String) -> ScanRecord {
    ScanRecord {
        timestamp,
        reader: reader.to_string(),
        uid_hex: String::new(),
        uid_length: 0,
        typed: false,
        typed_string: String::new(),
        status: ScanStatus::ReadError,
        scan_id: Uuid::new_v4().to_string(),
        uid_formatted: String::new(),
        request_id: None,
        delivery_targets: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typer::mock::MockTyper;

    const UID: [u8; 4] = [0x04, 0xA1, 0xB2, 0xC3];

    fn ts() -> String {
        "2026-06-19T20:15:00Z".into()
    }

    #[test]
    fn typing_disabled_does_not_call_typer() {
        let mut t = MockTyper::ok();
        let r = handle_scan(&UID, "R", ts(), &Config::default(), false, None, &mut t);
        assert_eq!(t.call_count(), 0);
        assert!(!r.typed);
        assert_eq!(r.status, ScanStatus::TypingDisabled);
        assert_eq!(r.uid_hex, "04A1B2C3");
        assert_eq!(r.typed_string, "04A1B2C3\n");
    }

    #[test]
    fn typing_enabled_calls_typer_and_marks_ok() {
        let mut t = MockTyper::ok();
        let r = handle_scan(&UID, "R", ts(), &Config::default(), true, None, &mut t);
        assert_eq!(t.call_count(), 1);
        assert!(r.typed);
        assert_eq!(r.status, ScanStatus::Ok);
    }

    #[test]
    fn answering_a_request_skips_typing() {
        let mut t = MockTyper::ok();
        let r = handle_scan(
            &UID,
            "R",
            ts(),
            &Config::default(),
            true,
            Some("abc".into()),
            &mut t,
        );
        assert_eq!(t.call_count(), 0);
        assert_eq!(r.status, ScanStatus::ServerRequest);
        assert_eq!(r.request_id, Some("abc".to_string()));
        assert!(!r.typed);
    }

    #[test]
    fn answering_wins_over_typing_disabled() {
        let mut t = MockTyper::ok();
        let r = handle_scan(
            &UID,
            "R",
            ts(),
            &Config::default(),
            false,
            Some("abc".into()),
            &mut t,
        );
        assert_eq!(t.call_count(), 0);
        assert_eq!(r.status, ScanStatus::ServerRequest);
        assert_eq!(r.request_id, Some("abc".to_string()));
    }

    #[test]
    fn uid_formatted_is_body_without_suffix() {
        let mut cfg = Config::default();
        cfg.format.prefix = "ID:".into();
        cfg.format.separator = ":".into();
        let mut t = MockTyper::ok();
        let r = handle_scan(&UID, "R", ts(), &cfg, false, None, &mut t);
        assert_eq!(r.uid_formatted, "ID:04:A1:B2:C3");
        assert_eq!(r.uid_hex, "04A1B2C3");
        assert_eq!(r.request_id, None);
        assert!(r.delivery_targets.is_empty());
    }

    #[test]
    fn each_scan_gets_a_distinct_uuid() {
        let mut t = MockTyper::ok();
        let a = handle_scan(&UID, "R", ts(), &Config::default(), false, None, &mut t);
        let b = handle_scan(&UID, "R", ts(), &Config::default(), false, None, &mut t);
        assert_ne!(a.scan_id, b.scan_id);
        assert!(uuid::Uuid::parse_str(&a.scan_id).is_ok());
        assert!(uuid::Uuid::parse_str(&b.scan_id).is_ok());
    }

    #[test]
    fn no_accessibility_maps_to_status() {
        let mut t = MockTyper::failing(TypeError::NoAccessibility);
        let r = handle_scan(&UID, "R", ts(), &Config::default(), true, None, &mut t);
        assert!(!r.typed);
        assert_eq!(r.status, ScanStatus::NoAccessibility);
    }

    #[test]
    fn enigo_error_maps_to_type_error() {
        let mut t = MockTyper::failing(TypeError::Other("boom".into()));
        let r = handle_scan(&UID, "R", ts(), &Config::default(), true, None, &mut t);
        assert_eq!(r.status, ScanStatus::TypeError);
    }

    #[test]
    fn read_error_record_has_empty_uid() {
        let r = read_error_record("R", ts());
        assert_eq!(r.status, ScanStatus::ReadError);
        assert_eq!(r.uid_length, 0);
        assert!(r.uid_hex.is_empty());
        assert!(r.uid_formatted.is_empty());
        assert_eq!(r.request_id, None);
        assert!(uuid::Uuid::parse_str(&r.scan_id).is_ok());
    }
}
