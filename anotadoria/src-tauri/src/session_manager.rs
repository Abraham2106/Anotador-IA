//! # session_manager
//!
//! Responsabilidad: orquestar el flujo completo entre audio, STT y LLM.

use crate::config::AppConfig;
use crate::audio_capture::{AudioCapture, AudioHandle};
use crate::waveform_analyzer;
use tauri::{AppHandle, Emitter};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct SessionState {
    pub audio_handle: Option<AudioHandle>,
    pub last_emit: Instant,
}

pub struct SessionManager {
    pub state: Arc<Mutex<SessionState>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(SessionState {
                audio_handle: None,
                last_emit: Instant::now(),
            })),
        }
    }

    /// Inicia la grabación y la emisión de datos de waveform.
    pub fn start_recording(&self, app_handle: AppHandle) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        
        if state.audio_handle.is_some() {
            return Err("Ya hay una sesión de grabación activa".into());
        }

        let state_clone = Arc::clone(&self.state);
        let app_handle_clone = app_handle.clone();

        let handle = AudioCapture::start(move |samples| {
            // Obtener instante actual para el throttle
            let now = Instant::now();
            
            // Intentamos bloquear el estado para verificar el throttle
            if let Ok(mut s) = state_clone.try_lock() {
                if s.last_emit.elapsed().as_millis() > 16 {
                    let frame = waveform_analyzer::analyze(&samples, 64);
                    let _ = app_handle_clone.emit("waveform_data", &frame);
                    s.last_emit = now;
                }
            }
        }).map_err(|e| e.to_string())?;

        state.audio_handle = Some(handle);
        let _ = app_handle.emit("session_status", "recording");

        Ok(())
    }

    /// Detiene la grabación.
    pub fn stop_recording(&self, app_handle: AppHandle) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        
        if let Some(handle) = state.audio_handle.take() {
            handle.stop();
            let _ = app_handle.emit("session_status", "idle");
            Ok(())
        } else {
            Err("No hay ninguna sesión activa para detener".into())
        }
    }
}

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
