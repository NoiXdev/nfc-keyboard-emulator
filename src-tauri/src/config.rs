use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Case {
    #[default]
    Upper,
    Lower,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ByteOrder {
    #[default]
    Normal,
    Reversed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Suffix {
    #[default]
    Enter,
    Tab,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatConfig {
    pub case: Case,
    pub separator: String,
    pub byte_order: ByteOrder,
    pub prefix: String,
    pub suffix: Suffix,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            case: Case::Upper,
            separator: String::new(),
            byte_order: ByteOrder::Normal,
            prefix: String::new(),
            suffix: Suffix::Enter,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub selected_reader: Option<String>,
    pub typing_enabled_on_start: bool,
    pub format: FormatConfig,
    pub start_minimized: bool,
    pub autostart: bool,
    pub log_retention: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            selected_reader: None,
            typing_enabled_on_start: false,
            format: FormatConfig::default(),
            start_minimized: false,
            autostart: false,
            log_retention: 500,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let c = Config::default();
        assert_eq!(c.log_retention, 500);
        assert!(!c.typing_enabled_on_start);
        assert_eq!(c.format.case, Case::Upper);
        assert_eq!(c.format.suffix, Suffix::Enter);
    }

    #[test]
    fn config_roundtrips_through_json_camelcase() {
        let c = Config::default();
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("\"typingEnabledOnStart\""));
        assert!(json.contains("\"logRetention\""));
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let back: Config = serde_json::from_str("{}").unwrap();
        assert_eq!(back, Config::default());
    }
}
