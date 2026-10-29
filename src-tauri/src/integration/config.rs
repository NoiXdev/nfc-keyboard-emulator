use serde::{Deserialize, Serialize};
use url::Url;

use super::auth::{validate_auth, AuthConfig, AuthError};
use super::template::{validate_payload, PayloadConfig, TemplateError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct HttpTarget {
    pub enabled: bool,
    pub url: String,
    pub token: String,
    pub auth: AuthConfig,
    pub payload: PayloadConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WsTarget {
    pub url: String,
    pub token: String,
    pub stream: bool,
    pub request: bool,
    pub auth: AuthConfig,
    pub payload: PayloadConfig,
}

impl WsTarget {
    /// The WebSocket connection is only needed when a mode is on and a URL is set.
    pub fn active(&self) -> bool {
        (self.stream || self.request) && !self.url.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct IntegrationConfig {
    pub device_name: String,
    pub http: HttpTarget,
    pub ws: WsTarget,
}

impl IntegrationConfig {
    /// The configured device name, or the OS host name when it is empty.
    pub fn effective_device_name(&self) -> String {
        if !self.device_name.is_empty() {
            return self.device_name.clone();
        }
        let host = gethostname::gethostname()
            .to_string_lossy()
            .trim()
            .to_string();
        if host.is_empty() {
            "unknown".to_string()
        } else {
            host
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
// The names are the `kind` tags the frontend translates.
#[allow(clippy::enum_variant_names)]
pub enum ConfigError {
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
}

/// Trims the auth names (header name, username, parameter name).
pub(crate) fn normalize_target_auth(auth: &mut AuthConfig) {
    for s in [
        &mut auth.header_name,
        &mut auth.username,
        &mut auth.query_param,
    ] {
        *s = s.trim().to_string();
    }
}

/// Trims all strings (except templates) and validates the URLs, tokens, auth settings and
/// templates. An empty URL is valid (target not configured).
pub fn normalize(mut cfg: IntegrationConfig) -> Result<IntegrationConfig, ConfigError> {
    for s in [
        &mut cfg.device_name,
        &mut cfg.http.url,
        &mut cfg.http.token,
        &mut cfg.ws.url,
        &mut cfg.ws.token,
    ] {
        *s = s.trim().to_string();
    }
    normalize_target_auth(&mut cfg.http.auth);
    normalize_target_auth(&mut cfg.ws.auth);
    if !valid_url(&cfg.http.url, &["http", "https"]) {
        return Err(ConfigError::InvalidHttpUrl);
    }
    if !valid_url(&cfg.ws.url, &["ws", "wss"]) {
        return Err(ConfigError::InvalidWsUrl);
    }
    if !valid_token(&cfg.http.token) {
        return Err(ConfigError::InvalidHttpToken);
    }
    if !valid_token(&cfg.ws.token) {
        return Err(ConfigError::InvalidWsToken);
    }
    validate_auth(&cfg.http.auth).map_err(|e| match e {
        AuthError::HeaderName => ConfigError::InvalidHttpHeaderName,
        AuthError::Username => ConfigError::InvalidHttpUsername,
        AuthError::QueryParam => ConfigError::InvalidHttpQueryParam,
    })?;
    validate_auth(&cfg.ws.auth).map_err(|e| match e {
        AuthError::HeaderName => ConfigError::InvalidWsHeaderName,
        AuthError::Username => ConfigError::InvalidWsUsername,
        AuthError::QueryParam => ConfigError::InvalidWsQueryParam,
    })?;
    validate_payload(&cfg.http.payload)
        .map_err(|error| ConfigError::InvalidHttpTemplate { error })?;
    validate_payload(&cfg.ws.payload).map_err(|error| ConfigError::InvalidWsTemplate { error })?;
    Ok(cfg)
}

/// A token goes into the `Authorization` header, so it may only hold visible ASCII
/// (no spaces, control characters or non-ASCII). Empty means no token.
pub(crate) fn valid_token(value: &str) -> bool {
    value.bytes().all(|b| b.is_ascii_graphic())
}

pub(crate) fn valid_url(value: &str, schemes: &[&str]) -> bool {
    if value.is_empty() {
        return true;
    }
    Url::parse(value).is_ok_and(|u| schemes.contains(&u.scheme()) && u.has_host())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::integration::auth::AuthMode;
    use crate::integration::template::PayloadMode;

    #[test]
    fn defaults_are_all_off() {
        let c = IntegrationConfig::default();
        assert!(!c.http.enabled);
        assert!(!c.ws.stream);
        assert!(!c.ws.request);
        assert!(c.http.url.is_empty() && c.http.token.is_empty());
        assert!(c.ws.url.is_empty() && c.ws.token.is_empty());
    }

    #[test]
    fn old_settings_file_loads_with_integration_off() {
        let c: Config =
            serde_json::from_str(r#"{"typingEnabledOnStart":true,"logRetention":50}"#).unwrap();
        assert_eq!(c.integration, IntegrationConfig::default());
    }

    #[test]
    fn integration_roundtrips_camelcase() {
        let json = serde_json::to_string(&Config::default()).unwrap();
        assert!(json.contains(r#""integration":{"deviceName""#));
        assert!(json.contains(
            r#""ws":{"url":"","token":"","stream":false,"request":false,"auth":{"mode":"bearer","headerName":"X-API-Key","username":"","queryParam":"token"},"payload":{"mode":"standard","template":""}}"#
        ));
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Config::default());
    }

    #[test]
    fn normalize_trims_whitespace() {
        let c = IntegrationConfig {
            device_name: "  Reception ".into(),
            http: HttpTarget {
                url: "  https://x.test/api \n".into(),
                token: " abc ".into(),
                ..Default::default()
            },
            ws: WsTarget {
                url: " wss://x.test/ws ".into(),
                token: "\tdef\n".into(),
                ..Default::default()
            },
        };
        let n = normalize(c).unwrap();
        assert_eq!(n.device_name, "Reception");
        assert_eq!(n.http.url, "https://x.test/api");
        assert_eq!(n.http.token, "abc");
        assert_eq!(n.ws.url, "wss://x.test/ws");
        assert_eq!(n.ws.token, "def");
    }

    fn with_urls(http: &str, ws: &str) -> IntegrationConfig {
        IntegrationConfig {
            http: HttpTarget {
                url: http.into(),
                ..Default::default()
            },
            ws: WsTarget {
                url: ws.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn normalize_rejects_wrong_schemes() {
        assert_eq!(
            normalize(with_urls("example.com/api", "")),
            Err(ConfigError::InvalidHttpUrl)
        );
        assert_eq!(
            normalize(with_urls("ws://x.test", "")),
            Err(ConfigError::InvalidHttpUrl)
        );
        assert_eq!(
            normalize(with_urls("", "https://x.test")),
            Err(ConfigError::InvalidWsUrl)
        );
        assert!(normalize(with_urls("", "")).is_ok());
        assert!(normalize(with_urls("http://x.test", "ws://x.test")).is_ok());
    }

    #[test]
    fn normalize_rejects_unparseable_urls() {
        assert_eq!(
            normalize(with_urls("http://", "")),
            Err(ConfigError::InvalidHttpUrl)
        );
        assert_eq!(
            normalize(with_urls("", "wss://")),
            Err(ConfigError::InvalidWsUrl)
        );
    }

    #[test]
    fn normalize_rejects_tokens_that_cannot_be_sent_as_a_header() {
        let with_tokens = |http: &str, ws: &str| IntegrationConfig {
            http: HttpTarget {
                token: http.into(),
                ..Default::default()
            },
            ws: WsTarget {
                token: ws.into(),
                ..Default::default()
            },
            ..Default::default()
        };
        for bad in [
            "two words",
            "line\nbreak",
            "tab\there",
            "nul\0",
            "del\x7f",
            "tök",
        ] {
            assert_eq!(
                normalize(with_tokens(bad, "")),
                Err(ConfigError::InvalidHttpToken),
                "{bad:?}"
            );
            assert_eq!(
                normalize(with_tokens("", bad)),
                Err(ConfigError::InvalidWsToken),
                "{bad:?}"
            );
        }
        // Visible ASCII, including base64 and JWT characters, is fine; so is no token.
        let ok = "eyJhbGciOi.J9-_~+/=!#$%&'*.^`|";
        assert!(normalize(with_tokens(ok, ok)).is_ok());
        assert!(normalize(with_tokens("", "")).is_ok());
        // Surrounding whitespace is trimmed first.
        assert_eq!(
            normalize(with_tokens(" abc\n", "")).unwrap().http.token,
            "abc"
        );
    }

    #[test]
    fn ws_active_needs_mode_and_url() {
        let ws = |url: &str, stream, request| WsTarget {
            url: url.into(),
            stream,
            request,
            ..Default::default()
        };
        assert!(!ws("", true, false).active());
        assert!(ws("wss://x.test", true, false).active());
        assert!(ws("wss://x.test", false, true).active());
        assert!(!ws("wss://x.test", false, false).active());
    }

    #[test]
    fn effective_device_name_falls_back_to_hostname() {
        assert!(!IntegrationConfig::default()
            .effective_device_name()
            .is_empty());
        let c = IntegrationConfig {
            device_name: "Reception".into(),
            ..Default::default()
        };
        assert_eq!(c.effective_device_name(), "Reception");
    }

    #[test]
    fn old_settings_file_loads_with_bearer_and_standard() {
        let c: Config = serde_json::from_str(
            r#"{"integration":{"deviceName":"R","http":{"enabled":true,"url":"https://x.test","token":"t"},"ws":{"url":"wss://x.test","token":"u","stream":true,"request":false}}}"#,
        )
        .unwrap();
        assert_eq!(c.integration.http.auth, AuthConfig::default());
        assert_eq!(c.integration.ws.auth, AuthConfig::default());
        assert_eq!(c.integration.http.auth.mode, AuthMode::Bearer);
        assert_eq!(c.integration.http.payload, PayloadConfig::default());
        assert_eq!(c.integration.ws.payload.mode, PayloadMode::Standard);
        assert_eq!(c.integration.http.token, "t");
    }

    #[test]
    fn normalize_trims_auth_fields() {
        let auth = AuthConfig {
            mode: AuthMode::Header,
            header_name: " X-Key \n".into(),
            username: "\tuser ".into(),
            query_param: " api_key ".into(),
        };
        let c = IntegrationConfig {
            http: HttpTarget {
                auth: auth.clone(),
                ..Default::default()
            },
            ws: WsTarget {
                auth,
                payload: PayloadConfig {
                    mode: PayloadMode::Template,
                    template: " {\"a\": 1} \n".into(),
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let n = normalize(c).unwrap();
        for a in [&n.http.auth, &n.ws.auth] {
            assert_eq!(a.header_name, "X-Key");
            assert_eq!(a.username, "user");
            assert_eq!(a.query_param, "api_key");
        }
        // The template is never trimmed.
        assert_eq!(n.ws.payload.template, " {\"a\": 1} \n");
    }

    fn auth(mode: AuthMode, header: &str, user: &str, query: &str) -> AuthConfig {
        AuthConfig {
            mode,
            header_name: header.into(),
            username: user.into(),
            query_param: query.into(),
        }
    }

    #[test]
    fn normalize_rejects_invalid_auth() {
        let http = |a: AuthConfig| IntegrationConfig {
            http: HttpTarget {
                auth: a,
                ..Default::default()
            },
            ..Default::default()
        };
        let ws = |a: AuthConfig| IntegrationConfig {
            ws: WsTarget {
                auth: a,
                ..Default::default()
            },
            ..Default::default()
        };
        let bad_header = auth(AuthMode::Header, "Bad Name", "", "token");
        let bad_user = auth(AuthMode::Basic, "X-API-Key", "a:b", "token");
        let bad_query = auth(AuthMode::Query, "X-API-Key", "", "bad param");
        assert_eq!(
            normalize(http(bad_header.clone())),
            Err(ConfigError::InvalidHttpHeaderName)
        );
        assert_eq!(
            normalize(ws(bad_header)),
            Err(ConfigError::InvalidWsHeaderName)
        );
        assert_eq!(
            normalize(http(bad_user.clone())),
            Err(ConfigError::InvalidHttpUsername)
        );
        assert_eq!(normalize(ws(bad_user)), Err(ConfigError::InvalidWsUsername));
        assert_eq!(
            normalize(http(bad_query.clone())),
            Err(ConfigError::InvalidHttpQueryParam)
        );
        assert_eq!(
            normalize(ws(bad_query)),
            Err(ConfigError::InvalidWsQueryParam)
        );
        // Only the active mode's field is checked.
        assert!(normalize(http(auth(AuthMode::Bearer, "Bad Name", "a:b", "bad param"))).is_ok());
    }

    #[test]
    fn normalize_rejects_invalid_template() {
        let template = |t: &str, mode| PayloadConfig {
            mode,
            template: t.into(),
        };
        let ws = IntegrationConfig {
            ws: WsTarget {
                payload: template(r#"{"a":"{{foo}}"}"#, PayloadMode::Template),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            normalize(ws),
            Err(ConfigError::InvalidWsTemplate {
                error: TemplateError::UnknownPlaceholder { name: "foo".into() }
            })
        );
        let http = IntegrationConfig {
            http: HttpTarget {
                payload: template("not json", PayloadMode::Template),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            normalize(http),
            Err(ConfigError::InvalidHttpTemplate {
                error: TemplateError::NotJson
            })
        );
        // A bad template is ignored in standard mode.
        let standard = IntegrationConfig {
            ws: WsTarget {
                payload: template("{{foo}}", PayloadMode::Standard),
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(normalize(standard).is_ok());
    }

    #[test]
    fn config_error_serializes_template_error() {
        let e = ConfigError::InvalidHttpTemplate {
            error: TemplateError::UnknownPlaceholder { name: "foo".into() },
        };
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            r#"{"kind":"invalid_http_template","error":{"kind":"unknown_placeholder","name":"foo"}}"#
        );
    }

    #[test]
    fn config_error_serializes_with_kind_tag() {
        let json = serde_json::to_string(&ConfigError::InvalidHttpUrl).unwrap();
        assert_eq!(json, r#"{"kind":"invalid_http_url"}"#);
    }
}
