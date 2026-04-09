#![allow(unused)]
mod config;
mod session_manager;
mod audio_capture;
mod waveform_analyzer;
mod stt_client;
mod disfluency_filter;
mod llm_agents;
mod prompt_templates;
mod vault_writer;
mod template_builder;

use config::AppConfig;
use session_manager::SessionManager;
use tauri::{AppHandle, State};

#[tauri::command]
fn get_config() -> Result<AppConfig, String> {
    config::load_config().map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_recording(app: AppHandle, manager: State<'_, SessionManager>) -> Result<(), String> {
    manager.start_recording(app).await
}

#[tauri::command]
async fn stop_recording(app: AppHandle, manager: State<'_, SessionManager>) -> Result<(), String> {
    manager.stop_recording(app).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(SessionManager::new()) // Proveer el manager como estado global
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            start_recording,
            stop_recording
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
