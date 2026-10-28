use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{interval_at, timeout, MissedTickBehavior};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::{HeaderValue, AUTHORIZATION};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use url::Url;

use super::backoff::Backoff;
use super::config::WsTarget;
use super::http::{install_crypto_provider, origin_of};
use super::protocol::{
    parse_server_message, scan_payload, ClientMessage, DeliveryError, DeliveryResult, EndReason,
    Modes, ScanPayload, ServerMessage, PROTOCOL_VERSION,
};
use super::requests::RequestSlot;
use crate::scan::{DeliveryTarget, ScanRecord};

/// Upper bound for TCP connect, TLS and the WebSocket handshake together.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// A graceful close must not hold up a reconnect or shutdown.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(1);
/// A write the peer does not take within this time counts as a dead connection.
const SEND_TIMEOUT: Duration = Duration::from_secs(10);

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Deliberately not `Debug` (although `WsTarget` derives it), so these settings never
/// print the token.
#[derive(Clone, PartialEq, Eq)]
pub struct WsSettings {
    pub target: WsTarget,
    pub device: String,
    pub app_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum WsState {
    Off,
    Connecting,
    Connected,
    /// `lost` marks the drop worth announcing: the first one after a fresh start or after
    /// a stable session. A server that keeps accepting and closing is announced once.
    Reconnecting {
        error: DeliveryError,
        lost: bool,
    },
    TokenRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WsStatus {
    #[serde(flatten)]
    pub state: WsState,
    pub request_pending: bool,
}

pub trait WsEvents: Send + Sync + 'static {
    fn status(&self, s: &WsStatus);
    fn delivery(&self, r: &DeliveryResult);
    fn request_armed(&self, request_id: &str);
}

/// Keepalive, dead-peer, stability and request-expiry periods; injectable so tests do not
/// wait 30 s.
#[derive(Debug, Clone, Copy)]
struct WsTimings {
    ping: Duration,
    /// How long the peer may stay silent beyond one ping interval before the session
    /// counts as dead.
    grace: Duration,
    /// A session that stayed up this long resets the backoff and the "lost" announcement.
    stable: Duration,
    tick: Duration,
}

impl Default for WsTimings {
    fn default() -> Self {
        Self {
            ping: Duration::from_secs(30),
            grace: Duration::from_secs(10),
            stable: Duration::from_secs(30),
            tick: Duration::from_secs(1),
        }
    }
}

struct OutgoingScan {
    request_id: Option<String>,
    payload: ScanPayload,
}

enum Command {
    Update(WsSettings),
    Scan(OutgoingScan),
    Reconnect,
}

/// Controls the background connection. All methods only enqueue and never block.
pub struct WsHandle {
    commands: mpsc::UnboundedSender<Command>,
    status: Arc<Mutex<WsStatus>>,
}

impl WsHandle {
    pub fn update(&self, s: WsSettings) {
        let _ = self.commands.send(Command::Update(s));
    }

    pub fn send_scan(&self, rec: &ScanRecord, device: &str) {
        let _ = self.commands.send(Command::Scan(OutgoingScan {
            request_id: rec.request_id.clone(),
            payload: scan_payload(rec, device),
        }));
    }

    /// Retries now, also after a rejected token.
    pub fn reconnect(&self) {
        let _ = self.commands.send(Command::Reconnect);
    }

