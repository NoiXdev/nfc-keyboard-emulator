use crate::config::{Config, FormatConfig};
use crate::csv_export::scans_to_csv;
use crate::formatter::format_uid;
use crate::integration::config::{normalize, ConfigError, HttpTarget, IntegrationConfig, WsTarget};
use crate::integration::config_file::{
    self, parse_config_file, ImportError, ImportPreview, ImportSource, ServerConfigFile,
};
use crate::integration::protocol::DeliveryError;
use crate::integration::ws::WsStatus;
use crate::integration::{http, ws, IntegrationHub};
use crate::reader::WorkerCommand;
use crate::scan::ScanRecord;
use crate::state::{save_config, AppState};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, State};

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
    save_integration(&app, &state, &hub, integration)
}

/// The one save path for server settings: normalize, store, write, let the hub reconnect.
/// The disk write runs outside the config lock so the reader worker never waits on it;
/// sync commands run one at a time on the main thread, so writes cannot overtake.
fn save_integration(
    app: &AppHandle,
    state: &AppState,
    hub: &IntegrationHub,
    integration: IntegrationConfig,
) -> Result<Config, ConfigError> {
    let integration = normalize(integration)?;
    let saved = {
        let mut cfg = state.config.lock().unwrap();
        cfg.integration = integration;
        cfg.clone()
    };
    save_config(app, &saved);
    hub.settings_changed();
    Ok(saved)
}

/// Writes the current server settings, tokens included, to a file the user picks.
#[tauri::command]
pub fn export_server_config(app: AppHandle, state: State<AppState>) -> bool {
    use tauri_plugin_dialog::DialogExt;
    let file = config_file::export(&state.config.lock().unwrap().integration);
    let Ok(json) = serde_json::to_string_pretty(&file) else {
        return false;
    };
    let Some(path) = app
        .dialog()
        .file()
        .add_filter("JSON", &["json"])
        .set_file_name("nfc-server-config.json")
        .blocking_save_file()
    else {
        return false;
    };
    match path.into_path() {
        Ok(p) => std::fs::write(p, json).is_ok(),
        Err(_) => false,
    }
}

/// Lets the user pick a config file and stages it for confirmation. `None` when cancelled.
#[tauri::command]
pub fn import_server_config(
    app: AppHandle,
    state: State<AppState>,
) -> Result<Option<ImportPreview>, ImportError> {
    use tauri_plugin_dialog::DialogExt;
    let Some(path) = app
        .dialog()
        .file()
        .add_filter("JSON", &["json"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let unreadable = || ImportError::InvalidJson {
        message: "the file could not be read".to_string(),
    };
    let text = std::fs::read_to_string(path.into_path().map_err(|_| unreadable())?)
        .map_err(|_| unreadable())?;
    let file = parse_config_file(&text)?;
    Ok(Some(stage_import(&state, file, ImportSource::File)))
}

/// Stores `file` as the pending import, replacing any earlier one, and returns its preview.
pub fn stage_import(
    state: &AppState,
    file: ServerConfigFile,
    source: ImportSource,
) -> ImportPreview {
    let current = state.config.lock().unwrap().integration.clone();
    let preview = config_file::preview(&current, &file, source);
    *state.pending_import.lock().unwrap() = Some((preview.id.clone(), file, source));
    *state.pending_import_error.lock().unwrap() = None;
    preview
}

/// Keeps a link's error so a frontend that was not listening yet can still show it.
pub fn record_link_error(state: &AppState, error: ImportError) {
    *state.pending_import_error.lock().unwrap() = Some(error);
}

fn take_import_error(state: &AppState) -> Option<ImportError> {
    state.pending_import_error.lock().unwrap().take()
}

/// The error of the last config link, if no import has replaced it. Taking clears it.
#[tauri::command]
pub fn take_pending_import_error(state: State<AppState>) -> Option<ImportError> {
    take_import_error(&state)
}

/// Takes the pending import if `id` matches it; a wrong id leaves it in place.
fn take_pending(state: &AppState, id: &str) -> Option<ServerConfigFile> {
    let mut pending = state.pending_import.lock().unwrap();
    match pending.as_ref() {
        Some((pending_id, _, _)) if pending_id == id => pending.take().map(|(_, f, _)| f),
        _ => None,
    }
}

/// The preview of the import still waiting for confirmation, recomputed against the
/// current settings. Lets the frontend catch a link that arrived before it listened.
#[tauri::command]
pub fn get_pending_import(state: State<AppState>) -> Option<ImportPreview> {
    let (id, file, source) = state.pending_import.lock().unwrap().clone()?;
    let current = state.config.lock().unwrap().integration.clone();
    Some(ImportPreview {
        id,
        ..config_file::preview(&current, &file, source)
    })
}

/// Applies the confirmed import through the same save path as `update_integration`.
#[tauri::command]
pub fn apply_import(
    app: AppHandle,
    state: State<AppState>,
    hub: State<Arc<IntegrationHub>>,
    id: String,
) -> Result<Config, ImportError> {
    let file = take_pending(&state, &id).ok_or(ImportError::InvalidLink)?;
    let current = state.config.lock().unwrap().integration.clone();
    save_integration(&app, &state, &hub, config_file::apply(&current, &file)).map_err(|e| match e {
        ConfigError::InvalidHttpUrl => ImportError::InvalidHttpUrl,
        ConfigError::InvalidWsUrl => ImportError::InvalidWsUrl,
    })
}

#[tauri::command]
pub fn discard_import(state: State<AppState>, id: String) {
    take_pending(&state, &id);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::config_file::{parse_config_file, ImportSource};

    #[test]
    fn apply_pending_requires_matching_id() {
        let state = AppState::new(Config::default());
        let file =
            parse_config_file(r#"{"format":"nfc-keyboard-emulator/server-config","version":1}"#)
                .unwrap();
        *state.pending_import.lock().unwrap() =
            Some(("id-1".to_string(), file.clone(), ImportSource::Link));

        assert_eq!(take_pending(&state, "other"), None);
        assert!(state.pending_import.lock().unwrap().is_some());

        assert_eq!(take_pending(&state, "id-1"), Some(file));
        assert!(state.pending_import.lock().unwrap().is_none());
        assert_eq!(take_pending(&state, "id-1"), None);
    }

    #[test]
    fn link_error_is_kept_until_taken_or_replaced_by_an_import() {
        let state = AppState::new(Config::default());
        assert_eq!(take_import_error(&state), None);

        record_link_error(&state, ImportError::InvalidLink);
        record_link_error(&state, ImportError::WrongFormat);
        assert_eq!(take_import_error(&state), Some(ImportError::WrongFormat));
        assert_eq!(take_import_error(&state), None);

        record_link_error(&state, ImportError::InvalidLink);
        let file =
            parse_config_file(r#"{"format":"nfc-keyboard-emulator/server-config","version":1}"#)
                .unwrap();
        stage_import(&state, file, ImportSource::File);
        assert_eq!(take_import_error(&state), None);
    }
}
