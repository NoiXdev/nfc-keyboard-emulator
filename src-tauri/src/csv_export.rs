use crate::scan::ScanRecord;

pub fn scans_to_csv(scans: &[ScanRecord]) -> String {
    let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
    wtr.write_record(["timestamp", "reader", "uidHex", "uidLength", "typed", "typedString", "status"])
        .expect("header");
    for s in scans {
        let status = serde_json::to_value(s.status)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        wtr.write_record([
            s.timestamp.as_str(),
            s.reader.as_str(),
            s.uid_hex.as_str(),
            &s.uid_length.to_string(),
            &s.typed.to_string(),
            s.typed_string.as_str(),
            status.as_str(),
        ])
        .expect("row");
    }
    let bytes = wtr.into_inner().expect("flush");
    String::from_utf8(bytes).expect("utf8")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::ScanStatus;

    fn rec(reader: &str) -> ScanRecord {
        ScanRecord {
            timestamp: "2026-06-19T20:15:00Z".into(),
            reader: reader.into(),
            uid_hex: "04A1B2C3".into(),
            uid_length: 4,
            typed: true,
            typed_string: "04A1B2C3\n".into(),
            status: ScanStatus::Ok,
        }
    }

    #[test]
    fn header_is_first_line() {
        let csv = scans_to_csv(&[]);
        assert!(csv.starts_with("timestamp,reader,uidHex,uidLength,typed,typedString,status"));
    }

    #[test]
    fn row_contains_values_and_status_string() {
        let csv = scans_to_csv(&[rec("ACR122U")]);
        assert!(csv.contains("ACR122U"));
        assert!(csv.contains("04A1B2C3"));
        assert!(csv.contains(",ok"));
    }

    #[test]
    fn fields_with_comma_are_quoted() {
        let csv = scans_to_csv(&[rec("Reader, Inc")]);
        assert!(csv.contains("\"Reader, Inc\""));
    }
}