    /// The last published status.
    pub fn status(&self) -> WsStatus {
        self.status.lock().unwrap().clone()
    }
}

pub fn spawn_ws(
    settings: WsSettings,
    slot: Arc<RequestSlot>,
    events: Arc<dyn WsEvents>,
) -> WsHandle {
    let (handle, connection) = create(settings, slot, events, WsTimings::default());
    tauri::async_runtime::spawn(connection.run());
    handle
}

/// Connects, sends `hello` and closes again; reports whether the handshake succeeded.
pub async fn test_ws(
    target: &WsTarget,
    device: &str,
    app_version: &str,
) -> Result<(), DeliveryError> {
    let (mut ws, origin) = connect(target.clone()).await?;
    let hello = hello_message(target, device, app_version);
    send(&mut ws, &hello, &origin).await?;
    close(&mut ws).await;
    Ok(())
}

fn create(
    settings: WsSettings,
    slot: Arc<RequestSlot>,
    events: Arc<dyn WsEvents>,
    timings: WsTimings,
) -> (WsHandle, Connection) {
    let (tx, rx) = mpsc::unbounded_channel();
    let status = Arc::new(Mutex::new(WsStatus {
        state: WsState::Off,
        request_pending: false,
    }));
    let handle = WsHandle {
        commands: tx,
        status: status.clone(),
    };
    let connection = Connection {
        settings,
        slot,
        events,
        timings,
        status,
        commands: rx,
    };
    (handle, connection)
}

enum Phase {
    Off,
    /// `fresh` attempts announce `Connecting` and start with a reset backoff; retries
    /// stay in `Reconnecting`.
    Connect {
        fresh: bool,
    },
    Retry(DeliveryError),
    /// Like `Retry`, after a drop that is announced as `lost`.
    Lost(DeliveryError),
    Rejected,
}

enum Attempt {
    Connected(Box<WsStream>, String),
    Failed(DeliveryError),
    /// Settings changed while connecting.
    Restart,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Idle {
    Off,
    Rejected,
    Retry,
}

/// What a settings update changed.
struct Change {
    endpoint: bool,
    device: bool,
    modes: bool,
    request_off: bool,
}

enum Event {
    Frame(Option<Result<Message, WsError>>),
    Command(Option<Command>),
    Ping,
    Tick,
}

/// The connection task. Ends when the `WsHandle` is dropped.
struct Connection {
    settings: WsSettings,
    slot: Arc<RequestSlot>,
    events: Arc<dyn WsEvents>,
    timings: WsTimings,
    status: Arc<Mutex<WsStatus>>,
    commands: mpsc::UnboundedReceiver<Command>,
}

impl Connection {
    async fn run(mut self) {
        let mut backoff = Backoff::new();
        // Set once a drop was announced; only a stable session or a fresh start clears it.
        let mut drop_announced = false;
        let mut phase = if self.settings.target.active() {
            Phase::Connect { fresh: true }
        } else {
            Phase::Off
        };
        loop {
            let next = match phase {
                Phase::Off => {
                    self.publish(WsState::Off);
                    self.idle(Idle::Off, None).await
                }
                Phase::Rejected => {
                    self.publish(WsState::TokenRejected);
                    self.idle(Idle::Rejected, None).await
                }
                Phase::Retry(error) => self.retry(&mut backoff, error, false).await,
                Phase::Lost(error) => self.retry(&mut backoff, error, true).await,
                Phase::Connect { fresh } => {
                    if fresh {
                        backoff.reset();
                        drop_announced = false;
                        self.publish(WsState::Connecting);
                    }
                    match self.attempt().await {
                        Some(Attempt::Connected(ws, origin)) => {
                            let started = tokio::time::Instant::now();
                            let next = self.session(*ws, &origin).await;
                            // Only a session that stayed up counts as recovered; a server
                            // that accepts and closes at once keeps backing off.
                            if started.elapsed() >= self.timings.stable {
                                backoff.reset();
                                drop_announced = false;
                            }
                            match next {
                                Some(Phase::Retry(error)) if !drop_announced => {
                                    drop_announced = true;
                                    Some(Phase::Lost(error))
                                }
                                next => next,
                            }
                        }
                        Some(Attempt::Failed(DeliveryError::TokenRejected { .. })) => {
                            Some(Phase::Rejected)
                        }
                        Some(Attempt::Failed(error)) => Some(Phase::Retry(error)),
                        Some(Attempt::Restart) if self.settings.target.active() => {
                            Some(Phase::Connect { fresh: true })
                        }
                        Some(Attempt::Restart) => Some(Phase::Off),
                        None => None,
                    }
                }
            };
            match next {
                Some(p) => phase = p,
                None => return,
            }
        }
    }

    async fn retry(
        &mut self,
        backoff: &mut Backoff,
        error: DeliveryError,
        lost: bool,
    ) -> Option<Phase> {
        self.publish(WsState::Reconnecting { error, lost });
        let delay = backoff.next_delay(jitter());
        self.idle(Idle::Retry, Some(delay)).await
    }

    /// Waits while not connected (and, for `Retry`, until `wake` has passed). Returns the
    /// next phase, or `None` once the handle is gone.
    async fn idle(&mut self, idle: Idle, wake: Option<Duration>) -> Option<Phase> {
        let sleep = tokio::time::sleep(wake.unwrap_or(Duration::MAX));
        tokio::pin!(sleep);
        loop {
            let cmd = tokio::select! {
                _ = &mut sleep, if wake.is_some() => return Some(Phase::Connect { fresh: false }),
                cmd = self.commands.recv() => cmd?,
            };
            match cmd {
                Command::Scan(scan) => self.report_not_connected(scan),
                Command::Reconnect => {
                    if self.settings.target.active() {
                        return Some(Phase::Connect { fresh: true });
                    }
                }
                Command::Update(new) => {
                    let change = self.apply(new);
                    if !self.settings.target.active() {
                        if idle != Idle::Off {
                            return Some(Phase::Off);
                        }
                    } else if idle == Idle::Off
                        || change.endpoint
                        || (idle == Idle::Retry && change.device)
                    {
                        return Some(Phase::Connect { fresh: true });
                    }
                }
            }
        }
    }

    /// One connection attempt. Commands keep being served meanwhile; a settings change
    /// that affects the connection abandons the attempt.
    async fn attempt(&mut self) -> Option<Attempt> {
        let connecting = connect(self.settings.target.clone());
        tokio::pin!(connecting);
        loop {
            let cmd = tokio::select! {
                result = &mut connecting => return Some(match result {
                    Ok((ws, origin)) => Attempt::Connected(Box::new(ws), origin),
                    Err(error) => Attempt::Failed(error),
                }),
                cmd = self.commands.recv() => cmd?,
            };
            match cmd {
                Command::Scan(scan) => self.report_not_connected(scan),
                Command::Reconnect => {}
                Command::Update(new) => {
                    let change = self.apply(new);
                    if !self.settings.target.active() || change.endpoint || change.device {
                        return Some(Attempt::Restart);
                    }
                }
            }
        }
    }

    async fn session(&mut self, mut ws: WsStream, origin: &str) -> Option<Phase> {
        let next = self.serve(&mut ws, origin).await;
        // Whatever ended the session, an armed request can no longer be answered here.
        self.slot.clear();
        if !matches!(next, Some(Phase::Retry(_))) {
            close(&mut ws).await;
        }
        next
    }

