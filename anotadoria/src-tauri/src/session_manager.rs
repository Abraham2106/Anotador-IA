//! # session_manager
//!
//! Responsabilidad: orquestar el flujo completo entre audio, STT y LLM.

use crate::config::AppConfig;

pub struct SessionManager;

/// Verifica si el proxy LLM está vivo antes de iniciar una sesión.
pub async fn check_llm_proxy(config: &AppConfig) -> Result<(), String> {
    let health_url = config.llm_proxy_url
        .replace("/v1/chat/completions", "/health");

    let client = reqwest::Client::new();
    match client.get(&health_url).timeout(std::time::Duration::from_secs(3)).send().await {
        Ok(r) if r.status().is_success() => Ok(()),
        Ok(r) => Err(format!("Proxy responde pero con error: {}", r.status())),
        Err(_) => Err(format!(
            "No se puede conectar al proxy LLM en {}.\n\
             Inicia gemini-proxy-balancer con: uvicorn main:app --port 8000",
            health_url
        )),
    }
}
