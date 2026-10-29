use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use url::Url;

/// How the server token travels with a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    #[default]
    Bearer,
    Header,
    Basic,
    Query,
    None,
}

/// Where and under which name the token is sent. Only the field of the active mode matters; the
/// others are kept so switching modes back and forth loses nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuthConfig {
    pub mode: AuthMode,
    pub header_name: String,
    pub username: String,
    pub query_param: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: AuthMode::Bearer,
            header_name: "X-API-Key".to_string(),
            username: String::new(),
            query_param: "token".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthError {
    HeaderName,
    Username,
    QueryParam,
}

/// Headers the transport owns; a token header must not clobber them.
const RESERVED_HEADERS: [&str; 6] = [
    "host",
    "content-length",
    "content-type",
    "connection",
    "upgrade",
    "transfer-encoding",
];

/// Checks only the field the active mode uses.
pub fn validate_auth(auth: &AuthConfig) -> Result<(), AuthError> {
    match auth.mode {
        AuthMode::Header if !valid_header_name(&auth.header_name) => Err(AuthError::HeaderName),
        AuthMode::Basic if !valid_username(&auth.username) => Err(AuthError::Username),
        AuthMode::Query if !valid_query_param(&auth.query_param) => Err(AuthError::QueryParam),
        _ => Ok(()),
    }
}

/// An RFC 7230 token that is not a header the transport sets itself.
fn valid_header_name(name: &str) -> bool {
    const TOKEN_PUNCT: &str = "!#$%&'*+-.^_`|~";
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || TOKEN_PUNCT.contains(c))
    {
        return false;
    }
    let lower = name.to_ascii_lowercase();
    !RESERVED_HEADERS.contains(&lower.as_str()) && !lower.starts_with("sec-websocket-")
}

/// Visible ASCII without `:`, which would split user and password. Empty is allowed.
fn valid_username(name: &str) -> bool {
    name.bytes().all(|b| b.is_ascii_graphic() && b != b':')
}

fn valid_query_param(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'~' | b'-'))
}

/// The credential as it goes on the wire. Deliberately not `Debug`: it holds the secret.
pub struct Credential {
    pub header: Option<(String, String)>,
    pub query: Option<(String, String)>,
}

pub fn credential(auth: &AuthConfig, token: &str) -> Credential {
    let mut cred = Credential {
        header: None,
        query: None,
    };
    match auth.mode {
        AuthMode::Bearer if !token.is_empty() => {
            cred.header = Some(("Authorization".to_string(), format!("Bearer {token}")));
        }
        AuthMode::Header if !token.is_empty() => {
            cred.header = Some((auth.header_name.clone(), token.to_string()));
        }
        AuthMode::Basic if !auth.username.is_empty() || !token.is_empty() => {
            let encoded = STANDARD.encode(format!("{}:{token}", auth.username));
            cred.header = Some(("Authorization".to_string(), format!("Basic {encoded}")));
        }
        AuthMode::Query if !token.is_empty() => {
            cred.query = Some((auth.query_param.clone(), token.to_string()));
        }
        _ => {}
    }
    cred
}

