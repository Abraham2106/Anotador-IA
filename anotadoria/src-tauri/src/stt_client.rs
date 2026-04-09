//! # stt_client
//!
//! Responsabilidad: Cliente WebSocket para Deepgram Nova-3.
//! Maneja la conexión, el envío de audio y la recepción de transcripciones.

use crate::config::AppConfig;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async,
    tungstenite::protocol::Message,
};
use url::Url;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SttData {
    pub text: String,
    pub is_final: bool,
}

#[derive(Deserialize)]
struct DeepgramResponse {
    channel: DeepgramChannel,
    is_final: bool,
}

#[derive(Deserialize)]
struct DeepgramChannel {
    alternatives: Vec<DeepgramAlternative>,
}

#[derive(Deserialize)]
struct DeepgramAlternative {
    transcript: String,
}

pub struct SttClient {
    tx: mpsc::Sender<Vec<u8>>,
}

impl SttClient {
    /// Inicia el cliente STT. Devuelve un Sender para enviar chunks de audio.
    pub fn start(config: &AppConfig, app_handle: AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let api_key = config.deepgram_api_key.clone();
        let lang = config.language.clone();
        let model = config.model_stt.clone();

        // Construcción de la URL de Deepgram
        let url_str = format!(
            "wss://api.deepgram.com/v1/listen?model={}&language={}&smart_format=true&interim_results=true&encoding=linear16&sample_rate=16000",
            model, lang
        );
        let url = Url::parse(&url_str)?;

        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(100);

        // Hilo asíncrono para el WebSocket
        tokio::spawn(async move {
            let request = http::Request::builder()
                .uri(url.as_str())
                .header("Authorization", format!("Token {}", api_key))
                .body(())
                .unwrap();

            let (ws_stream, _) = match connect_async(request).await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Error al conectar con Deepgram STT: {}", e);
                    return;
                }
            };

            let (mut write, mut read) = ws_stream.split();

            // Task 1: Enviar audio
            let send_task = tokio::spawn(async move {
                while let Some(audio) = rx.recv().await {
                    if write.send(Message::Binary(audio.into())).await.is_err() {
                        break;
                    }
                }
                // Enviar mensaje de cierre (Empty Binary Message)
                let _ = write.send(Message::Binary(vec![].into())).await;
            });

            // Task 2: Recibir transcripciones
            let app_handle_clone = app_handle.clone();
            let receive_task = tokio::spawn(async move {
                while let Some(msg) = read.next().await {
                    if let Ok(Message::Text(text)) = msg {
                        if let Ok(resp) = serde_json::from_str::<DeepgramResponse>(&text) {
                            if let Some(alt) = resp.channel.alternatives.get(0) {
                                if !alt.transcript.is_empty() {
                                    let data = SttData {
                                        text: alt.transcript.clone(),
                                        is_final: resp.is_final,
                                    };
                                    let _ = app_handle_clone.emit("stt_data", &data);
                                }
                            }
                        }
                    }
                }
            });

            let _ = tokio::join!(send_task, receive_task);
        });

        Ok(SttClient { tx })
    }

    pub fn clone_tx(&self) -> mpsc::Sender<Vec<u8>> {
        self.tx.clone()
    }
}
