use std::error::Error as _;
use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use reqwest::StatusCode;
use serde_json::Value;
use url::Url;

use super::auth::{credential, with_query};
use super::config::HttpTarget;
use super::protocol::{ClientMessage, DeliveryError, PROTOCOL_VERSION};
use super::template::{example_payload, render_template, PayloadMode};

const HTTP_TIMEOUT: Duration = Duration::from_secs(5);

/// Installs ring as the process-wide rustls crypto provider (reqwest is built with
/// `rustls-no-provider` so that aws-lc is not pulled in). Safe to call repeatedly: an
/// already installed provider is kept.
pub fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

pub fn http_client(app_version: &str) -> reqwest::Client {
    install_crypto_provider();
    reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        // A redirect would turn the POST into a GET or re-send scan data to a host the user
        // never configured; a 3xx is reported as an HTTP status error instead.
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(format!("nfc-keyboard-emulator/{app_version}"))
        .build()
        .expect("static HTTP client configuration is valid")
}

pub async fn post_message(
    client: &reqwest::Client,
    target: &HttpTarget,
    body: &Value,
) -> Result<(), DeliveryError> {
    let url = match Url::parse(&target.url) {
        Ok(url) if matches!(url.scheme(), "http" | "https") && url.host().is_some() => url,
        _ => return Err(DeliveryError::InvalidUrl),
    };
    let origin = origin_of(&url);

    let cred = credential(&target.auth, &target.token);
    let mut request = client.post(with_query(&url, &cred)).json(body);
    if let Some((name, value)) = &cred.header {
        let invalid = || DeliveryError::Unreachable {
            message: format!("invalid credentials ({origin})"),
        };
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| invalid())?;
        let mut value = HeaderValue::from_str(value).map_err(|_| invalid())?;
        // A sensitive value is never debug-printed.
        value.set_sensitive(true);
        request = request.header(name, value);
    }

    let response = request.send().await.map_err(|e| classify(&e, &origin))?;
    match response.status() {
        s if s.is_success() => Ok(()),
        s @ (StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) => {
            Err(DeliveryError::TokenRejected { status: s.as_u16() })
        }
        s => Err(DeliveryError::HttpStatus { status: s.as_u16() }),
    }
}

/// With a template the test sends the template rendered with an example scan, so the receiver
/// sees the shape it will get; otherwise the standard `test` message.
pub async fn test_http(
    client: &reqwest::Client,
    target: &HttpTarget,
    device: &str,
) -> Result<(), DeliveryError> {
    let body = match target.payload.mode {
        PayloadMode::Standard => serde_json::to_value(ClientMessage::Test {
            protocol_version: PROTOCOL_VERSION,
            device: device.to_string(),
        })
        .map_err(|_| DeliveryError::InvalidPayload)?,
        PayloadMode::Template => {
            render_template(&target.payload.template, &example_payload(device))
                .map_err(|_| DeliveryError::InvalidPayload)?
        }
    };
    post_message(client, target, &body).await
}

/// `scheme://host[:port]` only: path and query may carry secrets and are never reported.
pub(crate) fn origin_of(url: &Url) -> String {
    let host = url.host_str().unwrap_or_default();
    match url.port() {
        Some(port) => format!("{}://{host}:{port}", url.scheme()),
        None => format!("{}://{host}", url.scheme()),
    }
}

/// Maps a transport error to a `DeliveryError` without using reqwest's `Display`,
/// which includes the full URL.
fn classify(err: &reqwest::Error, origin: &str) -> DeliveryError {
    if err.is_timeout() {
        return DeliveryError::Timeout;
    }
    let reason = if err.is_connect() {
        connect_reason(err)
    } else {
        "request failed"
    };
    DeliveryError::Unreachable {
        message: format!("{reason} ({origin})"),
    }
}

