mod accessibility;
mod commands;
mod config;
mod csv_export;
mod formatter;
mod integration;
mod pipeline;
mod reader;
mod reader_pcsc;
mod scan;
mod state;
mod typer;
mod worker;

use crate::config::Config;
use crate::integration::protocol::DeliveryResult;
use crate::integration::route;
use crate::integration::ws::{WsEvents, WsStatus};
use crate::integration::{HubEvents, IntegrationHub};
use crate::reader::{ReaderBackend, ReaderStatus, WorkerCommand};
use crate::reader_pcsc::PcscReader;
use crate::scan::ScanRecord;
use crate::state::{load_config, AppState};
use crate::typer::{EnigoTyper, NullTyper, Typer};
use crate::worker::{run_worker, WorkerDeps, WorkerSink};
use std::collections::VecDeque;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};

/// Forwards integration events to the frontend. Payloads never contain tokens.
struct TauriEvents {
    app: AppHandle,
}

impl WsEvents for TauriEvents {
    fn status(&self, s: &WsStatus) {
        let _ = self.app.emit("ws-status", s);
    }
    fn delivery(&self, r: &DeliveryResult) {
        let _ = self.app.emit("scan-delivery", r);
    }
    fn request_armed(&self, request_id: &str) {
        let _ = self
            .app
            .emit("ws-request", serde_json::json!({ "requestId": request_id }));
    }
}

impl HubEvents for TauriEvents {}

struct TauriSink {
    app: AppHandle,
    config: Arc<Mutex<Config>>,
    hub: Arc<IntegrationHub>,
    history: Arc<Mutex<VecDeque<ScanRecord>>>,
    readers: Arc<Mutex<Vec<String>>>,
    retention: usize,
}

impl WorkerSink for TauriSink {
    fn scan(&self, record: &ScanRecord) {
        let mut record = record.clone();
        record.delivery_targets = route(&self.config.lock().unwrap().integration, &record);
        {
            let mut hist = self.history.lock().unwrap();
            hist.push_front(record.clone());
            while hist.len() > self.retention {
                hist.pop_back();
            }
        }
        let _ = self.app.emit("scan", &record);
        // Only enqueues work: HTTP runs on the async runtime, the WebSocket task owns its socket.
        self.hub.dispatch(&record);
    }
    fn readers_changed(&self, readers: &[String]) {
        *self.readers.lock().unwrap() = readers.to_vec();
        let _ = self.app.emit("readers-changed", readers);
    }
    fn status(&self, status: &ReaderStatus) {
        let _ = self.app.emit("reader-status", status);
    }
}

fn focus_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must stay first: the plugin exits a second instance from its own setup
        // hook, which has to happen before ours claims the PC/SC reader.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            focus_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let handle = app.handle().clone();
            let config = load_config(&handle);
            let retention = config.log_retention.max(1);
            let app_state = AppState::new(config.clone());

            let (tx, ctrl_rx) = mpsc::channel::<WorkerCommand>();
            *app_state.worker_tx.lock().unwrap() = Some(tx);

            let history = app_state.history.clone();
            let readers = app_state.readers.clone();
            let shared_config = app_state.config.clone();
            let typing_enabled = app_state.typing_enabled.clone();
            let requests = app_state.requests.clone();

            app.manage(app_state);

            let hub = Arc::new(IntegrationHub::new(
                shared_config.clone(),
                requests.clone(),
                Arc::new(TauriEvents {
                    app: handle.clone(),
                }),
                app.package_info().version.to_string(),
            ));
            app.manage(hub.clone());

            // Tray
            let show = MenuItem::with_id(app, "show", "Anzeigen", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Beenden", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            // macOS draws the tray icon as a template image (see below), which uses
            // only the alpha channel - so it needs a transparent glyph rather than the
            // opaque tile the Windows tray expects.
            #[cfg(target_os = "macos")]
            let tray_icon =
                tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))?;
            #[cfg(not(target_os = "macos"))]
            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
            let tray = TrayIconBuilder::new().icon(tray_icon).menu(&menu);
            // macOS: Template-Icon passt sich Hell/Dunkel an (Shadowing statt `mut`,
            // damit es auf Nicht-macOS kein unused_mut gibt).
            #[cfg(target_os = "macos")]
            let tray = tray.icon_as_template(true);
            let _tray = tray
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => focus_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // Worker-Thread
            let sink = TauriSink {
                app: handle.clone(),
                config: shared_config.clone(),
                hub,
                history,
                readers,
                retention,
            };
            let typer: Box<dyn Typer> = match EnigoTyper::new() {
                Ok(t) => Box::new(t),
                Err(_) => Box::new(NullTyper),
            };
            let backend: Box<dyn ReaderBackend> = match PcscReader::new() {
                Ok(b) => Box::new(b),
                Err(e) => {
                    eprintln!("PC/SC nicht verfügbar: {e}");
                    return Ok(());
                }
            };

            std::thread::spawn(move || {
                run_worker(WorkerDeps {
                    backend,
                    sink: Box::new(sink),
                    typer,
                    config: shared_config,
                    typing_enabled,
                    requests,
                    ctrl_rx,
                });
            });

            // Start-minimized
            if config.start_minimized {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::list_readers,
            commands::rescan_readers,
            commands::select_reader,
            commands::set_typing_enabled,
            commands::update_format,
            commands::format_preview,
            commands::set_start_minimized,
            commands::export_log_csv,
            commands::check_accessibility,
            commands::open_accessibility_settings,
            commands::update_integration,
            commands::test_http,
            commands::test_ws,
            commands::ws_reconnect,
            commands::get_ws_status,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        // macOS: a dock click must bring the tray-hidden window back, otherwise
        // the app looks dead and users start it a second time. Underscore names
        // keep the closure warning-free on non-macOS.
        .run(|_app, _event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } = _event
            {
                focus_main_window(_app);
            }
        });
}
