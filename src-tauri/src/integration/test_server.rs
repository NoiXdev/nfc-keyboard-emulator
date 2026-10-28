//! Local WebSocket server for the `ws` tests: records the `Authorization` header, can reject
//! the handshake with a status code and hands every accepted connection to the test.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_hdr_async, WebSocketStream};

const WAIT: Duration = Duration::from_secs(3);

pub struct TestServer {
    pub url: String,
    attempts: Arc<AtomicUsize>,
    conns: mpsc::UnboundedReceiver<Conn>,
}

pub struct Conn {
    pub auth: Option<String>,
    ws: WebSocketStream<TcpStream>,
}

impl TestServer {
    /// Starts a server on a free local port. With `reject`, every handshake is answered
    /// with that HTTP status instead of being upgraded.
    pub async fn start(reject: Option<u16>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}/ws", listener.local_addr().unwrap());
        let attempts = Arc::new(AtomicUsize::new(0));
        let (tx, conns) = mpsc::unbounded_channel();
        let counter = attempts.clone();
        tokio::spawn(async move {
            while let Ok((sock, _)) = listener.accept().await {
                counter.fetch_add(1, Ordering::SeqCst);
                let tx = tx.clone();
                tokio::spawn(async move {
                    let mut auth = None;
                    // The error type is fixed by tungstenite's `Callback`.
                    #[allow(clippy::result_large_err)]
                    let callback = |req: &Request, resp: Response| {
                        auth = req
                            .headers()
                            .get(AUTHORIZATION)
                            .map(|v| v.to_str().unwrap().to_string());
                        match reject {
                            Some(code) => {
                                let mut err = ErrorResponse::new(None);
                                *err.status_mut() = StatusCode::from_u16(code).unwrap();
                                Err(err)
                            }
                            None => Ok(resp),
                        }
                    };
                    if let Ok(ws) = accept_hdr_async(sock, callback).await {
                        let _ = tx.send(Conn { auth, ws });
                    }
                });
            }
        });
        Self {
            url,
            attempts,
            conns,
        }
    }

    /// Number of TCP connections the server has seen, upgraded or not.
    pub fn attempts(&self) -> usize {
        self.attempts.load(Ordering::SeqCst)
    }

    pub async fn accept(&mut self) -> Conn {
        timeout(WAIT, self.conns.recv())
            .await
            .expect("no websocket connection arrived")
            .unwrap()
    }
}

impl Conn {
    /// The next text frame as JSON; control frames are skipped.
    pub async fn recv(&mut self) -> Value {
        let text = timeout(WAIT, async {
            loop {
                match self.ws.next().await {
                    Some(Ok(Message::Text(t))) => return t.to_string(),
                    Some(Ok(_)) => continue,
                    other => panic!("connection ended: {other:?}"),
                }
            }
        })
        .await
        .expect("no text frame arrived");
        serde_json::from_str(&text).unwrap()
    }

    pub async fn expect_ping(&mut self) {
        timeout(WAIT, async {
            loop {
                match self.ws.next().await {
                    Some(Ok(Message::Ping(_))) => return,
                    Some(Ok(_)) => continue,
                    other => panic!("connection ended: {other:?}"),
                }
            }
        })
        .await
        .expect("no ping arrived");
    }

    /// Fails if a text frame arrives within `wait`.
    pub async fn expect_silence(&mut self, wait: Duration) {
        let got = timeout(wait, async {
            loop {
                match self.ws.next().await {
                    Some(Ok(Message::Text(t))) => return Some(t.to_string()),
                    Some(Ok(_)) => continue,
                    _ => return None,
                }
            }
        })
        .await;
        if let Ok(Some(text)) = got {
            panic!("unexpected frame: {text}");
        }
    }

    pub async fn send_text(&mut self, text: &str) {
        self.ws.send(Message::text(text)).await.unwrap();
    }

    pub async fn send(&mut self, msg: Message) {
        self.ws.send(msg).await.unwrap();
    }

    pub async fn close(mut self) {
        let _ = self.ws.close(None).await;
    }
}