    async fn serve(&mut self, ws: &mut WsStream, origin: &str) -> Option<Phase> {
        if let Err(error) = send(ws, &self.hello(), origin).await {
            return Some(Phase::Retry(error));
        }
        self.publish(WsState::Connected);

        let start = tokio::time::Instant::now();
        let mut ping = interval_at(start + self.timings.ping, self.timings.ping);
        ping.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut tick = interval_at(start + self.timings.tick, self.timings.tick);
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
        // Any inbound frame, pongs included, proves the peer is still there.
        let mut last_heard = start;
        loop {
            let event = tokio::select! {
                frame = ws.next() => Event::Frame(frame),
                cmd = self.commands.recv() => Event::Command(cmd),
                _ = ping.tick() => Event::Ping,
                _ = tick.tick() => Event::Tick,
            };
            if let Event::Frame(Some(Ok(_))) = event {
                last_heard = tokio::time::Instant::now();
            }
            let result = match event {
                Event::Frame(Some(Ok(Message::Text(text)))) => {
                    self.on_server_message(ws, text.as_str(), origin).await
                }
                Event::Frame(Some(Ok(Message::Close(_))) | None) => {
                    Err(DeliveryError::Unreachable {
                        message: format!("connection closed ({origin})"),
                    })
                }
                // Binary, ping and pong frames; tungstenite answers pings itself.
                Event::Frame(Some(Ok(_))) => Ok(()),
                Event::Frame(Some(Err(e))) => Err(classify(&e, origin)),
                // A half-open socket (sleep, network change) never errors on its own.
                Event::Ping if last_heard.elapsed() > self.timings.ping + self.timings.grace => {
                    Err(no_response(origin))
                }
                Event::Ping => send_frame(ws, Message::Ping(Default::default()), origin).await,
                Event::Tick => match self.slot.expire(Instant::now()) {
                    Some(id) => self.end_request(ws, id, EndReason::Timeout, origin).await,
                    None => Ok(()),
                },
                Event::Command(None) => return None,
                Event::Command(Some(Command::Reconnect)) => {
                    return Some(Phase::Connect { fresh: true })
                }
                Event::Command(Some(Command::Scan(scan))) => self.send_scan(ws, scan, origin).await,
                Event::Command(Some(Command::Update(new))) => {
                    let change = self.apply(new);
                    let mut result = Ok(());
                    if change.request_off {
                        if let Some(id) = self.slot.clear() {
                            result = self.end_request(ws, id, EndReason::Cancelled, origin).await;
                        }
                    }
                    if !self.settings.target.active() {
                        return Some(Phase::Off);
                    }
                    if change.endpoint || change.device {
                        return Some(Phase::Connect { fresh: true });
                    }
                    if result.is_ok() && change.modes {
                        result = send(ws, &self.hello(), origin).await;
                    }
                    result
                }
            };
            if let Err(error) = result {
                return Some(Phase::Retry(error));
            }
        }
    }

    async fn on_server_message(
        &self,
        ws: &mut WsStream,
        text: &str,
        origin: &str,
    ) -> Result<(), DeliveryError> {
        match parse_server_message(text) {
            Some(ServerMessage::ScanRequest {
                request_id,
                timeout_ms,
            }) => {
                if !self.settings.target.request {
                    return self
                        .end_request(ws, request_id, EndReason::Disabled, origin)
                        .await;
                }
                let replaced = self
                    .slot
                    .arm(request_id.clone(), timeout_ms, Instant::now());
                self.events.request_armed(&request_id);
                self.publish(WsState::Connected);
                match replaced {
                    // A repeated id is the same request, not a superseded one.
                    Some(old) if old != request_id => {
                        self.end_request(ws, old, EndReason::Superseded, origin)
                            .await
                    }
                    _ => Ok(()),
                }
            }
            Some(ServerMessage::ScanRequestCancel { request_id }) => {
                if self.slot.cancel(&request_id) {
                    self.end_request(ws, request_id, EndReason::Cancelled, origin)
                        .await
                } else {
                    Ok(())
                }
            }
            Some(ServerMessage::Unknown) | None => Ok(()),
        }
    }

    async fn end_request(
        &self,
        ws: &mut WsStream,
        request_id: String,
        reason: EndReason,
        origin: &str,
    ) -> Result<(), DeliveryError> {
        self.publish(WsState::Connected);
        let msg = ClientMessage::ScanRequestEnded { request_id, reason };
        send(ws, &msg, origin).await
    }

    async fn send_scan(
        &self,
        ws: &mut WsStream,
        scan: OutgoingScan,
        origin: &str,
    ) -> Result<(), DeliveryError> {
        let answers_request = scan.request_id.is_some();
        let scan_id = scan.payload.scan_id.clone();
        let messages = self.scan_messages(scan);
        if messages.is_empty() {
            return Ok(());
        }
        // One delivery result per scan; a failed write ends the session, so stop there.
        let mut result = Ok(());
        for msg in &messages {
            result = send(ws, msg, origin).await;
            if result.is_err() {
                break;
            }
        }
        self.events.delivery(&DeliveryResult {
            scan_id,
            target: DeliveryTarget::Ws,
            ok: result.is_ok(),
            error: result.clone().err(),
        });
        if answers_request {
            // The worker has taken the request from the slot.
            self.publish(WsState::Connected);
        }
        result
    }

