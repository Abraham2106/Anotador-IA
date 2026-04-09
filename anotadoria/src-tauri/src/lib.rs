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

#[tauri::command]
fn get_config() -> Result<AppConfig, String> {
    config::load_config().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
