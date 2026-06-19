use crate::config::{ByteOrder, Case, FormatConfig, Suffix};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattedScan {
    pub body: String,
    pub suffix: Suffix,
}

pub fn format_uid(uid: &[u8], cfg: &FormatConfig) -> FormattedScan {
    let mut bytes = uid.to_vec();
    if cfg.byte_order == ByteOrder::Reversed {
        bytes.reverse();
    }
    let parts: Vec<String> = bytes
        .iter()
        .map(|b| match cfg.case {
            Case::Upper => format!("{b:02X}"),
            Case::Lower => format!("{b:02x}"),
        })
        .collect();
    let body = format!("{}{}", cfg.prefix, parts.join(&cfg.separator));
    FormattedScan {
        body,
        suffix: cfg.suffix,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UID4: [u8; 4] = [0x04, 0xA1, 0xB2, 0xC3];
    const UID7: [u8; 7] = [0x04, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66];

    fn cfg() -> FormatConfig {
        FormatConfig::default()
    }

    #[test]
    fn upper_no_separator_is_default() {
        assert_eq!(format_uid(&UID4, &cfg()).body, "04A1B2C3");
    }

    #[test]
    fn lowercase() {
        let c = FormatConfig { case: Case::Lower, ..cfg() };
        assert_eq!(format_uid(&UID4, &c).body, "04a1b2c3");
    }

    #[test]
    fn colon_separator() {
        let c = FormatConfig { separator: ":".into(), ..cfg() };
        assert_eq!(format_uid(&UID4, &c).body, "04:A1:B2:C3");
    }

    #[test]
    fn reversed_byte_order() {
        let c = FormatConfig { byte_order: ByteOrder::Reversed, ..cfg() };
        assert_eq!(format_uid(&UID4, &c).body, "C3B2A104");
    }

    #[test]
    fn prefix_is_prepended() {
        let c = FormatConfig { prefix: "ID".into(), ..cfg() };
        assert_eq!(format_uid(&UID4, &c).body, "ID04A1B2C3");
    }

    #[test]
    fn seven_byte_uid() {
        assert_eq!(format_uid(&UID7, &cfg()).body, "04112233445566");
    }

    #[test]
    fn suffix_is_passed_through() {
        let c = FormatConfig { suffix: Suffix::Tab, ..cfg() };
        assert_eq!(format_uid(&UID4, &c).suffix, Suffix::Tab);
    }
}
