use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use url::Url;

use super::config::{valid_url, IntegrationConfig};
use super::http::origin_of;

pub const FORMAT_ID: &str = "nfc-keyboard-emulator/server-config";
pub const FORMAT_VERSION: u32 = 1;
pub const SCHEMA_URL: &str =
    "https://docs.noix.dev/nfc-keyboard-emulator/server-config.v1.schema.json";

const LINK_SCHEME: &str = "nfc-keyboard-emulator";
const LINK_ACTION: &str = "configure";
const LINK_PARAM: &str = "config";

/// The import/export file. Every key except `format` and `version` is optional: an absent
/// key leaves the setting unchanged, a present one (even `""`) replaces it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfigFile {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub format: String,
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpPatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ws: Option<WsPatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ImportError {
    InvalidJson { message: String },
    WrongFormat,
    UnsupportedVersion { version: u32 },
    InvalidHttpUrl,
    InvalidWsUrl,
    InvalidLink,
}

/// Parses and validates a config file. Strings are trimmed like `normalize` does; unknown
/// keys are ignored. Error messages never echo file content, since it may hold tokens.
pub fn parse_config_file(text: &str) -> Result<ServerConfigFile, ImportError> {
    let mut value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| ImportError::InvalidJson {
            message: match e.classify() {
                serde_json::error::Category::Data => "unexpected value".to_string(),
                _ => e.to_string(),
            },
        })?;

    if value.get("format").and_then(|f| f.as_str()) != Some(FORMAT_ID) {
        return Err(ImportError::WrongFormat);
    }
    // JSON Schema treats `1.0` as the integer 1, so the parser accepts it too.
    if let Some(f) = value.get("version").and_then(|v| v.as_f64()) {
        if f.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&f) {
            value["version"] = serde_json::Value::from(f as u64);
        }
    }
    match value.get("version").and_then(|v| v.as_u64()) {
        Some(v) if v > u64::from(FORMAT_VERSION) => {
            return Err(ImportError::UnsupportedVersion {
                version: u32::try_from(v).unwrap_or(u32::MAX),
            })
        }
        Some(v) if v >= 1 => {}
        _ => {
            return Err(ImportError::InvalidJson {
                message: "version must be a positive integer".to_string(),
            })
        }
    }

    let mut file: ServerConfigFile =
        serde_json::from_value(value).map_err(|_| ImportError::InvalidJson {
            message: "a value has the wrong type".to_string(),
        })?;

    for s in [&mut file.device_name].into_iter().flatten() {
        trim_in_place(s);
    }
    if let Some(http) = &mut file.http {
        for s in [&mut http.url, &mut http.token].into_iter().flatten() {
            trim_in_place(s);
        }
    }
    if let Some(ws) = &mut file.ws {
        for s in [&mut ws.url, &mut ws.token].into_iter().flatten() {
            trim_in_place(s);
        }
    }

    let http_url = file.http.as_ref().and_then(|h| h.url.as_deref());
    if !valid_url(http_url.unwrap_or(""), &["http", "https"]) {
        return Err(ImportError::InvalidHttpUrl);
    }
    let ws_url = file.ws.as_ref().and_then(|w| w.url.as_deref());
    if !valid_url(ws_url.unwrap_or(""), &["ws", "wss"]) {
        return Err(ImportError::InvalidWsUrl);
    }
    Ok(file)
}

fn trim_in_place(s: &mut String) {
    *s = s.trim().to_string();
}

/// Absent key keeps the current value; a present key (including `""`) replaces it.
pub fn apply(cfg: &IntegrationConfig, f: &ServerConfigFile) -> IntegrationConfig {
    let mut out = cfg.clone();
    if let Some(v) = &f.device_name {
        out.device_name = v.clone();
    }
    if let Some(http) = &f.http {
        if let Some(v) = http.enabled {
            out.http.enabled = v;
        }
        if let Some(v) = &http.url {
            out.http.url = v.clone();
        }
        if let Some(v) = &http.token {
            out.http.token = v.clone();
        }
    }
    if let Some(ws) = &f.ws {
        if let Some(v) = &ws.url {
            out.ws.url = v.clone();
        }
        if let Some(v) = &ws.token {
            out.ws.token = v.clone();
        }
        if let Some(v) = ws.stream {
            out.ws.stream = v;
        }
        if let Some(v) = ws.request {
            out.ws.request = v;
        }
    }
    out
}

