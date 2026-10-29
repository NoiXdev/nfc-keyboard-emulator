use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::protocol::ScanPayload;

pub const MAX_TEMPLATE_BYTES: usize = 8192;

/// The names a `{{placeholder}}` may refer to; they match the `ScanPayload` wire fields.
pub const PLACEHOLDERS: [&str; 7] = [
    "uid",
    "uidFormatted",
    "uidLength",
    "scanId",
    "device",
    "reader",
    "timestamp",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PayloadMode {
    #[default]
    Standard,
    Template,
}

/// How a server target shapes the scan it sends. The template is only used in template mode,
/// but is kept in standard mode so switching back and forth loses nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PayloadConfig {
    pub mode: PayloadMode,
    pub template: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TemplateError {
    NotJson,
    UnknownPlaceholder { name: String },
    TooLarge,
}

pub fn validate_payload(cfg: &PayloadConfig) -> Result<(), TemplateError> {
    match cfg.mode {
        PayloadMode::Standard => Ok(()),
        PayloadMode::Template => validate_template(&cfg.template),
    }
}

pub fn validate_template(template: &str) -> Result<(), TemplateError> {
    parse_checked(template).map(|_| ())
}

/// Substitutes the scan into every string value of the template. Keys are never touched. A
/// string that is exactly one placeholder takes the placeholder's own JSON type (so
/// `"{{uidLength}}"` becomes a number); anywhere else the value is interpolated as text.
pub fn render_template(template: &str, payload: &ScanPayload) -> Result<Value, TemplateError> {
    let mut value = parse_checked(template)?;
    fill(&mut value, payload);
    Ok(value)
}

/// The scan sent by the "send test" action: fixed values, so a template can be tried without
/// holding a card to a reader.
pub fn example_payload(device: &str) -> ScanPayload {
    ScanPayload {
        scan_id: "test".to_string(),
        device: device.to_string(),
        reader: "Example Reader".to_string(),
        uid: "04A1B2C3D4E5F6".to_string(),
        uid_formatted: "04A1B2C3D4E5F6".to_string(),
        uid_length: 7,
        timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
    }
}

/// Size, then JSON syntax, then placeholder names (depth-first in document order; the first
/// unknown name is reported).
fn parse_checked(template: &str) -> Result<Value, TemplateError> {
    if template.len() > MAX_TEMPLATE_BYTES {
        return Err(TemplateError::TooLarge);
    }
    let value: Value = serde_json::from_str(template).map_err(|_| TemplateError::NotJson)?;
    check_placeholders(&value)?;
    Ok(value)
}

fn check_placeholders(value: &Value) -> Result<(), TemplateError> {
    match value {
        Value::String(s) => {
            for segment in segments(s) {
                if let Segment::Placeholder(name) = segment {
                    if !PLACEHOLDERS.contains(&name) {
                        return Err(TemplateError::UnknownPlaceholder {
                            name: name.to_string(),
                        });
                    }
                }
            }
            Ok(())
        }
        Value::Array(items) => items.iter().try_for_each(check_placeholders),
        Value::Object(map) => map.values().try_for_each(check_placeholders),
        _ => Ok(()),
    }
}

fn fill(value: &mut Value, payload: &ScanPayload) {
    match value {
        Value::String(s) => {
            let parts = segments(s);
            if let [Segment::Placeholder(name)] = parts.as_slice() {
                if let Some(v) = lookup(name, payload) {
                    *value = v;
                }
                return;
            }
            let mut out = String::with_capacity(s.len());
            for part in parts {
                match part {
                    Segment::Text(text) => out.push_str(text),
                    Segment::Placeholder(name) => match lookup(name, payload) {
                        Some(Value::String(text)) => out.push_str(&text),
                        Some(other) => out.push_str(&other.to_string()),
                        None => {}
                    },
                }
            }
            *s = out;
        }
        Value::Array(items) => items.iter_mut().for_each(|v| fill(v, payload)),
        Value::Object(map) => map.values_mut().for_each(|v| fill(v, payload)),
        _ => {}
    }
}

fn lookup(name: &str, payload: &ScanPayload) -> Option<Value> {
    Some(match name {
        "uid" => payload.uid.clone().into(),
        "uidFormatted" => payload.uid_formatted.clone().into(),
        "uidLength" => payload.uid_length.into(),
        "scanId" => payload.scan_id.clone().into(),
        "device" => payload.device.clone().into(),
        "reader" => payload.reader.clone().into(),
        "timestamp" => payload.timestamp.clone().into(),
        _ => return None,
    })
}

#[derive(Debug, PartialEq, Eq)]
enum Segment<'a> {
    Text(&'a str),
    Placeholder(&'a str),
}

/// Splits a string at `{{ name }}` placeholders: a name is letters, digits and underscores
/// (not starting with a digit), with optional ASCII whitespace inside the braces. Anything else, such
/// as `{{}}`, `{ {uid} }` or an unclosed `{{uid`, stays literal text.
fn segments(s: &str) -> Vec<Segment<'_>> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(open) = rest.find("{{") {
        let after = &rest[open + 2..];
        let Some(close) = after.find("}}") else { break };
        let name = after[..close].trim_matches(|c: char| c.is_ascii_whitespace());
        if is_identifier(name) {
            if open > 0 {
                out.push(Segment::Text(&rest[..open]));
            }
            out.push(Segment::Placeholder(name));
            rest = &after[close + 2..];
        } else {
            // Not a placeholder: keep the first brace and look again from the second, so
            // `{{{uid}}}` still finds `{{uid}}`.
            out.push(Segment::Text(&rest[..open + 1]));
            rest = &rest[open + 1..];
        }
    }
    if !rest.is_empty() {
        out.push(Segment::Text(rest));
    }
    out
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    #[derive(Deserialize)]
    struct Vector {
        name: String,
        template: String,
        payload: ScanPayload,
        expected: Option<serde_json::Value>,
        error: Option<serde_json::Value>,
    }

    #[test]
    fn shared_vectors() {
        let vectors: Vec<Vector> =
            serde_json::from_str(include_str!("../../../src/lib/templateVectors.json")).unwrap();
        assert!(!vectors.is_empty());
        for v in vectors {
            let result = render_template(&v.template, &v.payload);
            match (v.expected, v.error) {
                (Some(expected), None) => {
                    // Compare serialized strings so key order counts.
                    let got = result.unwrap_or_else(|e| panic!("{}: {e:?}", v.name));
                    assert_eq!(got.to_string(), expected.to_string(), "{}", v.name);
                    assert_eq!(validate_template(&v.template), Ok(()), "{}", v.name);
                }
                (None, Some(error)) => {
                    let got = result.expect_err(&v.name);
                    assert_eq!(serde_json::to_value(&got).unwrap(), error, "{}", v.name);
                    assert_eq!(validate_template(&v.template), Err(got), "{}", v.name);
                }
                _ => panic!("{}: exactly one of expected/error", v.name),
            }
        }
    }

    #[test]
    fn standard_mode_skips_validation() {
        let cfg = PayloadConfig {
            mode: PayloadMode::Standard,
            template: "nope".into(),
        };
        assert_eq!(validate_payload(&cfg), Ok(()));
    }

    #[test]
    fn template_mode_validates() {
        let mut cfg = PayloadConfig {
            mode: PayloadMode::Template,
            template: "nope".into(),
        };
        assert_eq!(validate_payload(&cfg), Err(TemplateError::NotJson));
        cfg.template = r#"{"uid": "{{uid}}"}"#.into();
        assert_eq!(validate_payload(&cfg), Ok(()));
    }

    #[test]
    fn example_payload_shape() {
        let p = example_payload("Kasse 1");
        assert_eq!(p.scan_id, "test");
        assert_eq!(p.device, "Kasse 1");
        assert_eq!(p.reader, "Example Reader");
        assert_eq!(p.uid, "04A1B2C3D4E5F6");
        assert_eq!(p.uid_formatted, "04A1B2C3D4E5F6");
        assert_eq!(p.uid_length, 7);
        assert!(p.timestamp.ends_with('Z'));
        assert!(DateTime::parse_from_rfc3339(&p.timestamp).is_ok());
        // Millisecond precision: `.mmmZ`.
        assert_eq!(p.timestamp.rfind('.').unwrap() + 5, p.timestamp.len());
    }
}
