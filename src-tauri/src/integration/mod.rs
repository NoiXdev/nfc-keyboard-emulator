pub mod auth;
pub mod backoff;
pub mod config;
pub mod config_file;
pub mod http;
pub mod protocol;
pub mod requests;
pub mod template;
pub mod ws;

#[cfg(test)]
mod test_server;

use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::scan::{DeliveryTarget, ScanRecord};
use config::IntegrationConfig;
use protocol::{scan_payload, ClientMessage, DeliveryResult, PROTOCOL_VERSION};
use requests::RequestSlot;
use ws::{spawn_ws, WsEvents, WsHandle, WsSettings};

/// Everything the hub reports to the frontend; the WebSocket events are a supertrait.
pub trait HubEvents: WsEvents {}

/// Which targets a scan is delivered to. Nothing leaves the machine unless a mode is on,
/// and read errors never carry a UID.
pub fn route(cfg: &IntegrationConfig, rec: &ScanRecord) -> Vec<DeliveryTarget> {
    let mut targets = Vec::new();
    if rec.uid_hex.is_empty() {
        return targets;
    }
    if cfg.http.enabled && !cfg.http.url.is_empty() {
        targets.push(DeliveryTarget::Http);
    }
    if cfg.ws.active() && (cfg.ws.stream || rec.request_id.is_some()) {
        targets.push(DeliveryTarget::Ws);
    }
    targets
}

/// Delivers scans to the configured server targets without ever blocking the caller.
pub struct IntegrationHub {
    config: Arc<Mutex<Config>>,
    events: Arc<dyn HubEvents>,
    http: reqwest::Client,
    ws: WsHandle,
    app_version: String,
}

impl IntegrationHub {
    pub fn new(
        config: Arc<Mutex<Config>>,
        slot: Arc<RequestSlot>,
        events: Arc<dyn HubEvents>,
        app_version: String,
    ) -> Self {
        let integration = config.lock().unwrap().integration.clone();
        let ws = spawn_ws(
            WsSettings {
                device: integration.effective_device_name(),
                target: integration.ws,
                app_version: app_version.clone(),
            },
            slot,
            events.clone(),
        );
        Self {
            config,
            events,
            http: http::http_client(&app_version),
            ws,
            app_version,
        }
    }

    /// Hands the scan to its delivery targets (`rec.delivery_targets`, set by `route`).
    /// `integration` must be the snapshot `route` used, so a settings change in between
    /// can not send the scan somewhere else. HTTP runs on the async runtime and WebSocket
    /// sends are only enqueued.
    pub fn dispatch(&self, rec: &ScanRecord, integration: &IntegrationConfig) {
        // A scan answering a request always goes to the WebSocket task: it sends the
        // response and publishes that the request is no longer pending.
        let answers_request = rec.request_id.as_deref().is_some_and(|id| !id.is_empty());
        let to_http = rec.delivery_targets.contains(&DeliveryTarget::Http);
        let to_ws = answers_request || rec.delivery_targets.contains(&DeliveryTarget::Ws);
        if !to_http && !to_ws {
            return;
        }

        let device = integration.effective_device_name();
        if to_ws {
            self.ws.send_scan(rec, &device);
        }
        if to_http {
            let client = self.http.clone();
            let events = self.events.clone();
            let target = integration.http.clone();
            let scan_id = rec.scan_id.clone();
            let msg = ClientMessage::Scan {
                protocol_version: PROTOCOL_VERSION,
                payload: scan_payload(rec, &device),
            };
            tauri::async_runtime::spawn(async move {
                let result = http::post_message(&client, &target, &msg).await;
                events.delivery(&DeliveryResult {
                    scan_id,
                    target: DeliveryTarget::Http,
                    ok: result.is_ok(),
                    error: result.err(),
                });
            });
        }
    }

    /// Applies the current settings to the WebSocket connection.
    pub fn settings_changed(&self) {
        let integration = self.config.lock().unwrap().integration.clone();
        self.ws.update(WsSettings {
            device: integration.effective_device_name(),
            target: integration.ws,
            app_version: self.app_version.clone(),
        });
    }

