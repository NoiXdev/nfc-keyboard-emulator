mod accessibility;
mod commands;
mod config;
mod csv_export;
mod formatter;
mod pipeline;
mod reader;
mod reader_pcsc;
mod scan;
mod state;
mod typer;
mod worker;

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

struct TauriSink {
    app: AppHandle,
    history: Arc<Mutex<VecDeque<ScanRecord>>>,
    readers: Arc<Mutex<Vec<String>>>,
    retention: usize,
}

impl WorkerSink for TauriSink {
    fn scan(&self, record: &ScanRecord) {
        {
            let mut hist = self.history.lock().unwrap();
            hist.push_front(record.clone());
            while hist.len() > self.retention {
                hist.pop_back();
            }
        }
        let _ = self.app.emit("scan", record);
    }
    fn readers_changed(&self, readers: &[String]) {
        *self.readers.lock().unwrap() = readers.to_vec();
        let _ = self.app.emit("readers-changed", readers);
    }
    fn status(&self, status: &ReaderStatus) {
        let _ = self.app.emit("reader-status", status);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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

            app.manage(app_state);

            // Tray
            let show = MenuItem::with_id(app, "show", "Anzeigen", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Beenden", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
            let tray = TrayIconBuilder::new().icon(tray_icon).menu(&menu);
            // macOS: Template-Icon passt sich Hell/Dunkel an (Shadowing statt `mut`,
            // damit es auf Nicht-macOS kein unused_mut gibt).
            #[cfg(target_os = "macos")]
            let tray = tray.icon_as_template(true);
            let _tray = tray
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // Worker-Thread
            let sink = TauriSink {
                app: handle.clone(),
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