    /// `scan_response` for an answered request, plus `scan` whenever streaming is on.
    fn scan_messages(&self, scan: OutgoingScan) -> Vec<ClientMessage> {
        let mut messages = Vec::new();
        if let Some(request_id) = scan.request_id {
            messages.push(ClientMessage::ScanResponse {
                protocol_version: PROTOCOL_VERSION,
                request_id,
                payload: scan.payload.clone(),
            });
        }
        if self.settings.target.stream {
            messages.push(ClientMessage::Scan {
                protocol_version: PROTOCOL_VERSION,
                payload: scan.payload,
            });
        }
        messages
    }

    fn report_not_connected(&self, scan: OutgoingScan) {
        let scan_id = scan.payload.scan_id.clone();
        if !self.scan_messages(scan).is_empty() {
            self.events.delivery(&DeliveryResult {
                scan_id,
                target: DeliveryTarget::Ws,
                ok: false,
                error: Some(DeliveryError::NotConnected),
            });
        }
    }

    fn apply(&mut self, new: WsSettings) -> Change {
        let old = &self.settings;
        let change = Change {
            endpoint: old.target.url != new.target.url || old.target.token != new.target.token,
            device: old.device != new.device,
            modes: old.target.stream != new.target.stream
                || old.target.request != new.target.request,
            request_off: old.target.request && !new.target.request,
        };
        self.settings = new;
        change
    }

    fn hello(&self) -> ClientMessage {
        hello_message(
            &self.settings.target,
            &self.settings.device,
            &self.settings.app_version,
        )
    }

