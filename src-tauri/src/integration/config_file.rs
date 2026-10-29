use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use url::Url;

use super::auth::{validate_auth, AuthConfig, AuthError, AuthMode};
use super::config::{normalize_target_auth, valid_token, valid_url, IntegrationConfig};
use super::http::origin_of;
use super::template::{validate_payload, PayloadConfig, PayloadMode, TemplateError};

pub const FORMAT_ID: &str = "nfc-keyboard-emulator/server-config";
pub const FORMAT_VERSION: u32 = 1;
pub const SCHEMA_URL: &str =
    "https://docs.noix.dev/nfc-keyboard-emulator/server-config.v1.schema.json";

const LINK_SCHEME: &str = "nfc-keyboard-emulator";
const LINK_ACTION: &str = "configure";
const LINK_PARAM: &str = "config";

/// The import/export file. Every key except `format` and `version` is optional: an absent
/// key leaves the setting unchanged, a present one (even `""`) replaces it. A present `auth`
/// or `payload` object replaces the whole setting; its missing keys take their defaults.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<PayloadConfig>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<PayloadConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ImportError {
    InvalidJson { message: String },
    WrongFormat,
    UnsupportedVersion { version: u32 },
    InvalidHttpUrl,
    InvalidWsUrl,
    InvalidHttpToken,
    InvalidWsToken,
    InvalidHttpHeaderName,
    InvalidWsHeaderName,
    InvalidHttpUsername,
    InvalidWsUsername,
    InvalidHttpQueryParam,
    InvalidWsQueryParam,
    InvalidHttpTemplate { error: TemplateError },
    InvalidWsTemplate { error: TemplateError },
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
        if let Some(auth) = &mut http.auth {
            normalize_target_auth(auth);
        }
    }
    if let Some(ws) = &mut file.ws {
        for s in [&mut ws.url, &mut ws.token].into_iter().flatten() {
            trim_in_place(s);
        }
        if let Some(auth) = &mut ws.auth {
            normalize_target_auth(auth);
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
    let http_token = file.http.as_ref().and_then(|h| h.token.as_deref());
    if !valid_token(http_token.unwrap_or("")) {
        return Err(ImportError::InvalidHttpToken);
    }
    let ws_token = file.ws.as_ref().and_then(|w| w.token.as_deref());
    if !valid_token(ws_token.unwrap_or("")) {
        return Err(ImportError::InvalidWsToken);
    }
    if let Some(auth) = file.http.as_ref().and_then(|h| h.auth.as_ref()) {
        validate_auth(auth).map_err(|e| match e {
            AuthError::HeaderName => ImportError::InvalidHttpHeaderName,
            AuthError::Username => ImportError::InvalidHttpUsername,
            AuthError::QueryParam => ImportError::InvalidHttpQueryParam,
        })?;
    }
    if let Some(auth) = file.ws.as_ref().and_then(|w| w.auth.as_ref()) {
        validate_auth(auth).map_err(|e| match e {
            AuthError::HeaderName => ImportError::InvalidWsHeaderName,
            AuthError::Username => ImportError::InvalidWsUsername,
            AuthError::QueryParam => ImportError::InvalidWsQueryParam,
        })?;
    }
    if let Some(payload) = file.http.as_ref().and_then(|h| h.payload.as_ref()) {
        validate_payload(payload).map_err(|error| ImportError::InvalidHttpTemplate { error })?;
    }
    if let Some(payload) = file.ws.as_ref().and_then(|w| w.payload.as_ref()) {
        validate_payload(payload).map_err(|error| ImportError::InvalidWsTemplate { error })?;
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
        if let Some(v) = &http.auth {
            out.http.auth = v.clone();
        }
        if let Some(v) = &http.payload {
            out.http.payload = v.clone();
        }
    }
    if let Some(ws) = &f.ws {
        if let Some(v) = &ws.url {
            out.ws.url = v.clone();
        }
        if let Some(v) = &ws.token {
            out.ws.token = v.clone();
        }
        if let Some(v) = &ws.auth {
            out.ws.auth = v.clone();
        }
        if let Some(v) = &ws.payload {
            out.ws.payload = v.clone();
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

/// The current settings as a config file, tokens included. A full snapshot: every key is
/// written, defaults too, so importing it onto another device restores exactly these settings.
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
            auth: Some(cfg.http.auth.clone()),
            payload: Some(cfg.http.payload.clone()),
        }),
        ws: Some(WsPatch {
            url: Some(cfg.ws.url.clone()),
            token: Some(cfg.ws.token.clone()),
            stream: Some(cfg.ws.stream),
            request: Some(cfg.ws.request),
            auth: Some(cfg.ws.auth.clone()),
            payload: Some(cfg.ws.payload.clone()),
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
    pub http_auth: AuthMode,
    /// The whole HTTP auth setting differs from the current one.
    pub http_auth_changed: bool,
    pub http_payload: PayloadMode,
    /// The whole HTTP payload setting differs from the current one.
    pub http_payload_changed: bool,
    pub ws_host_before: Option<String>,
    pub ws_host_after: Option<String>,
    pub ws_stream: bool,
    pub ws_request: bool,
    pub ws_token: TokenChange,
    /// The stored, non-empty WebSocket token is kept but goes to another scheme or host.
    pub ws_token_to_new_host: bool,
    /// `wss` becomes `ws`.
    pub ws_downgrade: bool,
    pub ws_auth: AuthMode,
    /// The whole WebSocket auth setting differs from the current one.
    pub ws_auth_changed: bool,
    pub ws_payload: PayloadMode,
    /// The whole WebSocket payload setting differs from the current one.
    pub ws_payload_changed: bool,
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
        (current.http.auth.mode, after.http.auth.mode),
        "https",
    );
    let ws = HostChange::new(
        (&current.ws.url, &current.ws.token),
        (&after.ws.url, &after.ws.token),
        (current.ws.auth.mode, after.ws.auth.mode),
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
        http_auth: after.http.auth.mode,
        http_auth_changed: after.http.auth != current.http.auth,
        http_payload: after.http.payload.mode,
        http_payload_changed: after.http.payload != current.http.payload,
        ws_host_before: ws.before,
        ws_host_after: ws.after,
        ws_stream: after.ws.stream,
        ws_request: after.ws.request,
        ws_token: token_change(&current.ws.token, ws_token),
        ws_token_to_new_host: ws.token_to_new_host,
        ws_downgrade: ws.downgrade,
        ws_auth: after.ws.auth.mode,
        ws_auth_changed: after.ws.auth != current.ws.auth,
        ws_payload: after.ws.payload.mode,
        ws_payload_changed: after.ws.payload != current.ws.payload,
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
    /// `before` and `after` are `(url, token)`; `modes` are the auth modes before and after;
    /// `secure` is the scheme that encrypts. A kept token that moves into the URL query also
    /// counts as going somewhere new: URLs end up in logs and histories.
    fn new(
        before: (&str, &str),
        after: (&str, &str),
        modes: (AuthMode, AuthMode),
        secure: &str,
    ) -> Self {
        let old = Url::parse(before.0).ok().filter(Url::has_host);
        let new = Url::parse(after.0).ok().filter(Url::has_host);
        let old_origin = old.as_ref().map(origin_of);
        let new_origin = new.as_ref().map(origin_of);
        let token_kept = !before.1.is_empty() && after.1 == before.1;
        Self {
            token_to_new_host: token_kept
                && new_origin.is_some()
                && (new_origin != old_origin
                    || (modes.0 != AuthMode::Query && modes.1 == AuthMode::Query)),
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
                ..Default::default()
            },
            ws: WsTarget {
                url: "wss://example.com:8443/scanner".into(),
                token: "ws-secret".into(),
                stream: true,
                request: false,
                ..Default::default()
            },
        }
    }

    fn json(f: &ServerConfigFile) -> String {
        serde_json::to_string(f).unwrap()
    }

    /// `sample()` with non-default auth and payload on both targets.
    fn custom() -> IntegrationConfig {
        let mut cfg = sample();
        cfg.http.auth = AuthConfig {
            mode: AuthMode::Basic,
            username: "reader".into(),
            ..Default::default()
        };
        cfg.http.payload = PayloadConfig {
            mode: PayloadMode::Template,
            template: r#"{"tag":"{{uid}}"}"#.into(),
        };
        cfg.ws.auth = AuthConfig {
            mode: AuthMode::Header,
            header_name: "X-Station-Key".into(),
            ..Default::default()
        };
        cfg.ws.payload = PayloadConfig {
            mode: PayloadMode::Template,
            template: r#"{"n":"{{uidLength}}"}"#.into(),
        };
        cfg
    }

    fn parse_body(body: &str) -> Result<ServerConfigFile, ImportError> {
        parse_config_file(&format!(
            r#"{{"format":"nfc-keyboard-emulator/server-config","version":1,{body}}}"#
        ))
    }

    #[test]
    fn old_file_without_auth_imports_unchanged() {
        let cfg = custom();
        let f =
            parse_body(r#""http":{"url":"https://new.example.com/hook"},"ws":{"stream":false}"#)
                .unwrap();
        assert_eq!(f.http.as_ref().unwrap().auth, None);
        assert_eq!(f.http.as_ref().unwrap().payload, None);
        assert_eq!(f.ws.as_ref().unwrap().auth, None);
        assert_eq!(f.ws.as_ref().unwrap().payload, None);
        let out = apply(&cfg, &f);
        assert_eq!(out.http.auth, cfg.http.auth);
        assert_eq!(out.http.payload, cfg.http.payload);
        assert_eq!(out.ws.auth, cfg.ws.auth);
        assert_eq!(out.ws.payload, cfg.ws.payload);
    }

    #[test]
    fn auth_object_replaces_whole_setting() {
        let mut cfg = custom();
        cfg.http.auth.header_name = "X-Old".into();
        cfg.http.auth.query_param = "old".into();
        cfg.ws.payload.template = r#"{"old":true}"#.into();
        let f = parse_body(
            r#""http":{"auth":{"mode":"query"},"payload":{"mode":"standard"}},
               "ws":{"auth":{"mode":"none"},"payload":{"template":"{}"}}"#,
        )
        .unwrap();
        let out = apply(&cfg, &f);
        // Missing sub-fields take their defaults instead of keeping the current values.
        assert_eq!(
            out.http.auth,
            AuthConfig {
                mode: AuthMode::Query,
                ..Default::default()
            }
        );
        assert_eq!(out.http.payload, PayloadConfig::default());
        assert_eq!(
            out.ws.auth,
            AuthConfig {
                mode: AuthMode::None,
                ..Default::default()
            }
        );
        assert_eq!(
            out.ws.payload,
            PayloadConfig {
                mode: PayloadMode::Standard,
                template: "{}".into(),
            }
        );
    }

    #[test]
    fn import_trims_auth_names_but_not_templates() {
        let f = parse_body(
            r#""http":{"auth":{"mode":"header","headerName":" X-Key\n","username":" u ","queryParam":" q "},
                       "payload":{"mode":"template","template":" {\"a\":1}\n"}}"#,
        )
        .unwrap();
        let http = f.http.unwrap();
        let auth = http.auth.unwrap();
        assert_eq!(auth.header_name, "X-Key");
        assert_eq!(auth.username, "u");
        assert_eq!(auth.query_param, "q");
        assert_eq!(http.payload.unwrap().template, " {\"a\":1}\n");
    }

    #[test]
    fn import_rejects_bad_header_name() {
        let err =
            parse_body(r#""http":{"auth":{"mode":"header","headerName":"bad name"}}"#).unwrap_err();
        assert_eq!(err, ImportError::InvalidHttpHeaderName);
        assert!(!serde_json::to_string(&err).unwrap().contains("bad name"));
        assert_eq!(
            parse_body(r#""ws":{"auth":{"mode":"header","headerName":"Host"}}"#),
            Err(ImportError::InvalidWsHeaderName)
        );
        assert_eq!(
            parse_body(r#""http":{"auth":{"mode":"header","headerName":""}}"#),
            Err(ImportError::InvalidHttpHeaderName)
        );
        assert_eq!(
            parse_body(r#""http":{"auth":{"mode":"basic","username":"a:b"}}"#),
            Err(ImportError::InvalidHttpUsername)
        );
        assert_eq!(
            parse_body(r#""ws":{"auth":{"mode":"basic","username":"a b"}}"#),
            Err(ImportError::InvalidWsUsername)
        );
        assert_eq!(
            parse_body(r#""http":{"auth":{"mode":"query","queryParam":"a b"}}"#),
            Err(ImportError::InvalidHttpQueryParam)
        );
        assert_eq!(
            parse_body(r#""ws":{"auth":{"mode":"query","queryParam":""}}"#),
            Err(ImportError::InvalidWsQueryParam)
        );
        // Only the field of the active mode is checked.
        assert!(parse_body(
            r#""http":{"auth":{"mode":"bearer","headerName":"bad name","username":"a:b","queryParam":""}}"#
        )
        .is_ok());
        assert!(parse_body(r#""ws":{"auth":{"mode":"basic","username":""}}"#).is_ok());
        // A mode the app does not know is a type error, like any other wrong value.
        assert!(matches!(
            parse_body(r#""http":{"auth":{"mode":"digest"}}"#),
            Err(ImportError::InvalidJson { .. })
        ));
    }

    #[test]
    fn import_rejects_bad_template() {
        assert_eq!(
            parse_body(r#""http":{"payload":{"mode":"template","template":"not json"}}"#),
            Err(ImportError::InvalidHttpTemplate {
                error: TemplateError::NotJson
            })
        );
        assert_eq!(
            parse_body(r#""ws":{"payload":{"mode":"template","template":"{\"a\":\"{{nope}}\"}"}}"#),
            Err(ImportError::InvalidWsTemplate {
                error: TemplateError::UnknownPlaceholder {
                    name: "nope".into()
                }
            })
        );
        let big = format!(r#"{{"a":"{}"}}"#, "x".repeat(8192));
        assert_eq!(
            parse_body(&format!(
                r#""http":{{"payload":{{"mode":"template","template":{}}}}}"#,
                serde_json::to_string(&big).unwrap()
            )),
            Err(ImportError::InvalidHttpTemplate {
                error: TemplateError::TooLarge
            })
        );
        // The template of a standard-mode payload is kept, not validated.
        assert!(
            parse_body(r#""http":{"payload":{"mode":"standard","template":"not json"}}"#).is_ok()
        );
        // The error says what is wrong, never the template text.
        let err = parse_body(r#""http":{"payload":{"mode":"template","template":"secret-ish"}}"#)
            .unwrap_err();
        assert!(!serde_json::to_string(&err).unwrap().contains("secret-ish"));
    }

    #[test]
    fn url_and_token_errors_come_before_auth_and_template_errors() {
        assert_eq!(
            parse_body(
                r#""http":{"auth":{"mode":"header","headerName":"bad name"}},"ws":{"url":"https://x.test"}"#
            ),
            Err(ImportError::InvalidWsUrl)
        );
        assert_eq!(
            parse_body(
                r#""http":{"payload":{"mode":"template","template":"x"}},
                   "ws":{"auth":{"mode":"header","headerName":"bad name"}}"#
            ),
            Err(ImportError::InvalidWsHeaderName)
        );
    }

    #[test]
    fn export_writes_default_auth_and_payload_explicitly() {
        let file = export(&sample());
        for (auth, payload) in [
            (
                &file.http.as_ref().unwrap().auth,
                &file.http.as_ref().unwrap().payload,
            ),
            (
                &file.ws.as_ref().unwrap().auth,
                &file.ws.as_ref().unwrap().payload,
            ),
        ] {
            assert_eq!(auth.as_ref(), Some(&AuthConfig::default()));
            assert_eq!(payload.as_ref(), Some(&PayloadConfig::default()));
        }
        let text = json(&file);
        assert_eq!(text.matches(r#""auth":{"mode":"bearer""#).count(), 2);
        assert_eq!(text.matches(r#""payload":{"mode":"standard""#).count(), 2);
    }

    #[test]
    fn importing_a_default_export_resets_custom_auth() {
        let current = custom();
        let f = parse_config_file(&json(&export(&sample()))).unwrap();
        let out = apply(&current, &f);
        assert_eq!(out, sample());
        assert_eq!(out.http.auth, AuthConfig::default());
        assert_eq!(out.ws.payload, PayloadConfig::default());

        let p = preview(&current, &f, ImportSource::File);
        assert!(p.http_auth_changed && p.ws_auth_changed);
        assert!(p.http_payload_changed && p.ws_payload_changed);
        assert_eq!(p.http_auth, AuthMode::Bearer);
        assert_eq!(p.ws_payload, PayloadMode::Standard);
    }

    #[test]
    fn export_includes_custom_auth_and_payload() {
        let cfg = custom();
        let file = export(&cfg);
        let text = json(&file);
        assert!(text.contains(r#""auth":{"mode":"basic""#));
        assert!(text.contains(r#""payload":{"mode":"template""#));
        let parsed = parse_config_file(&json(&file)).unwrap();
        assert_eq!(parsed, file);
        assert_eq!(apply(&IntegrationConfig::default(), &parsed), cfg);
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
    fn rejects_tokens_with_invalid_characters() {
        let with = |body: &str| {
            parse_config_file(&format!(
                r#"{{"format":"nfc-keyboard-emulator/server-config","version":1,{body}}}"#
            ))
        };
        assert_eq!(
            with(r#""http":{"token":"two words"}"#),
            Err(ImportError::InvalidHttpToken)
        );
        assert_eq!(
            with(r#""ws":{"token":"a\u0001b"}"#),
            Err(ImportError::InvalidWsToken)
        );
        assert_eq!(
            with(r#""ws":{"token":"tök"}"#),
            Err(ImportError::InvalidWsToken)
        );
        let err = with(r#""http":{"token":"secret\nvalue"}"#).unwrap_err();
        assert_eq!(err, ImportError::InvalidHttpToken);
        assert!(!serde_json::to_string(&err).unwrap().contains("secret"));
        assert!(with(r#""http":{"token":" ok-token \n"},"ws":{"token":""}"#).is_ok());
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
    fn preview_reports_auth_and_payload_changes() {
        let current = sample();
        let p = preview_of(
            &current,
            r#""http":{"auth":{"mode":"basic","username":"reader"}},
               "ws":{"payload":{"mode":"template","template":"{}"}}"#,
        );
        assert_eq!(p.http_auth, AuthMode::Basic);
        assert!(p.http_auth_changed);
        assert_eq!(p.http_payload, PayloadMode::Standard);
        assert!(!p.http_payload_changed);
        assert_eq!(p.ws_auth, AuthMode::Bearer);
        assert!(!p.ws_auth_changed);
        assert_eq!(p.ws_payload, PayloadMode::Template);
        assert!(p.ws_payload_changed);

        let out = serde_json::to_string(&p).unwrap();
        assert!(out.contains(r#""httpAuth":"basic""#));
        assert!(out.contains(r#""httpAuthChanged":true"#));
        assert!(out.contains(r#""wsPayload":"template""#));
        assert!(out.contains(r#""wsPayloadChanged":true"#));
        assert!(!out.contains("reader"));

        // The whole setting is compared, not just the mode.
        let current = custom();
        let p = preview_of(
            &current,
            r#""http":{"auth":{"mode":"basic","username":"other"},
                       "payload":{"mode":"template","template":"{\"tag\":\"{{uid}}\"}"}}"#,
        );
        assert_eq!(p.http_auth, AuthMode::Basic);
        assert!(p.http_auth_changed);
        assert!(!p.http_payload_changed);
        let p = preview_of(
            &current,
            r#""http":{"payload":{"mode":"template","template":"{}"}}"#,
        );
        assert!(p.http_payload_changed);

        // Repeating the current setting, or sending nothing, changes nothing.
        let p = preview_of(
            &current,
            r#""http":{"auth":{"mode":"basic","username":"reader"}}"#,
        );
        assert!(!p.http_auth_changed);
        let p = preview_of(&current, r#""deviceName":"x""#);
        assert!(!p.http_auth_changed && !p.ws_auth_changed);
        assert!(!p.http_payload_changed && !p.ws_payload_changed);
        assert_eq!(p.ws_auth, AuthMode::Header);
    }

    #[test]
    fn switch_to_query_with_kept_token_warns() {
        let p = preview_of(
            &sample(),
            r#""http":{"auth":{"mode":"query"}},"ws":{"auth":{"mode":"query"}}"#,
        );
        assert_eq!(p.http_host_before, p.http_host_after);
        assert_eq!(p.http_token, TokenChange::Unchanged);
        assert!(p.http_token_to_new_host);
        assert!(p.ws_token_to_new_host);

        // Repeating the stored token in the file is still a kept token.
        let p = preview_of(
            &sample(),
            r#""http":{"token":"http-secret","auth":{"mode":"query"}}"#,
        );
        assert!(p.http_token_to_new_host);

        // From any other mode, not only from bearer.
        let mut current = sample();
        current.ws.auth.mode = AuthMode::None;
        let p = preview_of(&current, r#""ws":{"auth":{"mode":"query"}}"#);
        assert!(p.ws_token_to_new_host);

        // No warning when the mode stays, the token is replaced or cleared, there is no
        // token, or the mode is not query.
        let mut queried = sample();
        queried.http.auth.mode = AuthMode::Query;
        let p = preview_of(&queried, r#""http":{"auth":{"mode":"query"}}"#);
        assert!(!p.http_token_to_new_host);
        let p = preview_of(
            &sample(),
            r#""http":{"token":"fresh","auth":{"mode":"query"}}"#,
        );
        assert!(!p.http_token_to_new_host);
        let p = preview_of(&sample(), r#""http":{"token":"","auth":{"mode":"query"}}"#);
        assert!(!p.http_token_to_new_host);
        let mut no_token = sample();
        no_token.http.token = String::new();
        let p = preview_of(&no_token, r#""http":{"auth":{"mode":"query"}}"#);
        assert!(!p.http_token_to_new_host);
        let p = preview_of(&sample(), r#""http":{"auth":{"mode":"header"}}"#);
        assert!(!p.http_token_to_new_host);
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
        let full = serde_json::to_value(export(&custom())).unwrap();
        assert_eq!(keys(&schema["properties"]), keys(&full));
        for section in ["http", "ws"] {
            assert_eq!(
                keys(&schema["properties"][section]["properties"]),
                keys(&full[section]),
                "{section}"
            );
            assert_eq!(schema["properties"][section]["additionalProperties"], true);
        }

        // The sub-objects list the keys the parser knows, with the enums the code serializes.
        let auth_keys = keys(&serde_json::to_value(AuthConfig::default()).unwrap());
        let payload_keys = keys(&serde_json::to_value(PayloadConfig::default()).unwrap());
        // Adding a variant breaks these matches, as a reminder to extend the schema.
        let auth_modes = [
            AuthMode::Bearer,
            AuthMode::Header,
            AuthMode::Basic,
            AuthMode::Query,
            AuthMode::None,
        ];
        let payload_modes = [PayloadMode::Standard, PayloadMode::Template];
        let _ = |m: AuthMode| match m {
            AuthMode::Bearer
            | AuthMode::Header
            | AuthMode::Basic
            | AuthMode::Query
            | AuthMode::None => (),
        };
        let _ = |m: PayloadMode| match m {
            PayloadMode::Standard | PayloadMode::Template => (),
        };
        let names = |v: Vec<serde_json::Value>| -> Vec<String> {
            let mut n: Vec<String> = v.iter().map(|x| x.as_str().unwrap().to_string()).collect();
            n.sort();
            n
        };
        for section in ["http", "ws"] {
            let target = &schema["properties"][section]["properties"];
            assert_eq!(
                keys(&target["auth"]["properties"]),
                auth_keys,
                "{section} auth"
            );
            assert_eq!(
                keys(&target["payload"]["properties"]),
                payload_keys,
                "{section} payload"
            );
            assert_eq!(
                names(
                    target["auth"]["properties"]["mode"]["enum"]
                        .as_array()
                        .unwrap()
                        .clone()
                ),
                names(
                    auth_modes
                        .iter()
                        .map(|m| serde_json::to_value(m).unwrap())
                        .collect()
                ),
                "{section} auth.mode"
            );
            assert_eq!(
                names(
                    target["payload"]["properties"]["mode"]["enum"]
                        .as_array()
                        .unwrap()
                        .clone()
                ),
                names(
                    payload_modes
                        .iter()
                        .map(|m| serde_json::to_value(m).unwrap())
                        .collect()
                ),
                "{section} payload.mode"
            );
            // The documented defaults are the real ones.
            assert_eq!(
                target["auth"]["properties"]["mode"]["default"],
                serde_json::to_value(AuthConfig::default().mode).unwrap()
            );
            assert_eq!(
                target["auth"]["properties"]["headerName"]["default"],
                AuthConfig::default().header_name
            );
            assert_eq!(
                target["auth"]["properties"]["queryParam"]["default"],
                AuthConfig::default().query_param
            );
            assert_eq!(
                target["payload"]["properties"]["mode"]["default"],
                serde_json::to_value(PayloadConfig::default().mode).unwrap()
            );
            assert_eq!(target["auth"]["additionalProperties"], true);
            assert_eq!(target["payload"]["additionalProperties"], true);
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
            (
                "visible ascii token",
                doc(r#","http":{"token":"eyJ0.e30-_~+/=!*"},"ws":{"token":"abc"}"#),
                true,
            ),
            (
                "auth and payload",
                doc(r#","http":{"auth":{"mode":"header","headerName":"X-Key"},
                       "payload":{"mode":"template","template":"{\"a\":\"{{uid}}\"}"}},
                       "ws":{"auth":{"mode":"basic","username":"reader"},"payload":{"mode":"standard"}}"#),
                true,
            ),
            (
                "auth with inactive fields",
                doc(
                    r#","http":{"auth":{"mode":"bearer","headerName":"bad name","username":"a:b","queryParam":""}}"#,
                ),
                true,
            ),
            (
                "auth empty object",
                doc(r#","ws":{"auth":{},"payload":{}}"#),
                true,
            ),
            (
                "auth none",
                doc(r#","http":{"auth":{"mode":"none"}}"#),
                true,
            ),
            (
                "auth query",
                doc(r#","ws":{"auth":{"mode":"query","queryParam":"api_key"}}"#),
                true,
            ),
            (
                "auth basic empty user",
                doc(r#","http":{"auth":{"mode":"basic","username":""}}"#),
                true,
            ),
            (
                "auth header token chars",
                doc(r#","ws":{"auth":{"mode":"header","headerName":"X-A.b_c~1"}}"#),
                true,
            ),
            (
                "auth unknown mode",
                doc(r#","http":{"auth":{"mode":"digest"}}"#),
                false,
            ),
            ("auth not object", doc(r#","http":{"auth":"basic"}"#), false),
            (
                "auth mode wrong case",
                doc(r#","ws":{"auth":{"mode":"Bearer"}}"#),
                false,
            ),
            (
                "header name with space",
                doc(r#","http":{"auth":{"mode":"header","headerName":"bad name"}}"#),
                false,
            ),
            (
                "header name empty",
                doc(r#","ws":{"auth":{"mode":"header","headerName":""}}"#),
                false,
            ),
            (
                "header name colon",
                doc(r#","http":{"auth":{"mode":"header","headerName":"X:Key"}}"#),
                false,
            ),
            (
                "header name reserved",
                doc(r#","http":{"auth":{"mode":"header","headerName":"Host"}}"#),
                false,
            ),
            (
                "header name reserved mixed case",
                doc(r#","ws":{"auth":{"mode":"header","headerName":"cOnTeNt-TyPe"}}"#),
                false,
            ),
            (
                "header name websocket prefix",
                doc(r#","ws":{"auth":{"mode":"header","headerName":"Sec-WebSocket-Key"}}"#),
                false,
            ),
            (
                "header name reserved only as whole",
                doc(r#","http":{"auth":{"mode":"header","headerName":"Host-Id"}}"#),
                true,
            ),
            (
                "username with colon",
                doc(r#","http":{"auth":{"mode":"basic","username":"a:b"}}"#),
                false,
            ),
            (
                "username with space",
                doc(r#","ws":{"auth":{"mode":"basic","username":"a b"}}"#),
                false,
            ),
            (
                "username non-ascii",
                doc(r#","ws":{"auth":{"mode":"basic","username":"jörg"}}"#),
                false,
            ),
            (
                "query param empty",
                doc(r#","http":{"auth":{"mode":"query","queryParam":""}}"#),
                false,
            ),
            (
                "query param with equals",
                doc(r#","ws":{"auth":{"mode":"query","queryParam":"a=b"}}"#),
                false,
            ),
            (
                "payload unknown mode",
                doc(r#","http":{"payload":{"mode":"xml"}}"#),
                false,
            ),
            (
                "template not string",
                doc(r#","http":{"payload":{"mode":"template","template":{"a":1}}}"#),
                false,
            ),
            ("token with space", doc(r#","http":{"token":"a b"}"#), false),
            ("token with tab", doc(r#","ws":{"token":"a\tb"}"#), false),
            (
                "token with control char",
                doc(r#","ws":{"token":"a\u0001b"}"#),
                false,
            ),
            (
                "token with del",
                doc(r#","http":{"token":"a\u007fb"}"#),
                false,
            ),
            ("token non-ascii", doc(r#","ws":{"token":"tök"}"#), false),
            (
                "token with surrounding whitespace",
                doc(r#","http":{"token":" abc "}"#),
                false,
            ),
        ];

        // The parser is deliberately more lenient here (it percent-encodes and lowercases
        // URLs, and trims tokens).
        // The schema cannot say that a template is valid JSON; the parser checks that.
        const SCHEMA_STRICTER: [&str; 3] = [
            "http space in path",
            "http uppercase scheme",
            "token with surrounding whitespace",
        ];

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
