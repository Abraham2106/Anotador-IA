#![allow(dead_code)]
//! llm_agents.rs
//!
//! Responsabilidad: ejecutar los dos agentes LLM secuenciales al finalizar
//! una sesión de grabación. Toda comunicación LLM pasa por el proxy local.
//!
//! NO sabe qué modelo está corriendo internamente.
//! NO maneja rate limits (el proxy lo hace).
//! NO tiene API keys.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::config::AppConfig;

// ── Tipos del protocolo OpenAI-compatible ───────────────────────────────────

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Deserialize)]
struct ChatMessageResponse {
    content: String,
}

// ── Errores de dominio ───────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("El proxy LLM no está corriendo en {0}. Inicia gemini-proxy-balancer primero.")]
    ProxyUnreachable(String),
    #[error("El proxy retornó error {status}: {body}")]
    ProxyError { status: u16, body: String },
    #[error("Error de red: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Respuesta del proxy sin contenido")]
    EmptyResponse,
}

// ── Cliente LLM ─────────────────────────────────────────────────────────────

pub struct LlmAgents {
    client: Client,
    proxy_url: String,
    model: String,
}

impl LlmAgents {
    pub fn new(config: &AppConfig) -> Self {
        let timeout = config.llm_timeout_secs.unwrap_or(60);
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout))
            .build()
            .expect("No se pudo crear el cliente HTTP");

        LlmAgents {
            client,
            proxy_url: config.llm_proxy_url.clone(),
            model: config.llm_model.clone(),
        }
    }

    /// Llama al proxy con un system prompt y un user prompt.
    /// Devuelve el texto de la respuesta del modelo.
    async fn call(&self, system: &str, user: &str) -> Result<String, AgentError> {
        let body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage { role: "system".into(), content: system.into() },
                ChatMessage { role: "user".into(),   content: user.into() },
            ],
            temperature: 0.2,  // Bajo: queremos respuestas deterministas para limpieza/resumen
        };

        let response = self.client
            .post(&self.proxy_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    AgentError::ProxyUnreachable(self.proxy_url.clone())
                } else {
                    AgentError::Network(e)
                }
            })?;

        let status = response.status().as_u16();
        if status != 200 {
            let body_text = response.text().await.unwrap_or_default();
            return Err(AgentError::ProxyError { status, body: body_text });
        }

        let parsed: ChatResponse = response.json().await?;
        let content = parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or(AgentError::EmptyResponse)?;

        Ok(content)
    }

    /// Agente 1: corrige errores acústicos de la transcripción sin alterar el significado.
    pub async fn clean_transcript(&self, raw: &str) -> Result<String, AgentError> {
        self.call(
            crate::prompt_templates::CLEANER_SYSTEM,
            raw,
        ).await
    }

    /// Agente 2: genera el bloque ## Resumen con action items concisos.
    pub async fn generate_summary(&self, clean_text: &str) -> Result<String, AgentError> {
        self.call(
            crate::prompt_templates::SUMMARIZER_SYSTEM,
            clean_text,
        ).await
    }
}