/// Appends the query credential to `url`, keeping any existing query and the fragment.
pub fn with_query(url: &Url, cred: &Credential) -> Url {
    let mut url = url.clone();
    if let Some((name, value)) = &cred.query {
        url.query_pairs_mut().append_pair(name, value);
    }
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(mode: AuthMode) -> AuthConfig {
        AuthConfig {
            mode,
            ..AuthConfig::default()
        }
    }

    #[test]
    fn default_is_bearer() {
        let a = AuthConfig::default();
        assert_eq!(a.mode, AuthMode::Bearer);
        assert_eq!(a.header_name, "X-API-Key");
        assert_eq!(a.username, "");
        assert_eq!(a.query_param, "token");
    }

    #[test]
    fn bearer_header() {
        let c = credential(&cfg(AuthMode::Bearer), "abc");
        assert_eq!(
            c.header,
            Some(("Authorization".to_string(), "Bearer abc".to_string()))
        );
        assert!(c.query.is_none());
    }

    #[test]
    fn custom_header() {
        let c = credential(&cfg(AuthMode::Header), "abc");
        assert_eq!(c.header, Some(("X-API-Key".to_string(), "abc".to_string())));
        assert!(c.query.is_none());
    }

    #[test]
    fn basic_value() {
        let a = AuthConfig {
            username: "user".into(),
            ..cfg(AuthMode::Basic)
        };
        let c = credential(&a, "pass");
        assert_eq!(
            c.header,
            Some((
                "Authorization".to_string(),
                "Basic dXNlcjpwYXNz".to_string()
            ))
        );
    }

    #[test]
    fn basic_without_password() {
        let a = AuthConfig {
            username: "user".into(),
            ..cfg(AuthMode::Basic)
        };
        let c = credential(&a, "");
        assert_eq!(
            c.header,
            Some(("Authorization".to_string(), "Basic dXNlcjo=".to_string()))
        );
    }

    #[test]
    fn empty_token_sends_nothing() {
        for mode in [AuthMode::Bearer, AuthMode::Header, AuthMode::Query] {
            let c = credential(&cfg(mode), "");
            assert!(c.header.is_none(), "{mode:?}");
            assert!(c.query.is_none(), "{mode:?}");
        }
    }

    #[test]
    fn none_sends_nothing() {
        let c = credential(&cfg(AuthMode::None), "abc");
        assert!(c.header.is_none());
        assert!(c.query.is_none());
    }

    #[test]
    fn query_appends_and_keeps_existing() {
        let c = credential(&cfg(AuthMode::Query), "a&b");
        assert!(c.header.is_none());
        let url = Url::parse("https://x/a?b=1#f").unwrap();
        assert_eq!(
            with_query(&url, &c).as_str(),
            "https://x/a?b=1&token=a%26b#f"
        );
    }

    #[test]
    fn with_query_keeps_existing_query_verbatim() {
        let c = credential(&cfg(AuthMode::Query), "a b");
        for existing in ["q=b%20c", "q=b+c&r=%2F", "q=%zz&flag", "a=1&&b=2"] {
            let url = Url::parse(&format!("https://x/a?{existing}")).unwrap();
            let got = with_query(&url, &c);
            assert_eq!(
                got.query(),
                Some(format!("{existing}&token=a+b").as_str()),
                "{existing}"
            );
        }
    }

    #[test]
    fn with_query_leaves_url_alone_without_query_credential() {
        let c = credential(&cfg(AuthMode::Bearer), "abc");
        let url = Url::parse("https://x/a?b=1#f").unwrap();
        assert_eq!(with_query(&url, &c), url);
    }

    #[test]
    fn rejects_bad_header_names() {
        for name in ["", "X Key", "Host", "sec-websocket-key", "Content-Type"] {
            let a = AuthConfig {
                header_name: name.into(),
                ..cfg(AuthMode::Header)
            };
            assert_eq!(validate_auth(&a), Err(AuthError::HeaderName), "{name:?}");
        }
    }

    #[test]
    fn accepts_x_api_key() {
        assert_eq!(validate_auth(&cfg(AuthMode::Header)), Ok(()));
    }

    #[test]
    fn rejects_username_with_colon() {
        let a = AuthConfig {
            username: "a:b".into(),
            ..cfg(AuthMode::Basic)
        };
        assert_eq!(validate_auth(&a), Err(AuthError::Username));
    }

    #[test]
    fn rejects_bad_query_param() {
        for name in ["", "a b", "a&b"] {
            let a = AuthConfig {
                query_param: name.into(),
                ..cfg(AuthMode::Query)
            };
            assert_eq!(validate_auth(&a), Err(AuthError::QueryParam), "{name:?}");
        }
    }

    #[test]
    fn only_active_mode_is_validated() {
        let a = AuthConfig {
            header_name: "bad name".into(),
            username: "a:b".into(),
            query_param: "a b".into(),
            ..cfg(AuthMode::Bearer)
        };
        assert_eq!(validate_auth(&a), Ok(()));
    }

    #[test]
    fn serde_roundtrip_camelcase() {
        let json =
            r#"{"mode":"basic","headerName":"X-API-Key","username":"u","queryParam":"token"}"#;
        let a: AuthConfig = serde_json::from_str(json).unwrap();
        assert_eq!(a.mode, AuthMode::Basic);
        assert_eq!(a.username, "u");
        let back = serde_json::to_value(&a).unwrap();
        assert_eq!(
            back,
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
    }

    #[test]
    fn missing_fields_take_defaults() {
        let a: AuthConfig = serde_json::from_str(r#"{"mode":"header"}"#).unwrap();
        assert_eq!(a.mode, AuthMode::Header);
        assert_eq!(a.header_name, "X-API-Key");
        assert_eq!(a.query_param, "token");
    }
}
