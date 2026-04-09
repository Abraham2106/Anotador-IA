//! # config
//!
//! Responsabilidad: deserializar el archivo config.toml en una struct tipada.
//! NO hace: validación de paths, conexiones de red, lógica de negocio.

use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    pub vault_path: String,
    pub deepgram_api_key: String,
    pub language: String,
    pub model_stt: String,

    // LLM via proxy — sin API key en la app
    pub llm_proxy_url: String,
    pub llm_model: String,
    pub llm_timeout_secs: Option<u64>,

    pub note_subfolder: Option<String>,
}

pub fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    // Intentar en CWD (raiz del proyecto si se corre con npm run tauri dev)
    let mut path = std::path::PathBuf::from("config.toml");
    
    if !path.exists() {
        // Intentar en el padre (si el CWD es src-tauri)
        path = std::path::PathBuf::from("../config.toml");
    }

    if !path.exists() {
        return Err(format!("No se encontró config.toml en {:?} ni en el directorio superior. CWD: {:?}", 
            std::env::current_dir()?.join("config.toml"),
            std::env::current_dir()?
        ).into());
    }

    let content = fs::read_to_string(path)?;
    let config: AppConfig = toml::from_str(&content)?;
    Ok(config)
}
