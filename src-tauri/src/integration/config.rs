use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct HttpTarget {
    pub enabled: bool,
    pub url: String,
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WsTarget {
    pub url: String,
    pub token: String,
    pub stream: bool,
    pub request: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
// The names are the `kind` tags the frontend translates.
#[allow(clippy::enum_variant_names)]
pub enum ConfigError {
    InvalidHttpUrl,
    InvalidWsUrl,
    InvalidHttpToken,
    InvalidWsToken,
}

/// Trims all strings and validates the URLs and tokens. An empty URL is valid (target not
/// configured).
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
        assert!(json.contains(r#""ws":{"url":"","token":"","stream":false,"request":false}"#));
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
    fn config_error_serializes_with_kind_tag() {
        let json = serde_json::to_string(&ConfigError::InvalidHttpUrl).unwrap();
        assert_eq!(json, r#"{"kind":"invalid_http_url"}"#);
    }
}