/// The current settings as a config file, tokens included.
pub fn export(cfg: &IntegrationConfig) -> ServerConfigFile {
    ServerConfigFile {
        schema: Some(SCHEMA_URL.to_string()),
        format: FORMAT_ID.to_string(),
        version: FORMAT_VERSION,
        device_name: Some(cfg.device_name.clone()),
        http: Some(HttpPatch {
            enabled: Some(cfg.http.enabled),
            url: Some(cfg.http.url.clone()),
            token: Some(cfg.http.token.clone()),
        }),
        ws: Some(WsPatch {
            url: Some(cfg.ws.url.clone()),
            token: Some(cfg.ws.token.clone()),
            stream: Some(cfg.ws.stream),
            request: Some(cfg.ws.request),
        }),
    }
}

/// Decodes `nfc-keyboard-emulator://configure?config=<base64(JSON)>`. Accepts the URL-safe
/// and the standard alphabet, with or without padding, optionally percent-encoded.
pub fn decode_link(url: &str) -> Result<ServerConfigFile, ImportError> {
    let url = Url::parse(url.trim()).map_err(|_| ImportError::InvalidLink)?;
    if url.scheme() != LINK_SCHEME {
        return Err(ImportError::InvalidLink);
    }
    // `scheme://configure?...` has the action as host; `scheme:configure?...` as path.
    let action = match url.host_str() {
        Some(host) => host,
        None => url.path(),
    };
    if !action.trim_matches('/').eq_ignore_ascii_case(LINK_ACTION) {
        return Err(ImportError::InvalidLink);
    }

    // Not `query_pairs`: it would turn a raw '+' of the standard alphabet into a space.
    let raw = url
        .query()
        .and_then(|q| {
            q.split('&')
                .filter_map(|pair| pair.split_once('='))
                .find(|(k, _)| *k == LINK_PARAM)
                .map(|(_, v)| v)
        })
        .ok_or(ImportError::InvalidLink)?;
    let encoded = percent_decode_str(raw)
        .decode_utf8()
        .map_err(|_| ImportError::InvalidLink)?;

    let normalized: String = encoded
        .trim()
        .trim_end_matches('=')
        .chars()
        .map(|c| match c {
            '+' => '-',
            '/' => '_',
            c => c,
        })
        .collect();
    let bytes = URL_SAFE_NO_PAD
        .decode(normalized)
        .map_err(|_| ImportError::InvalidLink)?;
    let text = String::from_utf8(bytes).map_err(|_| ImportError::InvalidJson {
        message: "not valid UTF-8".to_string(),
    })?;
    parse_config_file(&text)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportSource {
    File,
    Link,
}

/// What applying the import does to a token. The token value itself is never exposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenChange {
    Unchanged,
    Set,
    Cleared,
}

/// What the import dialog shows. Values describe the state after applying, hosts also the
/// state before (as `scheme://host[:port]`); no tokens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub id: String,
    pub source: ImportSource,
    pub device_name: String,
    pub http_enabled: bool,
    pub http_host_before: Option<String>,
    pub http_host_after: Option<String>,
    pub http_token: TokenChange,
    /// The stored, non-empty HTTP token is kept but goes to another scheme or host.
    pub http_token_to_new_host: bool,
    /// `https` becomes `http`.
    pub http_downgrade: bool,
    pub ws_host_before: Option<String>,
    pub ws_host_after: Option<String>,
    pub ws_stream: bool,
    pub ws_request: bool,
    pub ws_token: TokenChange,
    /// The stored, non-empty WebSocket token is kept but goes to another scheme or host.
    pub ws_token_to_new_host: bool,
    /// `wss` becomes `ws`.
    pub ws_downgrade: bool,
}

