//! # session_manager
//!
//! Responsabilidad: orquestar el flujo completo entre audio, STT y LLM.

use crate::config::{AppConfig, load_config};
use crate::audio_capture::{AudioCapture, AudioHandle};
use crate::stt_client::SttClient;
use crate::waveform_analyzer;
use tauri::{AppHandle, Emitter};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct SessionState {
    pub audio_handle: Option<AudioHandle>,
    pub last_emit: Instant,
    pub stt_client: Option<SttClient>,
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
                stt_client: None,
            })),
        }
    }

    /// Inicia la grabación y la emisión de datos de waveform y STT.
    pub async fn start_recording(&self, app_handle: AppHandle) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        
        if state.audio_handle.is_some() {
            return Err("Ya hay una sesión de grabación activa".into());
        }

        // Cargar config fresca
        let config = load_config().map_err(|e| e.to_string())?;

        let state_clone = Arc::clone(&self.state);
        let app_handle_clone = app_handle.clone();
        
        // 1. Iniciar AudioCapture PRIMERO para obtener el sample_rate y canales reales
        let (handle, sample_rate, channels) = AudioCapture::start({
            let app_handle_inner = app_handle.clone();
            let state_inner = Arc::clone(&self.state);
            move |samples| {
                let now = Instant::now();
                if let Ok(mut s) = state_inner.try_lock() {
                    // Pipeline de Waveform
                    if s.last_emit.elapsed().as_millis() > 16 {
                        let frame = waveform_analyzer::analyze(&samples, 64);
                        let _ = app_handle_inner.emit("waveform_data", &frame);
                        s.last_emit = now;
                    }
                    // Pipeline de STT
                    if let Some(stt) = &s.stt_client {
                        let pcm_data = f32_to_i16_pcm(&samples);
                        let _ = stt.clone_tx().try_send(pcm_data);
                    }
                }
            }
        }).map_err(|e| e.to_string())?;

        // 2. Iniciar STT Client con parámetros reales
        let stt_client = SttClient::start(&config, app_handle.clone(), sample_rate, channels)
            .map_err(|e| format!("Error iniciando STT: {}", e))?;

        state.audio_handle = Some(handle);
        state.stt_client = Some(stt_client);
        let _ = app_handle.emit("session_status", "recording");

        Ok(())
    }

    /// Detiene la grabación.
    pub async fn stop_recording(&self, app_handle: AppHandle) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        
        if let Some(handle) = state.audio_handle.take() {
            handle.stop();
            state.stt_client = None; // Al soltar el cliente se cierra el canal y el websocket
            let _ = app_handle.emit("session_status", "idle");
            Ok(())
        } else {
            Err("No hay ninguna sesión activa para detener".into())
        }
    }
}

/// Conversión simple de samples f32 (-1.0 a 1.0) a i16 para PCM Linear16.
fn f32_to_i16_pcm(samples: &[f32]) -> Vec<u8> {
    let mut pcm = Vec::with_capacity(samples.len() * 2);
    for &sample in samples {
        let s = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        pcm.extend_from_slice(&s.to_le_bytes());
    }
    pcm
}

// Re-exportar check_llm_proxy si es necesario
pub async fn check_llm_proxy(config: &AppConfig) -> Result<(), String> {
    let health_url = config.llm_proxy_url.replace("/v1/chat/completions", "/health");
    let client = reqwest::Client::new();
    match client.get(&health_url).timeout(std::time::Duration::from_secs(3)).send().await {
        Ok(r) if r.status().is_success() => Ok(()),
        _ => Err("Proxy LLM no disponible".into()),
    }
}