    pub fn ws(&self) -> &WsHandle {
        &self.ws
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::ScanStatus;
    use config::{HttpTarget, WsTarget};
    use protocol::DeliveryError;
    use ws::WsStatus;

    fn record(uid_hex: &str, request_id: Option<&str>) -> ScanRecord {
        ScanRecord {
            timestamp: "2026-10-28T13:45:21.123Z".into(),
            reader: "ACS ACR122U PICC Interface".into(),
            uid_hex: uid_hex.into(),
            uid_length: uid_hex.len() / 2,
            typed: true,
            typed_string: format!("{uid_hex}\n"),
            status: ScanStatus::Ok,
            scan_id: "scan-1".into(),
            uid_formatted: uid_hex.into(),
            request_id: request_id.map(str::to_string),
            delivery_targets: vec![],
        }
    }

    fn ok_record() -> ScanRecord {
        record("04A2B3C4", None)
    }

    fn http(enabled: bool, url: &str) -> HttpTarget {
        HttpTarget {
            enabled,
            url: url.into(),
            token: String::new(),
        }
    }

    fn ws(url: &str, stream: bool, request: bool) -> WsTarget {
        WsTarget {
            url: url.into(),
            token: String::new(),
            stream,
            request,
        }
    }

    fn all_on() -> IntegrationConfig {
        IntegrationConfig {
            http: http(true, "http://x.test/api"),
            ws: ws("ws://x.test/ws", true, true),
            ..Default::default()
        }
    }

    #[test]
    fn all_off_routes_nowhere() {
        let cfg = IntegrationConfig::default();
        assert!(route(&cfg, &ok_record()).is_empty());
        assert!(route(&cfg, &record("04A2B3C4", Some("req-1"))).is_empty());
    }

    #[test]
    fn read_errors_route_nowhere() {
        assert!(route(&all_on(), &record("", None)).is_empty());
        assert!(route(&all_on(), &record("", Some("req-1"))).is_empty());
    }

    #[test]
    fn http_needs_enabled_and_url() {
        let with = |http| IntegrationConfig {
            http,
            ..Default::default()
        };
        let rec = ok_record();
        assert_eq!(
            route(&with(http(true, "http://x.test")), &rec),
            vec![DeliveryTarget::Http]
        );
        assert!(route(&with(http(false, "http://x.test")), &rec).is_empty());
        assert!(route(&with(http(true, "")), &rec).is_empty());
    }

    #[test]
    fn ws_stream_routes_plain_scans() {
        let cfg = IntegrationConfig {
            ws: ws("ws://x.test/ws", true, false),
            ..Default::default()
        };
        assert_eq!(route(&cfg, &ok_record()), vec![DeliveryTarget::Ws]);
        // Stream without a URL is not active.
        let no_url = IntegrationConfig {
            ws: ws("", true, false),
            ..Default::default()
        };
        assert!(route(&no_url, &ok_record()).is_empty());
    }

    #[test]
    fn ws_request_only_routes_responses() {
        let cfg = IntegrationConfig {
            ws: ws("ws://x.test/ws", false, true),
            ..Default::default()
        };
        assert_eq!(
            route(&cfg, &record("04A2B3C4", Some("req-1"))),
            vec![DeliveryTarget::Ws]
        );
        assert!(route(&cfg, &ok_record()).is_empty());
    }

    #[test]
    fn both_targets_route_in_order() {
        assert_eq!(
            route(&all_on(), &ok_record()),
            vec![DeliveryTarget::Http, DeliveryTarget::Ws]
        );
    }

    #[derive(Default)]
    struct Recorder {
        deliveries: Mutex<Vec<DeliveryResult>>,
    }

    impl WsEvents for Recorder {
        fn status(&self, _: &WsStatus) {}
        fn delivery(&self, r: &DeliveryResult) {
            self.deliveries.lock().unwrap().push(r.clone());
        }
        fn request_armed(&self, _: &str) {}
    }

    impl HubEvents for Recorder {}

    /// A local port nothing listens on.
    fn closed_port_url() -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}/hook", listener.local_addr().unwrap())
    }

    fn wait_for_deliveries(events: &Recorder, count: usize) -> Vec<DeliveryResult> {
        for _ in 0..60 {
            {
                let got = events.deliveries.lock().unwrap();
                if got.len() >= count {
                    return got.clone();
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        events.deliveries.lock().unwrap().clone()
    }

    #[test]
    fn http_failure_is_reported_without_blocking_dispatch() {
        let mut config = Config::default();
        config.integration.http = http(true, &closed_port_url());
        let events = Arc::new(Recorder::default());
        let hub = IntegrationHub::new(
            Arc::new(Mutex::new(config.clone())),
            Arc::new(RequestSlot::default()),
            events.clone(),
            "1.2.3".into(),
        );

        let mut rec = ok_record();
        rec.delivery_targets = route(&config.integration, &rec);
        hub.dispatch(&rec, &config.integration);

        let got = wait_for_deliveries(&events, 1);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].scan_id, "scan-1");
        assert_eq!(got[0].target, DeliveryTarget::Http);
        assert!(!got[0].ok);
        assert!(matches!(
            got[0].error,
            Some(DeliveryError::Unreachable { .. })
        ));
    }

    #[test]
    fn dispatch_uses_the_routed_snapshot() {
        // The settings were changed (HTTP cleared) after the scan was routed.
        let events = Arc::new(Recorder::default());
        let hub = IntegrationHub::new(
            Arc::new(Mutex::new(Config::default())),
            Arc::new(RequestSlot::default()),
            events.clone(),
            "1.2.3".into(),
        );
        let snapshot = IntegrationConfig {
            http: http(true, &closed_port_url()),
            ..Default::default()
        };
        let mut rec = ok_record();
        rec.delivery_targets = route(&snapshot, &rec);
        hub.dispatch(&rec, &snapshot);

        let got = wait_for_deliveries(&events, 1);
        assert_eq!(got.len(), 1);
        // Sent to the snapshot's URL, not to the current (empty) one.
        assert!(matches!(
            got[0].error,
            Some(DeliveryError::Unreachable { .. })
        ));
    }

    #[test]
    fn dispatch_without_targets_delivers_nothing() {
        let events = Arc::new(Recorder::default());
        let hub = IntegrationHub::new(
            Arc::new(Mutex::new(Config::default())),
            Arc::new(RequestSlot::default()),
            events.clone(),
            "1.2.3".into(),
        );
        hub.dispatch(&ok_record(), &IntegrationConfig::default());
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(events.deliveries.lock().unwrap().is_empty());
    }
}