pub fn preview(
    current: &IntegrationConfig,
    f: &ServerConfigFile,
    source: ImportSource,
) -> ImportPreview {
    let after = apply(current, f);
    let http_token = f.http.as_ref().and_then(|h| h.token.as_deref());
    let ws_token = f.ws.as_ref().and_then(|w| w.token.as_deref());
    let http = HostChange::new(
        (&current.http.url, &current.http.token),
        (&after.http.url, &after.http.token),
        "https",
    );
    let ws = HostChange::new(
        (&current.ws.url, &current.ws.token),
        (&after.ws.url, &after.ws.token),
        "wss",
    );
    ImportPreview {
        id: uuid::Uuid::new_v4().to_string(),
        source,
        device_name: after.effective_device_name(),
        http_enabled: after.http.enabled,
        http_host_before: http.before,
        http_host_after: http.after,
        http_token: token_change(&current.http.token, http_token),
        http_token_to_new_host: http.token_to_new_host,
        http_downgrade: http.downgrade,
        ws_host_before: ws.before,
        ws_host_after: ws.after,
        ws_stream: after.ws.stream,
        ws_request: after.ws.request,
        ws_token: token_change(&current.ws.token, ws_token),
        ws_token_to_new_host: ws.token_to_new_host,
        ws_downgrade: ws.downgrade,
    }
}

/// What an import does to one target's address.
struct HostChange {
    before: Option<String>,
    after: Option<String>,
    token_to_new_host: bool,
    downgrade: bool,
}

impl HostChange {
    /// `before` and `after` are `(url, token)`; `secure` is the scheme that encrypts.
    fn new(before: (&str, &str), after: (&str, &str), secure: &str) -> Self {
        let old = Url::parse(before.0).ok().filter(Url::has_host);
        let new = Url::parse(after.0).ok().filter(Url::has_host);
        let old_origin = old.as_ref().map(origin_of);
        let new_origin = new.as_ref().map(origin_of);
        let token_kept = !before.1.is_empty() && after.1 == before.1;
        Self {
            token_to_new_host: token_kept && new_origin.is_some() && new_origin != old_origin,
            downgrade: old.is_some_and(|u| u.scheme() == secure)
                && new.is_some_and(|u| u.scheme() != secure),
            before: old_origin,
            after: new_origin,
        }
    }
}