fn connect_reason(err: &reqwest::Error) -> &'static str {
    let mut source = err.source();
    while let Some(cause) = source {
        if let Some(io) = cause.downcast_ref::<std::io::Error>() {
            if io.kind() == std::io::ErrorKind::ConnectionRefused {
                return "connection refused";
            }
        }
        if cause.to_string().contains("dns error") {
            return "dns error";
        }
        source = cause.source();
    }
    "connection failed"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::auth::{AuthConfig, AuthMode};
    use crate::integration::protocol::{scan_body, scan_payload};
    use crate::integration::template::{PayloadConfig, PayloadMode};
    use crate::scan::{ScanRecord, ScanStatus};
    use serde_json::Value;
    use std::time::{Duration, Instant};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Serves exactly one connection, records the raw request and answers with `status`.
    async fn serve_once(status: u16) -> (String, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let handle = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut raw = Vec::new();
            let mut buf = [0u8; 1024];
            loop {
                let n = sock.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                raw.extend_from_slice(&buf[..n]);
                if request_complete(&raw) {
                    break;
                }
            }
            let location = if (300..400).contains(&status) {
                "Location: /elsewhere\r\n"
            } else {
                ""
            };
            let reply = format!(
                "HTTP/1.1 {status} X\r\n{location}Content-Length: 0\r\nConnection: close\r\n\r\n"
            );
            sock.write_all(reply.as_bytes()).await.unwrap();
            sock.shutdown().await.ok();
            String::from_utf8_lossy(&raw).into_owned()
        });
        (base, handle)
    }

    fn request_complete(raw: &[u8]) -> bool {
        let text = String::from_utf8_lossy(raw);
        let Some((head, body)) = text.split_once("\r\n\r\n") else {
            return false;
        };
        let len = head
            .lines()
            .find_map(|l| {
                let (k, v) = l.split_once(':')?;
                k.eq_ignore_ascii_case("content-length")
                    .then(|| v.trim().parse::<usize>().ok())?
            })
            .unwrap_or(0);
        body.len() >= len
    }

    fn body_json(raw: &str) -> Value {
        let (_, body) = raw.split_once("\r\n\r\n").unwrap();
        serde_json::from_str(body).unwrap()
    }

    fn target(url: String, token: &str) -> HttpTarget {
        HttpTarget {
            enabled: true,
            url,
            token: token.into(),
            ..Default::default()
        }
    }

    fn scan_message() -> Value {
        let rec = ScanRecord {
            timestamp: "2026-10-28T13:45:21.123Z".into(),
            reader: "ACS ACR122U PICC Interface".into(),
            uid_hex: "04A2B3C4D5E6F7".into(),
            uid_length: 7,
            typed: true,
            typed_string: "04A2B3C4D5E6F7\n".into(),
            status: ScanStatus::Ok,
            scan_id: "6f1c9a52-6c1e-4a59-9a43-1d1f5b0f2b7e".into(),
            uid_formatted: "04:A2:B3:C4:D5:E6:F7".into(),
            request_id: None,
            delivery_targets: vec![],
        };
        scan_body(&PayloadConfig::default(), scan_payload(&rec, "Reception")).unwrap()
    }

    fn with_auth(mut t: HttpTarget, auth: AuthConfig) -> HttpTarget {
        t.auth = auth;
        t
    }

    fn auth(mode: AuthMode) -> AuthConfig {
        AuthConfig {
            mode,
            ..AuthConfig::default()
        }
    }

    fn template(text: &str) -> PayloadConfig {
        PayloadConfig {
            mode: PayloadMode::Template,
            template: text.into(),
        }
    }

    #[tokio::test]
    async fn posts_json_with_bearer_token() {
        let (base, server) = serve_once(204).await;
        let client = http_client("1.2.3");
        let t = target(format!("{base}/scans"), "secret");
        post_message(&client, &t, &scan_message()).await.unwrap();

        let raw = server.await.unwrap();
        let lower = raw.to_lowercase();
        assert!(raw.starts_with("POST /scans HTTP/1.1"), "{raw}");
        assert!(
            lower.contains("\r\nauthorization: bearer secret\r\n"),
            "{raw}"
        );
        assert!(
            lower.contains("\r\ncontent-type: application/json\r\n"),
            "{raw}"
        );
        assert!(
            lower.contains("\r\nuser-agent: nfc-keyboard-emulator/1.2.3\r\n"),
            "{raw}"
        );
        let body = body_json(&raw);
        assert_eq!(body["type"], "scan");
        assert_eq!(body["uid"], "04A2B3C4D5E6F7");
    }

    #[tokio::test]
    async fn empty_token_omits_authorization_header() {
        let (base, server) = serve_once(200).await;
        let client = http_client("1.2.3");
        post_message(&client, &target(base, ""), &scan_message())
            .await
            .unwrap();
        let raw = server.await.unwrap().to_lowercase();
        assert!(!raw.contains("authorization"), "{raw}");
    }

    #[tokio::test]
    async fn status_mapping() {
        let client = http_client("1.2.3");
        for (status, expected) in [
            (204, Ok(())),
            (401, Err(DeliveryError::TokenRejected { status: 401 })),
            (403, Err(DeliveryError::TokenRejected { status: 403 })),
            (500, Err(DeliveryError::HttpStatus { status: 500 })),
            (302, Err(DeliveryError::HttpStatus { status: 302 })),
        ] {
            let (base, server) = serve_once(status).await;
            let got = post_message(&client, &target(base, "t"), &scan_message()).await;
            server.await.unwrap();
            assert_eq!(got, expected, "status {status}");
        }
    }

    #[tokio::test]
    async fn silent_server_times_out() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let _server = tokio::spawn(async move {
            let (_sock, _) = listener.accept().await.unwrap();
            std::future::pending::<()>().await;
        });
        let client = http_client("1.2.3");
        let started = Instant::now();
        let got = post_message(&client, &target(base, "t"), &scan_message()).await;
        assert_eq!(got, Err(DeliveryError::Timeout));
        assert!(started.elapsed() < Duration::from_secs(6));
    }

    #[tokio::test]
    async fn closed_port_is_unreachable_without_leaking_token() {
        // Bind to learn a free port, then drop the listener so nothing listens on it.
        let port = {
            let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
            l.local_addr().unwrap().port()
        };
        let client = http_client("1.2.3");
        let t = target(
            format!("http://127.0.0.1:{port}/hook?key=urlsecret"),
            "tok-123",
        );
        let got = post_message(&client, &t, &scan_message()).await;
        let Err(DeliveryError::Unreachable { message }) = got else {
            panic!("expected Unreachable, got {got:?}");
        };
        assert!(
            message.starts_with("connection refused (http://127.0.0.1:"),
            "{message}"
        );
        assert!(!message.contains("tok-123"), "{message}");
        assert!(
            !message.to_lowercase().contains("authorization"),
            "{message}"
        );
        assert!(!message.contains("urlsecret"), "{message}");
        assert!(!message.contains("/hook"), "{message}");
    }

    #[tokio::test]
    async fn https_to_closed_port_is_unreachable() {
        // Exercises the TLS stack setup: a missing rustls crypto provider would panic here.
        let port = {
            let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
            l.local_addr().unwrap().port()
        };
        let client = http_client("1.2.3");
        let t = target(format!("https://127.0.0.1:{port}/hook"), "tok-123");
        let got = post_message(&client, &t, &scan_message()).await;
        let Err(DeliveryError::Unreachable { message }) = got else {
            panic!("expected Unreachable, got {got:?}");
        };
        assert!(
            message.starts_with("connection refused (https://127.0.0.1:"),
            "{message}"
        );
    }

    #[tokio::test]
    async fn unparsable_url_is_invalid() {
        let client = http_client("1.2.3");
        for url in ["", "not a url", "ftp://example.com/x"] {
            let got = post_message(&client, &target(url.into(), "t"), &scan_message()).await;
            assert_eq!(got, Err(DeliveryError::InvalidUrl), "{url:?}");
        }
    }

    #[tokio::test]
    async fn test_message_is_sent() {
        let (base, server) = serve_once(200).await;
        let client = http_client("1.2.3");
        test_http(&client, &target(base, "t"), "Reception")
            .await
            .unwrap();
        let body = body_json(&server.await.unwrap());
        assert_eq!(body["type"], "test");
        assert_eq!(body["device"], "Reception");
    }

    #[tokio::test]
    async fn custom_header_is_sent() {
        let (base, server) = serve_once(200).await;
        let client = http_client("1.2.3");
        let t = with_auth(target(base, "secret"), auth(AuthMode::Header));
        post_message(&client, &t, &scan_message()).await.unwrap();
        let raw = server.await.unwrap().to_lowercase();
        assert!(raw.contains("\r\nx-api-key: secret\r\n"), "{raw}");
        assert!(!raw.contains("authorization"), "{raw}");
    }

    #[tokio::test]
    async fn basic_header_is_sent() {
        let (base, server) = serve_once(200).await;
        let client = http_client("1.2.3");
        let a = AuthConfig {
            username: "user".into(),
            ..auth(AuthMode::Basic)
        };
        let t = with_auth(target(base, "pass"), a);
        post_message(&client, &t, &scan_message()).await.unwrap();
        let raw = server.await.unwrap().to_lowercase();
        assert!(
            raw.contains("\r\nauthorization: basic dxnlcjpwyxnz\r\n"),
            "{raw}"
        );
    }

    #[tokio::test]
    async fn query_token_is_appended() {
        let (base, server) = serve_once(200).await;
        let client = http_client("1.2.3");
        let t = with_auth(
            target(format!("{base}/scans"), "secret"),
            auth(AuthMode::Query),
        );
        post_message(&client, &t, &scan_message()).await.unwrap();
        let raw = server.await.unwrap();
        assert!(
            raw.starts_with("POST /scans?token=secret HTTP/1.1"),
            "{raw}"
        );
        assert!(!raw.to_lowercase().contains("authorization"), "{raw}");
    }

    #[tokio::test]
    async fn none_sends_no_authorization() {
        let (base, server) = serve_once(200).await;
        let client = http_client("1.2.3");
        let t = with_auth(target(base, "secret"), auth(AuthMode::None));
        post_message(&client, &t, &scan_message()).await.unwrap();
        let raw = server.await.unwrap();
        assert!(!raw.to_lowercase().contains("authorization"), "{raw}");
        assert!(!raw.contains("secret"), "{raw}");
    }

    #[tokio::test]
    async fn unusable_header_value_is_invalid_credentials() {
        let client = http_client("1.2.3");
        // A newline can not be part of a header value; the token must not show up in the error.
        let t = with_auth(
            target("http://127.0.0.1:9/hook?k=urlsecret".into(), "bad\nsecret"),
            auth(AuthMode::Header),
        );
        let got = post_message(&client, &t, &scan_message()).await;
        assert_eq!(
            got,
            Err(DeliveryError::Unreachable {
                message: "invalid credentials (http://127.0.0.1:9)".into()
            })
        );
    }

    #[tokio::test]
    async fn template_body_is_sent() {
        let (base, server) = serve_once(200).await;
        let client = http_client("1.2.3");
        let tpl = template(r#"{"card":"{{uid}}","len":"{{uidLength}}"}"#);
        let rec_payload = crate::integration::template::example_payload("Reception");
        let body = scan_body(&tpl, rec_payload).unwrap();
        post_message(&client, &target(base, "t"), &body)
            .await
            .unwrap();
        let sent = body_json(&server.await.unwrap());
        assert_eq!(sent, body);
        assert_eq!(sent["card"], "04A1B2C3D4E5F6");
        assert_eq!(sent["len"], 7);
    }

    #[tokio::test]
    async fn test_with_template_sends_example_scan() {
        let (base, server) = serve_once(200).await;
        let client = http_client("1.2.3");
        let mut t = target(base, "t");
        t.payload = template(r#"{"scanId":"{{scanId}}","dev":"{{device}}"}"#);
        test_http(&client, &t, "Reception").await.unwrap();
        let body = body_json(&server.await.unwrap());
        assert_eq!(body["scanId"], "test");
        assert_eq!(body["dev"], "Reception");
        assert!(body.get("type").is_none());
    }

    #[tokio::test]
    async fn test_with_invalid_template_is_invalid_payload() {
        let client = http_client("1.2.3");
        let mut t = target("http://127.0.0.1:9".into(), "t");
        t.payload = template("not json");
        assert_eq!(
            test_http(&client, &t, "Reception").await,
            Err(DeliveryError::InvalidPayload)
        );
    }

    #[tokio::test]
    async fn query_token_not_in_error_on_closed_port() {
        let port = {
            let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
            l.local_addr().unwrap().port()
        };
        let client = http_client("1.2.3");
        let t = with_auth(
            target(format!("http://127.0.0.1:{port}/hook"), "secret"),
            auth(AuthMode::Query),
        );
        let err = post_message(&client, &t, &scan_message())
            .await
            .unwrap_err();
        let json = serde_json::to_string(&err).unwrap();
        assert!(!json.contains("secret"), "{json}");
        assert!(!json.contains("token="), "{json}");
    }

    #[tokio::test]
    async fn query_token_not_in_error_on_401() {
        let (base, server) = serve_once(401).await;
        let client = http_client("1.2.3");
        let t = with_auth(
            target(format!("{base}/scans"), "secret"),
            auth(AuthMode::Query),
        );
        let err = post_message(&client, &t, &scan_message())
            .await
            .unwrap_err();
        server.await.unwrap();
        assert_eq!(err, DeliveryError::TokenRejected { status: 401 });
        let json = serde_json::to_string(&err).unwrap();
        assert!(!json.contains("secret"), "{json}");
    }
}