    /// Stores and emits the status if it changed. `request_pending` mirrors the slot.
    fn publish(&self, state: WsState) {
        let next = WsStatus {
            state,
            request_pending: self.slot.is_armed(),
        };
        {
            let mut current = self.status.lock().unwrap();
            if *current == next {
                return;
            }
            *current = next.clone();
        }
        self.events.status(&next);
    }
}

fn hello_message(target: &WsTarget, device: &str, app_version: &str) -> ClientMessage {
    ClientMessage::Hello {
        protocol_version: PROTOCOL_VERSION,
        device: device.to_string(),
        app_version: app_version.to_string(),
        modes: Modes {
            stream: target.stream,
            request: target.request,
        },
    }
}

/// Opens the connection and returns it with the `scheme://host[:port]` used in error
/// messages.
async fn connect(target: WsTarget) -> Result<(WsStream, String), DeliveryError> {
    let url = match Url::parse(&target.url) {
        Ok(url) if matches!(url.scheme(), "ws" | "wss") && url.host().is_some() => url,
        _ => return Err(DeliveryError::InvalidUrl),
    };
    let origin = origin_of(&url);
    let mut request = target
        .url
        .as_str()
        .into_client_request()
        .map_err(|_| DeliveryError::InvalidUrl)?;
    if !target.token.is_empty() {
        let mut value =
            HeaderValue::from_str(&format!("Bearer {}", target.token)).map_err(|_| {
                DeliveryError::Unreachable {
                    message: format!("invalid token characters ({origin})"),
                }
            })?;
        value.set_sensitive(true);
        request.headers_mut().insert(AUTHORIZATION, value);
    }
    install_crypto_provider();
    match timeout(CONNECT_TIMEOUT, connect_async(request)).await {
        Err(_) => Err(DeliveryError::Timeout),
        Ok(Ok((ws, _response))) => Ok((ws, origin)),
        Ok(Err(e)) => Err(classify(&e, &origin)),
    }
}

async fn send(ws: &mut WsStream, msg: &ClientMessage, origin: &str) -> Result<(), DeliveryError> {
    let json = serde_json::to_string(msg).expect("client messages always serialize");
    send_frame(ws, Message::text(json), origin).await
}

async fn send_frame(ws: &mut WsStream, frame: Message, origin: &str) -> Result<(), DeliveryError> {
    match timeout(SEND_TIMEOUT, ws.send(frame)).await {
        Ok(result) => result.map_err(|e| classify(&e, origin)),
        Err(_) => Err(no_response(origin)),
    }
}

/// The peer stopped answering pings or taking writes.
fn no_response(origin: &str) -> DeliveryError {
    DeliveryError::Unreachable {
        message: format!("no response ({origin})"),
    }
}

async fn close(ws: &mut WsStream) {
    let _ = timeout(CLOSE_TIMEOUT, ws.close(None)).await;
}

/// Maps a WebSocket error to a `DeliveryError` without using its `Display`, which may
/// include the URL or response details.
fn classify(err: &WsError, origin: &str) -> DeliveryError {
    let reason = match err {
        WsError::Http(response) => {
            let status = response.status().as_u16();
            return if matches!(status, 401 | 403) {
                DeliveryError::TokenRejected { status }
            } else {
                DeliveryError::HttpStatus { status }
            };
        }
        WsError::Url(_) | WsError::HttpFormat(_) => return DeliveryError::InvalidUrl,
        WsError::Io(io) => io_reason(io),
        WsError::Tls(_) => "tls error",
        WsError::ConnectionClosed | WsError::AlreadyClosed => "connection closed",
        _ => "protocol error",
    };
    DeliveryError::Unreachable {
        message: format!("{reason} ({origin})"),
    }
}

fn io_reason(io: &std::io::Error) -> &'static str {
    if io.kind() == std::io::ErrorKind::ConnectionRefused {
        return "connection refused";
    }
    if io
        .get_ref()
        .is_some_and(|inner| inner.downcast_ref::<rustls::Error>().is_some())
    {
        return "tls error";
    }
    if io.to_string().contains("lookup address") {
        return "dns error";
    }
    "connection failed"
}

/// A value in `[0, 1)` for the backoff jitter, taken from the clock's sub-second part.
fn jitter() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| f64::from(d.subsec_nanos()) / 1e9)
        .unwrap_or(0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::test_server::TestServer;
    use crate::scan::{DeliveryTarget, ScanRecord, ScanStatus};
    use std::time::Instant as StdInstant;
    use tokio_tungstenite::tungstenite::Message;

    const TEST_TIMINGS: WsTimings = WsTimings {
        ping: Duration::from_secs(30),
        grace: Duration::from_secs(10),
        stable: Duration::from_secs(30),
        tick: Duration::from_millis(100),
    };

    #[derive(Default)]
    struct Recorder {
        statuses: Mutex<Vec<WsStatus>>,
        deliveries: Mutex<Vec<DeliveryResult>>,
        armed: Mutex<Vec<String>>,
    }

    impl WsEvents for Recorder {
        fn status(&self, s: &WsStatus) {
            self.statuses.lock().unwrap().push(s.clone());
        }
        fn delivery(&self, r: &DeliveryResult) {
            self.deliveries.lock().unwrap().push(r.clone());
        }
        fn request_armed(&self, request_id: &str) {
            self.armed.lock().unwrap().push(request_id.to_string());
        }
    }

    struct Client {
        handle: WsHandle,
        slot: Arc<RequestSlot>,
        events: Arc<Recorder>,
    }

    fn target(url: &str, token: &str, stream: bool, request: bool) -> WsTarget {
        WsTarget {
            url: url.into(),
            token: token.into(),
            stream,
            request,
        }
    }

    fn settings(target: WsTarget) -> WsSettings {
        WsSettings {
            target,
            device: "Reception".into(),
            app_version: "1.2.3".into(),
        }
    }

    fn start(target: WsTarget) -> Client {
        start_with(target, TEST_TIMINGS)
    }

    fn start_with(target: WsTarget, timings: WsTimings) -> Client {
        let slot = Arc::new(RequestSlot::default());
        let events = Arc::new(Recorder::default());
        let (handle, connection) = create(settings(target), slot.clone(), events.clone(), timings);
        tokio::spawn(connection.run());
        Client {
            handle,
            slot,
            events,
        }
    }

    fn record(request_id: Option<&str>) -> ScanRecord {
        ScanRecord {
            timestamp: "2026-10-28T13:45:21.123Z".into(),
            reader: "ACS ACR122U PICC Interface".into(),
            uid_hex: "04A2B3C4D5E6F7".into(),
            uid_length: 7,
            typed: true,
            typed_string: "04A2B3C4D5E6F7\n".into(),
            status: ScanStatus::Ok,
            scan_id: "scan-1".into(),
            uid_formatted: "04:A2:B3:C4:D5:E6:F7".into(),
            request_id: request_id.map(String::from),
            delivery_targets: vec![],
        }
    }

    async fn wait_until(what: &str, wait: Duration, cond: impl Fn() -> bool) {
        let started = StdInstant::now();
        while !cond() {
            assert!(started.elapsed() < wait, "timed out waiting for {what}");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    fn state(c: &Client) -> WsState {
        c.handle.status().state
    }

    async fn wait_connected(c: &Client) {
        wait_until("connected", Duration::from_secs(3), || {
            state(c) == WsState::Connected
        })
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sends_auth_header_and_hello() {
        let mut server = TestServer::start(None).await;
        let c = start(target(&server.url, "secret", true, false));
        let mut conn = server.accept().await;
        assert_eq!(conn.auth.as_deref(), Some("Bearer secret"));
        let hello = conn.recv().await;
        assert_eq!(
            hello,
            serde_json::json!({
                "type": "hello",
                "protocolVersion": 1,
                "device": "Reception",
                "appVersion": "1.2.3",
                "modes": {"stream": true, "request": false}
            })
        );
        wait_connected(&c).await;
        let states: Vec<WsState> = c
            .events
            .statuses
            .lock()
            .unwrap()
            .iter()
            .map(|s| s.state.clone())
            .collect();
        assert_eq!(states, vec![WsState::Connecting, WsState::Connected]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn empty_token_omits_auth_header() {
        let mut server = TestServer::start(None).await;
        let _c = start(target(&server.url, "", true, false));
        let conn = server.accept().await;
        assert_eq!(conn.auth, None);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejected_token_stops_retrying() {
        let server = TestServer::start(Some(401)).await;
        let c = start(target(&server.url, "revoked", true, false));
        wait_until("token rejected", Duration::from_secs(3), || {
            state(&c) == WsState::TokenRejected
        })
        .await;
        tokio::time::sleep(Duration::from_secs(3)).await;
        assert_eq!(server.attempts(), 1);
        assert_eq!(state(&c), WsState::TokenRejected);

        c.handle.reconnect();
        wait_until("second attempt", Duration::from_secs(3), || {
            server.attempts() == 2
        })
        .await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stream_mode_sends_scans() {
        let mut server = TestServer::start(None).await;
        let c = start(target(&server.url, "t", true, false));
        let mut conn = server.accept().await;
        conn.recv().await; // hello
        wait_connected(&c).await;

        c.handle.send_scan(&record(None), "Reception");
        let msg = conn.recv().await;
        assert_eq!(msg["type"], "scan");
        assert_eq!(msg["uid"], "04A2B3C4D5E6F7");
        assert_eq!(msg["device"], "Reception");
        wait_until("delivery", Duration::from_secs(3), || {
            !c.events.deliveries.lock().unwrap().is_empty()
        })
        .await;
        let d = c.events.deliveries.lock().unwrap()[0].clone();
        assert_eq!(d.scan_id, "scan-1");
        assert_eq!(d.target, DeliveryTarget::Ws);
        assert!(d.ok);
        assert_eq!(d.error, None);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stream_off_does_not_send_plain_scans() {
        let mut server = TestServer::start(None).await;
        let c = start(target(&server.url, "t", false, true));
        let mut conn = server.accept().await;
        conn.recv().await; // hello
        wait_connected(&c).await;

        c.handle.send_scan(&record(None), "Reception");
        conn.expect_silence(Duration::from_millis(500)).await;
        assert!(c.events.deliveries.lock().unwrap().is_empty());

        // An answering scan goes out as the response only.
        conn.send_text(r#"{"type":"scan_request","requestId":"abc"}"#)
            .await;
        wait_until("armed", Duration::from_secs(3), || c.slot.is_armed()).await;
        let id = c.slot.take_for_scan(StdInstant::now());
        c.handle.send_scan(&record(id.as_deref()), "Reception");
        assert_eq!(conn.recv().await["type"], "scan_response");
        conn.expect_silence(Duration::from_millis(500)).await;
        assert_eq!(c.events.deliveries.lock().unwrap().len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn answering_scan_is_also_streamed() {
        let mut server = TestServer::start(None).await;
        let c = start(target(&server.url, "t", true, true));
        let mut conn = server.accept().await;
        conn.recv().await; // hello
        conn.send_text(r#"{"type":"scan_request","requestId":"abc"}"#)
            .await;
        wait_until("armed", Duration::from_secs(3), || c.slot.is_armed()).await;

        let id = c.slot.take_for_scan(StdInstant::now());
        c.handle.send_scan(&record(id.as_deref()), "Reception");
        let response = conn.recv().await;
        assert_eq!(response["type"], "scan_response");
        assert_eq!(response["requestId"], "abc");
        let scan = conn.recv().await;
        assert_eq!(scan["type"], "scan");
        assert_eq!(scan["scanId"], response["scanId"]);
        assert_eq!(scan.get("requestId"), None);

        wait_until("delivery", Duration::from_secs(3), || {
            !c.events.deliveries.lock().unwrap().is_empty()
        })
        .await;
        conn.expect_silence(Duration::from_millis(300)).await;
        let deliveries = c.events.deliveries.lock().unwrap().clone();
        assert_eq!(deliveries.len(), 1);
        assert!(deliveries[0].ok);
        assert_eq!(deliveries[0].scan_id, "scan-1");
        assert_eq!(deliveries[0].target, DeliveryTarget::Ws);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn scan_request_arms_slot_and_response_is_sent() {
        let mut server = TestServer::start(None).await;
        let c = start(target(&server.url, "t", false, true));
        let mut conn = server.accept().await;
        conn.recv().await; // hello
        conn.send_text(r#"{"type":"scan_request","requestId":"abc","timeoutMs":30000}"#)
            .await;
        wait_until("armed", Duration::from_secs(3), || c.slot.is_armed()).await;
        wait_until("request pending status", Duration::from_secs(3), || {
            c.handle.status().request_pending
        })
        .await;
        assert_eq!(*c.events.armed.lock().unwrap(), vec!["abc".to_string()]);

        // The worker takes the request for the next scan.
        let id = c.slot.take_for_scan(StdInstant::now());
        c.handle.send_scan(&record(id.as_deref()), "Reception");
        let msg = conn.recv().await;
        assert_eq!(msg["type"], "scan_response");
        assert_eq!(msg["requestId"], "abc");
        assert_eq!(msg["uid"], "04A2B3C4D5E6F7");
        wait_until("request no longer pending", Duration::from_secs(3), || {
            !c.handle.status().request_pending
        })
        .await;
        assert!(c.events.deliveries.lock().unwrap()[0].ok);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn request_while_disabled_is_declined() {
        let mut server = TestServer::start(None).await;
        let c = start(target(&server.url, "t", true, false));
        let mut conn = server.accept().await;
        conn.recv().await; // hello
        conn.send_text(r#"{"type":"scan_request","requestId":"abc"}"#)
            .await;
        let msg = conn.recv().await;
        assert_eq!(
            msg,
            serde_json::json!({"type": "scan_request_ended", "requestId": "abc", "reason": "disabled"})
        );
        assert!(!c.slot.is_armed());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn request_times_out() {
        let mut server = TestServer::start(None).await;
        let c = start(target(&server.url, "t", false, true));
        let mut conn = server.accept().await;
        conn.recv().await; // hello
        let sent = StdInstant::now();
        conn.send_text(r#"{"type":"scan_request","requestId":"abc","timeoutMs":1000}"#)
            .await;
        let msg = conn.recv().await;
        assert!(sent.elapsed() < Duration::from_millis(2500));
        assert_eq!(
            msg,
            serde_json::json!({"type": "scan_request_ended", "requestId": "abc", "reason": "timeout"})
        );
        assert!(!c.slot.is_armed());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn second_request_supersedes_first() {
        let mut server = TestServer::start(None).await;
        let c = start(target(&server.url, "t", false, true));
        let mut conn = server.accept().await;
        conn.recv().await; // hello
        conn.send_text(r#"{"type":"scan_request","requestId":"first"}"#)
            .await;
        conn.send_text(r#"{"type":"scan_request","requestId":"second"}"#)
            .await;
        let msg = conn.recv().await;
        assert_eq!(
            msg,
            serde_json::json!({"type": "scan_request_ended", "requestId": "first", "reason": "superseded"})
        );
        wait_until("second armed", Duration::from_secs(3), || {
            c.events.armed.lock().unwrap().len() == 2
        })
        .await;
        assert_eq!(
            c.slot.take_for_scan(StdInstant::now()),
            Some("second".to_string())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cancel_disarms_and_is_acknowledged() {
        let mut server = TestServer::start(None).await;
        let c = start(target(&server.url, "t", false, true));
        let mut conn = server.accept().await;
        conn.recv().await; // hello
        conn.send_text(r#"{"type":"scan_request","requestId":"abc"}"#)
            .await;
        wait_until("armed", Duration::from_secs(3), || c.slot.is_armed()).await;
        conn.send_text(r#"{"type":"scan_request_cancel","requestId":"other"}"#)
            .await;
        conn.send_text(r#"{"type":"scan_request_cancel","requestId":"abc"}"#)
            .await;
        let msg = conn.recv().await;
        assert_eq!(
            msg,
            serde_json::json!({"type": "scan_request_ended", "requestId": "abc", "reason": "cancelled"})
        );
        assert!(!c.slot.is_armed());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn garbage_frames_keep_connection() {
        let mut server = TestServer::start(None).await;
        let c = start(target(&server.url, "t", false, true));
        let mut conn = server.accept().await;
        conn.recv().await; // hello
        wait_connected(&c).await;
        conn.send_text("nope").await;
        conn.send(Message::Binary(vec![1u8, 2, 3].into())).await;
        conn.send_text(r#"{"type":"scan_request"}"#).await;
        conn.send_text(r#"{"type":"welcome"}"#).await;
        conn.send_text(r#"{"type":"scan_request","requestId":"x","timeoutMs":1000000000}"#)
            .await;
        wait_until("armed", Duration::from_secs(3), || c.slot.is_armed()).await;
        assert_eq!(state(&c), WsState::Connected);
        assert_eq!(server.attempts(), 1);
        assert_eq!(*c.events.armed.lock().unwrap(), vec!["x".to_string()]);

        // Capped at 300 s.
        let now = StdInstant::now();
        assert_eq!(c.slot.expire(now + Duration::from_secs(299)), None);
        assert_eq!(
            c.slot.expire(now + Duration::from_secs(300)),
            Some("x".to_string())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reconnects_after_server_drop() {
        let mut server = TestServer::start(None).await;
        let c = start(target(&server.url, "t", false, true));
        let mut conn = server.accept().await;
        conn.recv().await; // hello
        conn.send_text(r#"{"type":"scan_request","requestId":"abc"}"#)
            .await;
        wait_until("armed", Duration::from_secs(3), || c.slot.is_armed()).await;

        conn.close().await;
        wait_until("reconnecting", Duration::from_secs(3), || {
            matches!(state(&c), WsState::Reconnecting { .. })
        })
        .await;
        // The server can not be told on a dropped connection; the request is gone.
        assert!(!c.slot.is_armed());
        assert!(!c.handle.status().request_pending);

        let mut conn = server.accept().await;
        assert_eq!(conn.recv().await["type"], "hello");
        wait_connected(&c).await;
    }

    fn lost_count(c: &Client) -> usize {
        c.events
            .statuses
            .lock()
            .unwrap()
            .iter()
            .filter(|s| matches!(s.state, WsState::Reconnecting { lost: true, .. }))
            .count()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unstable_sessions_back_off_and_are_announced_once() {
        let mut server = TestServer::start(None).await;
        let timings = WsTimings {
            stable: Duration::from_millis(300),
            ..TEST_TIMINGS
        };
        let c = start_with(target(&server.url, "t", true, false), timings);

        // A server that accepts and closes at once: the delays keep growing (1 s, 2 s).
        let mut accepted = Vec::new();
        for _ in 0..3 {
            let conn = server.accept().await;
            accepted.push(StdInstant::now());
            if accepted.len() < 3 {
                conn.close().await;
                continue;
            }
            assert!(
                accepted[2] - accepted[1] >= Duration::from_millis(1500),
                "backoff was reset by an unstable session"
            );
            assert_eq!(lost_count(&c), 1);

            // A session that stays up resets the backoff and the announcement.
            wait_connected(&c).await;
            tokio::time::sleep(timings.stable * 2).await;
            conn.close().await;
        }
        let closed = StdInstant::now();
        let _conn = server.accept().await;
        assert!(closed.elapsed() < Duration::from_millis(2500));
        assert_eq!(lost_count(&c), 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn silent_peer_is_dropped_and_reconnected() {
        let mut server = TestServer::start(None).await;
        let timings = WsTimings {
            ping: Duration::from_millis(100),
            grace: Duration::from_millis(100),
            ..TEST_TIMINGS
        };
        let c = start_with(target(&server.url, "t", true, false), timings);
        // Never read again, so pings stay unanswered.
        let mut silent = server.accept().await;
        silent.recv().await; // hello
        wait_until("dropped", Duration::from_secs(3), || {
            matches!(state(&c), WsState::Reconnecting { .. })
        })
        .await;
        let WsState::Reconnecting { error, lost } = state(&c) else {
            unreachable!()
        };
        let origin = server.url.trim_end_matches("/ws");
        assert_eq!(
            error,
            DeliveryError::Unreachable {
                message: format!("no response ({origin})")
            }
        );
        assert!(lost);

        let mut conn = server.accept().await;
        assert_eq!(conn.recv().await["type"], "hello");
        wait_connected(&c).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn answered_pings_keep_the_session() {
        let mut server = TestServer::start(None).await;
        let timings = WsTimings {
            ping: Duration::from_millis(100),
            grace: Duration::from_millis(100),
            ..TEST_TIMINGS
        };
        let c = start_with(target(&server.url, "t", true, false), timings);
        let mut conn = server.accept().await;
        conn.recv().await; // hello

        // Reading lets the server answer every ping with a pong.
        conn.expect_silence(Duration::from_millis(800)).await;
        assert_eq!(state(&c), WsState::Connected);
        assert_eq!(server.attempts(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pings_while_connected() {
        let mut server = TestServer::start(None).await;
        let timings = WsTimings {
            ping: Duration::from_millis(200),
            ..TEST_TIMINGS
        };
        let _c = start_with(target(&server.url, "t", true, false), timings);
        let mut conn = server.accept().await;
        conn.recv().await; // hello
        conn.expect_ping().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn off_means_no_connection() {
        let server = TestServer::start(None).await;
        let c = start(target(&server.url, "t", false, false));
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(server.attempts(), 0);
        assert_eq!(state(&c), WsState::Off);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn not_connected_scans_report_not_connected() {
        let server = TestServer::start(Some(401)).await;
        let c = start(target(&server.url, "t", true, false));
        wait_until("token rejected", Duration::from_secs(3), || {
            state(&c) == WsState::TokenRejected
        })
        .await;
        c.handle.send_scan(&record(None), "Reception");
        wait_until("delivery", Duration::from_secs(3), || {
            !c.events.deliveries.lock().unwrap().is_empty()
        })
        .await;
        let d = c.events.deliveries.lock().unwrap()[0].clone();
        assert!(!d.ok);
        assert_eq!(d.error, Some(DeliveryError::NotConnected));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn mode_change_resends_hello_and_cancels_request() {
        let mut server = TestServer::start(None).await;
        let c = start(target(&server.url, "t", true, true));
        let mut conn = server.accept().await;
        conn.recv().await; // hello
        conn.send_text(r#"{"type":"scan_request","requestId":"abc"}"#)
            .await;
        wait_until("armed", Duration::from_secs(3), || c.slot.is_armed()).await;

        c.handle
            .update(settings(target(&server.url, "t", true, false)));
        assert_eq!(
            conn.recv().await,
            serde_json::json!({"type": "scan_request_ended", "requestId": "abc", "reason": "cancelled"})
        );
        let hello = conn.recv().await;
        assert_eq!(hello["type"], "hello");
        assert_eq!(
            hello["modes"],
            serde_json::json!({"stream": true, "request": false})
        );
        assert!(!c.slot.is_armed());
        assert_eq!(server.attempts(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn token_change_reconnects_and_off_disconnects() {
        let mut server = TestServer::start(None).await;
        let c = start(target(&server.url, "old", true, false));
        let _first = server.accept().await;
        c.handle
            .update(settings(target(&server.url, "new", true, false)));
        let second = server.accept().await;
        assert_eq!(second.auth.as_deref(), Some("Bearer new"));
        wait_connected(&c).await;

        c.handle
            .update(settings(target(&server.url, "new", false, false)));
        wait_until("off", Duration::from_secs(3), || state(&c) == WsState::Off).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_ws_reports_handshake_outcome() {
        let mut server = TestServer::start(None).await;
        let t = target(&server.url, "t", false, false);
        test_ws(&t, "Reception", "1.2.3").await.unwrap();
        let mut conn = server.accept().await;
        assert_eq!(conn.recv().await["type"], "hello");

        let server = TestServer::start(Some(403)).await;
        let t = target(&server.url, "t", false, false);
        assert_eq!(
            test_ws(&t, "Reception", "1.2.3").await,
            Err(DeliveryError::TokenRejected { status: 403 })
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn closed_port_is_unreachable_without_leaking_token() {
        let port = {
            let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            l.local_addr().unwrap().port()
        };
        for scheme in ["ws", "wss"] {
            let t = target(
                &format!("{scheme}://127.0.0.1:{port}/ws?key=urlsecret"),
                "tok-123",
                true,
                false,
            );
            let got = test_ws(&t, "Reception", "1.2.3").await;
            let Err(DeliveryError::Unreachable { message }) = got else {
                panic!("expected Unreachable, got {got:?}");
            };
            assert_eq!(
                message,
                format!("connection refused ({scheme}://127.0.0.1:{port})")
            );
        }
    }

    #[test]
    fn status_serializes_flat() {
        let s = WsStatus {
            state: WsState::Reconnecting {
                error: DeliveryError::Timeout,
                lost: true,
            },
            request_pending: false,
        };
        assert_eq!(
            serde_json::to_value(&s).unwrap(),
            serde_json::json!({"state": "reconnecting", "error": {"kind": "timeout"}, "lost": true, "requestPending": false})
        );
        let s = WsStatus {
            state: WsState::TokenRejected,
            request_pending: true,
        };
        assert_eq!(
            serde_json::to_value(&s).unwrap(),
            serde_json::json!({"state": "token_rejected", "requestPending": true})
        );
    }

    #[test]
    fn jitter_is_in_unit_range() {
        for _ in 0..100 {
            let j = jitter();
            assert!((0.0..1.0).contains(&j), "{j}");
        }
    }
}