fn token_change(current: &str, incoming: Option<&str>) -> TokenChange {
    match incoming {
        None => TokenChange::Unchanged,
        Some(new) if new == current => TokenChange::Unchanged,
        Some("") => TokenChange::Cleared,
        Some(_) => TokenChange::Set,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::config::{HttpTarget, WsTarget};
    use base64::engine::general_purpose::{STANDARD, URL_SAFE};

    fn sample() -> IntegrationConfig {
        IntegrationConfig {
            device_name: "Reception".into(),
            http: HttpTarget {
                enabled: true,
                url: "https://example.com/api/scans".into(),
                token: "http-secret".into(),
            },
            ws: WsTarget {
                url: "wss://example.com:8443/scanner".into(),
                token: "ws-secret".into(),
                stream: true,
                request: false,
            },
        }
    }

    fn json(f: &ServerConfigFile) -> String {
        serde_json::to_string(f).unwrap()
    }

    #[test]
    fn export_then_parse_roundtrips() {
        let cfg = sample();
        let file = export(&cfg);
        assert_eq!(file.schema.as_deref(), Some(SCHEMA_URL));
        let parsed = parse_config_file(&json(&file)).unwrap();
        assert_eq!(parsed, file);
        assert_eq!(apply(&IntegrationConfig::default(), &parsed), cfg);
    }

    #[test]
    fn absent_keys_keep_values() {
        let mut cfg = sample();
        cfg.http.token = "old".into();
        let f = parse_config_file(
            r#"{"format":"nfc-keyboard-emulator/server-config","version":1,
                "http":{"url":"https://new.example.com/hook"}}"#,
        )
        .unwrap();
        let out = apply(&cfg, &f);
        assert_eq!(out.http.url, "https://new.example.com/hook");
        assert_eq!(out.http.token, "old");
        assert!(out.http.enabled);
        assert_eq!(out.device_name, "Reception");
        assert_eq!(out.ws, cfg.ws);

        let f = parse_config_file(
            r#"{"format":"nfc-keyboard-emulator/server-config","version":1,
                "http":{"token":""}}"#,
        )
        .unwrap();
        assert_eq!(apply(&cfg, &f).http.token, "");
    }

    #[test]
    fn rejects_other_format_and_newer_version() {
        assert_eq!(
            parse_config_file(r#"{"format":"nfc-keyboard-emulator/server-config","version":2}"#),
            Err(ImportError::UnsupportedVersion { version: 2 })
        );
        assert_eq!(
            parse_config_file(r#"{"format":"x","version":1}"#),
            Err(ImportError::WrongFormat)
        );
        assert_eq!(
            parse_config_file(r#"{"version":1}"#),
            Err(ImportError::WrongFormat)
        );
        assert!(matches!(
            parse_config_file("not json"),
            Err(ImportError::InvalidJson { .. })
        ));
    }

    #[test]
    fn rejects_bad_schemes() {
        let with = |body: &str| {
            parse_config_file(&format!(
                r#"{{"format":"nfc-keyboard-emulator/server-config","version":1,{body}}}"#
            ))
        };
        assert_eq!(
            with(r#""http":{"url":"ws://x.test"}"#),
            Err(ImportError::InvalidHttpUrl)
        );
        assert_eq!(
            with(r#""http":{"url":"example.com/api"}"#),
            Err(ImportError::InvalidHttpUrl)
        );
        assert_eq!(
            with(r#""ws":{"url":"https://x.test"}"#),
            Err(ImportError::InvalidWsUrl)
        );
        assert!(with(r#""http":{"url":""},"ws":{"url":""}"#).is_ok());
    }

    #[test]
    fn trims_pasted_values() {
        let f = parse_config_file(
            r#"{"format":"nfc-keyboard-emulator/server-config","version":1,
                "deviceName":" Desk ","http":{"url":" https://x.test/a\n","token":" t "}}"#,
        )
        .unwrap();
        assert_eq!(f.device_name.as_deref(), Some("Desk"));
        let http = f.http.unwrap();
        assert_eq!(http.url.as_deref(), Some("https://x.test/a"));
        assert_eq!(http.token.as_deref(), Some("t"));
    }

    #[test]
    fn ignores_unknown_keys() {
        let f = parse_config_file(
            r#"{"format":"nfc-keyboard-emulator/server-config","version":1,"future":{"a":1},
                "http":{"enabled":true,"retries":3},"ws":{"compression":"x"}}"#,
        )
        .unwrap();
        assert_eq!(f.http.unwrap().enabled, Some(true));
        assert_eq!(f.ws.unwrap().url, None);
    }

    #[test]
    fn invalid_json_message_does_not_leak_values() {
        let err = parse_config_file(
            r#"{"format":"nfc-keyboard-emulator/server-config","version":1,
                "http":{"enabled":"super-secret-value"}}"#,
        )
        .unwrap_err();
        assert!(!serde_json::to_string(&err)
            .unwrap()
            .contains("super-secret"));
        assert!(matches!(err, ImportError::InvalidJson { .. }));
    }

    #[test]
    fn link_decoding_is_tolerant() {
        // '>' and '?' make the standard alphabet differ from the URL-safe one.
        let mut cfg = sample();
        cfg.device_name = "Front>>desk???".into();
        let file = export(&cfg);
        let raw = json(&file);
        let bytes = raw.as_bytes();

        let std_enc = STANDARD.encode(bytes);
        assert!(std_enc.contains('+') || std_enc.contains('/'));
        assert!(std_enc.contains('='), "pick a payload that needs padding");
        let encoded = [
            URL_SAFE_NO_PAD.encode(bytes),
            URL_SAFE.encode(bytes),
            std_enc.clone(),
            std_enc
                .replace('+', "%2B")
                .replace('/', "%2F")
                .replace('=', "%3D"),
        ];
        for enc in encoded {
            let link = format!("nfc-keyboard-emulator://configure?config={enc}");
            assert_eq!(decode_link(&link), Ok(file.clone()), "{enc}");
        }
        // Trailing slash after the action, other parameters and a fragment are fine.
        let link = format!(
            "nfc-keyboard-emulator://configure/?x=1&config={}#frag",
            URL_SAFE_NO_PAD.encode(bytes)
        );
        assert_eq!(decode_link(&link), Ok(file));
    }

    #[test]
    fn link_rejects_other_links() {
        let enc = URL_SAFE_NO_PAD.encode(json(&export(&sample())));
        for link in [
            format!("nfc-keyboard-emulator://other?config={enc}"),
            format!("https://configure?config={enc}"),
            format!("other-app://configure?config={enc}"),
            "nfc-keyboard-emulator://configure".to_string(),
            "nfc-keyboard-emulator://configure?config=!!!".to_string(),
            "not a url".to_string(),
        ] {
            assert_eq!(decode_link(&link), Err(ImportError::InvalidLink), "{link}");
        }
        // The payload is validated like a file.
        let bad = URL_SAFE_NO_PAD.encode(r#"{"format":"x","version":1}"#);
        assert_eq!(
            decode_link(&format!("nfc-keyboard-emulator://configure?config={bad}")),
            Err(ImportError::WrongFormat)
        );
    }

    #[test]
    fn preview_masks_tokens() {
        let current = sample();
        let f = parse_config_file(
            r#"{"format":"nfc-keyboard-emulator/server-config","version":1,
                "deviceName":"Kiosk",
                "http":{"url":"https://new.example.com:9000/api/scans?key=abc","token":"brand-new-secret"},
                "ws":{"stream":false,"request":true,"token":""}}"#,
        )
        .unwrap();
        let p = preview(&current, &f, ImportSource::Link);
        assert_eq!(p.source, ImportSource::Link);
        assert_eq!(p.device_name, "Kiosk");
        assert!(p.http_enabled);
        assert_eq!(p.http_host_before.as_deref(), Some("https://example.com"));
        assert_eq!(
            p.http_host_after.as_deref(),
            Some("https://new.example.com:9000")
        );
        assert_eq!(p.http_token, TokenChange::Set);
        assert_eq!(p.ws_host_before.as_deref(), Some("wss://example.com:8443"));
        assert_eq!(p.ws_host_after.as_deref(), Some("wss://example.com:8443"));
        assert!(!p.ws_stream && p.ws_request);
        assert_eq!(p.ws_token, TokenChange::Cleared);
        assert!(!p.id.is_empty());

        let out = serde_json::to_string(&p).unwrap();
        for secret in ["brand-new-secret", "http-secret", "ws-secret", "key=abc"] {
            assert!(!out.contains(secret), "{secret} leaked");
        }
        assert!(out.contains(r#""httpToken":"set""#));
        assert!(out.contains(r#""source":"link""#));

        let only_url = parse_config_file(
            r#"{"format":"nfc-keyboard-emulator/server-config","version":1,
                "http":{"url":"https://other.example.com/x"}}"#,
        )
        .unwrap();
        let p = preview(&current, &only_url, ImportSource::File);
        assert_eq!(p.http_token, TokenChange::Unchanged);
        assert_eq!(p.ws_token, TokenChange::Unchanged);
        assert_eq!(p.device_name, "Reception");
    }

    #[test]
    fn preview_uses_effective_device_name_and_same_token_is_unchanged() {
        let mut current = sample();
        current.device_name = String::new();
        let f = parse_config_file(
            r#"{"format":"nfc-keyboard-emulator/server-config","version":1,
                "http":{"token":"http-secret"}}"#,
        )
        .unwrap();
        let p = preview(&current, &f, ImportSource::File);
        assert_eq!(p.device_name, current.effective_device_name());
        assert_eq!(p.http_token, TokenChange::Unchanged);
    }

    #[test]
    fn preview_has_no_host_for_empty_url() {
        let f =
            parse_config_file(r#"{"format":"nfc-keyboard-emulator/server-config","version":1}"#)
                .unwrap();
        let p = preview(&IntegrationConfig::default(), &f, ImportSource::File);
        assert_eq!(p.http_host_before, None);
        assert_eq!(p.http_host_after, None);
        assert_eq!(p.ws_host_before, None);
        assert_eq!(p.ws_host_after, None);
        assert!(!p.http_token_to_new_host && !p.ws_token_to_new_host);
    }

    fn preview_of(current: &IntegrationConfig, body: &str) -> ImportPreview {
        let text =
            format!(r#"{{"format":"nfc-keyboard-emulator/server-config","version":1,{body}}}"#);
        preview(
            current,
            &parse_config_file(&text).unwrap(),
            ImportSource::Link,
        )
    }

    #[test]
    fn preview_without_host_change_has_no_warning() {
        let p = preview_of(
            &sample(),
            r#""http":{"url":"https://example.com/other/path"},"ws":{"stream":false}"#,
        );
        assert_eq!(p.http_host_before, p.http_host_after);
        assert_eq!(p.ws_host_before, p.ws_host_after);
        assert!(!p.http_token_to_new_host && !p.http_downgrade);
        assert!(!p.ws_token_to_new_host && !p.ws_downgrade);
    }

    #[test]
    fn preview_warns_when_kept_token_goes_to_new_host() {
        let p = preview_of(
            &sample(),
            r#""http":{"url":"https://evil.example/hook"},"ws":{"url":"wss://evil.example/ws"}"#,
        );
        assert_eq!(p.http_host_after.as_deref(), Some("https://evil.example"));
        assert_eq!(p.http_token, TokenChange::Unchanged);
        assert!(p.http_token_to_new_host);
        assert!(p.ws_token_to_new_host);
        assert!(!p.http_downgrade && !p.ws_downgrade);

        // Repeating the stored token in the file still sends it to the new host.
        let p = preview_of(
            &sample(),
            r#""http":{"url":"https://evil.example/hook","token":"http-secret"}"#,
        );
        assert!(p.http_token_to_new_host);

        // A port change is a new host as well.
        let p = preview_of(
            &sample(),
            r#""ws":{"url":"wss://example.com:9443/scanner"}"#,
        );
        assert!(p.ws_token_to_new_host);
    }

    #[test]
    fn preview_warns_on_scheme_downgrade() {
        let p = preview_of(
            &sample(),
            r#""http":{"url":"http://example.com/api/scans"},"ws":{"url":"ws://example.com:8443/scanner"}"#,
        );
        assert!(p.http_downgrade && p.ws_downgrade);
        // The scheme is part of the address, so the kept token is flagged too.
        assert!(p.http_token_to_new_host && p.ws_token_to_new_host);

        // An upgrade is not a downgrade, and clearing the URL sends nothing.
        let mut current = sample();
        current.http.url = "http://example.com/api".into();
        let p = preview_of(
            &current,
            r#""http":{"url":"https://example.com/api"},"ws":{"url":""}"#,
        );
        assert!(!p.http_downgrade);
        assert!(!p.ws_downgrade && !p.ws_token_to_new_host);
        assert_eq!(p.ws_host_after, None);
    }

    #[test]
    fn preview_new_host_with_new_token_shows_change_without_warning() {
        let p = preview_of(
            &sample(),
            r#""http":{"url":"https://new.example/hook","token":"fresh"}"#,
        );
        assert_eq!(p.http_host_before.as_deref(), Some("https://example.com"));
        assert_eq!(p.http_host_after.as_deref(), Some("https://new.example"));
        assert_eq!(p.http_token, TokenChange::Set);
        assert!(!p.http_token_to_new_host);

        // Without a stored token there is nothing to redirect.
        let mut current = sample();
        current.http.token = String::new();
        let p = preview_of(&current, r#""http":{"url":"https://new.example/hook"}"#);
        assert!(!p.http_token_to_new_host);

        // A first URL with an already stored token counts as a new host.
        current.http.url = String::new();
        current.http.token = "kept".into();
        let p = preview_of(&current, r#""http":{"url":"https://new.example/hook"}"#);
        assert_eq!(p.http_host_before, None);
        assert!(p.http_token_to_new_host);
    }

    #[test]
    fn import_error_serializes_with_kind_tag() {
        assert_eq!(
            serde_json::to_string(&ImportError::UnsupportedVersion { version: 2 }).unwrap(),
            r#"{"kind":"unsupported_version","version":2}"#
        );
        assert_eq!(
            serde_json::to_string(&ImportError::WrongFormat).unwrap(),
            r#"{"kind":"wrong_format"}"#
        );
    }

    const SCHEMA: &str = include_str!("../../../docs/schema/server-config.v1.schema.json");

    /// The example file from the design spec.
    const DOC_EXAMPLE: &str = r#"{
  "$schema": "https://<docs-host>/nfc-keyboard-emulator/server-config.v1.schema.json",
  "format": "nfc-keyboard-emulator/server-config",
  "version": 1,
  "deviceName": "Reception",
  "http": { "enabled": true, "url": "https://example.com/api/scans", "token": "..." },
  "ws": {
    "url": "wss://example.com/scanner",
    "token": "...",
    "stream": true,
    "request": true
  }
}"#;

    fn keys(v: &serde_json::Value) -> Vec<String> {
        let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
        k.sort();
        k
    }

    #[test]
    fn schema_accepts_doc_examples() {
        let schema: serde_json::Value = serde_json::from_str(SCHEMA).unwrap();
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(schema["$id"], SCHEMA_URL);
        assert_eq!(schema["required"], serde_json::json!(["format", "version"]));
        assert_eq!(schema["properties"]["format"]["const"], FORMAT_ID);
        assert_eq!(schema["properties"]["version"]["const"], FORMAT_VERSION);

        // The schema describes exactly the fields the parser knows.
        let full = serde_json::to_value(export(&sample())).unwrap();
        assert_eq!(keys(&schema["properties"]), keys(&full));
        for section in ["http", "ws"] {
            assert_eq!(
                keys(&schema["properties"][section]["properties"]),
                keys(&full[section]),
                "{section}"
            );
            assert_eq!(schema["properties"][section]["additionalProperties"], true);
        }

        let f = parse_config_file(DOC_EXAMPLE).unwrap();
        assert_eq!(f.device_name.as_deref(), Some("Reception"));
        assert_eq!(
            f.http.unwrap().url.as_deref(),
            Some("https://example.com/api/scans")
        );
        assert_eq!(f.ws.unwrap().request, Some(true));
    }

    /// Documents with the schema's verdict and the parser's. A document the schema accepts
    /// must never be rejected by the parser; the schema may be stricter, never looser.
    #[test]
    fn schema_and_parser_agree() {
        fn doc(body: &str) -> String {
            format!(r#"{{"format":"nfc-keyboard-emulator/server-config","version":1{body}}}"#)
        }
        let url = |section: &str, u: &str| doc(&format!(r#","{section}":{{"url":"{u}"}}"#));

        let corpus: Vec<(&str, String, bool)> = vec![
            ("spec example", DOC_EXAMPLE.to_string(), true),
            ("minimal", doc(""), true),
            ("absent sections", doc(r#","deviceName":"A""#), true),
            ("empty http section", doc(r#","http":{}"#), true),
            (
                "cleared token",
                doc(r#","http":{"token":""},"ws":{"token":""}"#),
                true,
            ),
            (
                "cleared urls",
                doc(r#","http":{"url":""},"ws":{"url":""}"#),
                true,
            ),
            (
                "unknown keys",
                doc(r#","extra":1,"http":{"retries":3},"ws":{"compression":"x"}"#),
                true,
            ),
            ("http url", url("http", "http://example.com/a?b=c#d"), true),
            ("https port", url("http", "https://example.com:8443"), true),
            (
                "http userinfo",
                url("http", "https://u:p@example.com/x"),
                true,
            ),
            ("http ipv6", url("http", "http://[::1]:8080/x"), true),
            ("http max port", url("http", "http://h:65535"), true),
            ("ws url", url("ws", "ws://localhost:9000"), true),
            ("wss url", url("ws", "wss://example.com/scanner"), true),
            ("http empty host", url("http", "http://:80"), false),
            ("http space in host", url("http", "http://a b"), false),
            (
                "http port too large",
                url("http", "http://host:99999"),
                false,
            ),
            ("http unclosed bracket", url("http", "http://[::1"), false),
            ("http no host", url("http", "http://"), false),
            ("http no scheme", url("http", "example.com/api"), false),
            ("http ftp", url("http", "ftp://x"), false),
            ("http ws scheme", url("http", "ws://x.test"), false),
            (
                "http space in path",
                url("http", "http://x.test/a b"),
                false,
            ),
            ("http uppercase scheme", url("http", "HTTP://x.test"), false),
            ("ws no host", url("ws", "ws://"), false),
            ("ws ftp", url("ws", "ftp://x"), false),
            ("ws https scheme", url("ws", "https://x.test"), false),
            ("ws empty host", url("ws", "wss://:80"), false),
            (
                "version 1.0",
                r#"{"format":"nfc-keyboard-emulator/server-config","version":1.0}"#.to_string(),
                true,
            ),
            (
                "version 2",
                r#"{"format":"nfc-keyboard-emulator/server-config","version":2}"#.to_string(),
                false,
            ),
            (
                "version 1.5",
                r#"{"format":"nfc-keyboard-emulator/server-config","version":1.5}"#.to_string(),
                false,
            ),
            (
                "version string",
                r#"{"format":"nfc-keyboard-emulator/server-config","version":"1"}"#.to_string(),
                false,
            ),
            (
                "version missing",
                r#"{"format":"nfc-keyboard-emulator/server-config"}"#.to_string(),
                false,
            ),
            (
                "wrong format",
                r#"{"format":"x","version":1}"#.to_string(),
                false,
            ),
            ("format missing", r#"{"version":1}"#.to_string(), false),
            (
                "enabled not bool",
                doc(r#","http":{"enabled":"yes"}"#),
                false,
            ),
            ("stream not bool", doc(r#","ws":{"stream":1}"#), false),
            ("token not string", doc(r#","ws":{"token":5}"#), false),
        ];

        // The parser is deliberately more lenient here (it percent-encodes and lowercases).
        const SCHEMA_STRICTER: [&str; 2] = ["http space in path", "http uppercase scheme"];

        let schema: serde_json::Value = serde_json::from_str(SCHEMA).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        for (name, text, expect_valid) in corpus {
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            let schema_valid = validator.is_valid(&value);
            let parsed = parse_config_file(&text);
            assert_eq!(
                schema_valid, expect_valid,
                "schema verdict for {name}: {text}"
            );
            if schema_valid {
                assert!(
                    parsed.is_ok(),
                    "schema accepts but parser rejects {name}: {parsed:?}"
                );
            } else if !SCHEMA_STRICTER.contains(&name) {
                assert!(parsed.is_err(), "schema rejects but parser accepts {name}");
            }
        }
    }

    #[test]
    fn version_accepts_integral_float() {
        let f =
            parse_config_file(r#"{"format":"nfc-keyboard-emulator/server-config","version":1.0}"#)
                .unwrap();
        assert_eq!(f.version, 1);
        assert_eq!(
            parse_config_file(r#"{"format":"nfc-keyboard-emulator/server-config","version":2.0}"#),
            Err(ImportError::UnsupportedVersion { version: 2 })
        );
    }
}
