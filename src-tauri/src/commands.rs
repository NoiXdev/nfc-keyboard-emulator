use crate::config::{Config, FormatConfig};
use crate::csv_export::scans_to_csv;
use crate::formatter::format_uid;
use crate::integration::config::{normalize, ConfigError, HttpTarget, IntegrationConfig, WsTarget};
use crate::integration::protocol::DeliveryError;
use crate::integration::ws::WsStatus;
use crate::integration::{http, ws, IntegrationHub};
use crate::reader::WorkerCommand;
use crate::scan::ScanRecord;
use crate::state::{save_config, AppState};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::State;

fn send_worker(state: &AppState, cmd: WorkerCommand) {
    if let Some(tx) = state.worker_tx.lock().unwrap().as_ref() {
        let _ = tx.send(cmd);
    }
}

#[tauri::command]
pub fn get_config(state: State<AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn list_readers(state: State<AppState>) -> Vec<String> {
    state.readers.lock().unwrap().clone()
}

#[tauri::command]
pub fn rescan_readers(state: State<AppState>) {
    send_worker(&state, WorkerCommand::Rescan);
}

#[tauri::command]
pub fn select_reader(app: tauri::AppHandle, state: State<AppState>, name: Option<String>) {
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.selected_reader = name.clone();
        save_config(&app, &cfg);
    }
    send_worker(&state, WorkerCommand::SelectReader(name));
}

#[tauri::command]
pub fn set_typing_enabled(app: tauri::AppHandle, state: State<AppState>, enabled: bool) {
    state.typing_enabled.store(enabled, Ordering::Relaxed);
    let mut cfg = state.config.lock().unwrap();
    cfg.typing_enabled_on_start = enabled;
    save_config(&app, &cfg);
}

#[tauri::command]
pub fn update_format(app: tauri::AppHandle, state: State<AppState>, format: FormatConfig) {
    let mut cfg = state.config.lock().unwrap();
    cfg.format = format;
    save_config(&app, &cfg);
}

#[tauri::command]
pub fn format_preview(format: FormatConfig) -> String {
    let f = format_uid(&[0x04, 0xA1, 0xB2, 0xC3], &format);
    let suffix = match format.suffix {
        crate::config::Suffix::Enter => "⏎",
        crate::config::Suffix::Tab => "⇥",
        crate::config::Suffix::None => "",
    };
    format!("{}{}", f.body, suffix)
}

#[tauri::command]
pub fn set_start_minimized(app: tauri::AppHandle, state: State<AppState>, value: bool) {
    let mut cfg = state.config.lock().unwrap();
    cfg.start_minimized = value;
    save_config(&app, &cfg);
}

#[tauri::command]
pub fn export_log_csv(app: tauri::AppHandle, state: State<AppState>) -> bool {
    use tauri_plugin_dialog::DialogExt;
    let history: Vec<ScanRecord> = state.history.lock().unwrap().iter().cloned().collect();
    let csv = scans_to_csv(&history);
    // Rust-seitiger Speichern-Dialog + Write: braucht keine fs-Capability/Scope.
    let Some(path) = app
        .dialog()
        .file()
        .add_filter("CSV", &["csv"])
        .set_file_name("nfc-scans.csv")
        .blocking_save_file()
    else {
        return false;
    };
    match path.into_path() {
        Ok(p) => std::fs::write(p, csv).is_ok(),
        Err(_) => false,
    }
}

#[tauri::command]
pub fn check_accessibility() -> bool {
    crate::accessibility::is_trusted()
}

#[tauri::command]
pub fn open_accessibility_settings() {
    crate::accessibility::prompt_or_open();
}

#[tauri::command]
pub fn update_integration(
    app: tauri::AppHandle,
    state: State<AppState>,
    hub: State<Arc<IntegrationHub>>,
    integration: IntegrationConfig,
) -> Result<Config, ConfigError> {
    let integration = normalize(integration)?;
    let saved = {
        let mut cfg = state.config.lock().unwrap();
        cfg.integration = integration;
        save_config(&app, &cfg);
        cfg.clone()
    };
    hub.settings_changed();
    Ok(saved)
}

/// The device name a test announces: the form value, or the host name when it is empty.
fn test_device(device_name: String) -> String {
    IntegrationConfig {
        device_name: device_name.trim().to_string(),
        ..Default::default()
    }
    .effective_device_name()
}

/// Tests the values currently in the form, not the saved settings, and works while the
/// mode is off.
#[tauri::command]
pub async fn test_http(
    hub: State<'_, Arc<IntegrationHub>>,
    http: HttpTarget,
    device_name: String,
) -> Result<(), DeliveryError> {
    let target = HttpTarget {
        url: http.url.trim().to_string(),
        token: http.token.trim().to_string(),
        ..http
    };
    http::test_http(hub.http(), &target, &test_device(device_name)).await
}

#[tauri::command]
pub async fn test_ws(
    app: tauri::AppHandle,
    ws: WsTarget,
    device_name: String,
) -> Result<(), DeliveryError> {
    let target = WsTarget {
        url: ws.url.trim().to_string(),
        token: ws.token.trim().to_string(),
        ..ws
    };
    ws::test_ws(
        &target,
        &test_device(device_name),
        &app.package_info().version.to_string(),
    )
    .await
}

#[tauri::command]
pub fn ws_reconnect(hub: State<Arc<IntegrationHub>>) {
    hub.ws().reconnect();
}

#[tauri::command]
pub fn get_ws_status(hub: State<Arc<IntegrationHub>>) -> WsStatus {
    hub.ws().status()
}
