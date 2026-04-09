//! # stt_client
//!
//! Responsabilidad: Cliente WebSocket para Deepgram Nova-3.
//! Maneja la conexión, el envío de audio y la recepción de transcripciones.

use crate::config::AppConfig;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async_tls_with_config,
    tungstenite::client::IntoClientRequest,
    tungstenite::protocol::Message,
    Connector,
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
    channel: Option<DeepgramChannel>,
    #[serde(default)]
    is_final: bool,
    // Soporte para formato plano (modelos Flux / V2)
    transcript: Option<String>,
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
    pub fn start(config: &AppConfig, app_handle: AppHandle, sample_rate: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let api_key = config.deepgram_api_key.clone();
        let lang = config.language.clone();
        let model = config.model_stt.clone();

        // Construcción robusta de la URL de Deepgram
        let mut url = Url::parse("wss://api.deepgram.com/v1/listen")?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("model", &model);
            
            // Solo añadir lenguaje si no es un modelo específico de inglés (como flux-general-en)
            if !model.ends_with("-en") {
                query.append_pair("language", &lang);
            }
            
            query.append_pair("smart_format", "true");
            query.append_pair("interim_results", "true");
            query.append_pair("encoding", "linear16");
            query.append_pair("sample_rate", &sample_rate.to_string());
        }

        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(100);

        // Hilo asíncrono para el WebSocket
        tauri::async_runtime::spawn(async move {
            // Usar IntoClientRequest para que tungstenite gestione los headers del handshake
            let mut request = url.as_str().into_client_request().unwrap();
            request.headers_mut().insert(
                "Authorization",
                format!("Token {}", api_key).parse().unwrap(),
            );

            let (ws_stream, _) = match connect_async_tls_with_config(request, None, true, None).await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Error al conectar con Deepgram STT: {}", e);
                    return;
                }
            };

            let (mut write, mut read) = ws_stream.split();

            // Task 1: Enviar audio
            let send_task = tauri::async_runtime::spawn(async move {
                while let Some(audio) = rx.recv().await {
                    if write.send(Message::Binary(audio.into())).await.is_err() {
                        break;
                    }
                }
                // Enviar mensaje de cierre
                let _ = write.send(Message::Binary(vec![].into())).await;
            });

            // Task 2: Recibir transcripciones
            let app_handle_clone = app_handle.clone();
            let receive_task = tauri::async_runtime::spawn(async move {
                while let Some(msg) = read.next().await {
                    if let Ok(Message::Text(text)) = msg {
                        if let Ok(resp) = serde_json::from_str::<DeepgramResponse>(&text) {
                            // Intentar extraer de 'transcript' (formato Flux)
                            let transcript_text = if let Some(t) = resp.transcript {
                                Some(t)
                            } else {
                                // Intentar extraer de 'channel' (formato estándar)
                                resp.channel.and_then(|c| c.alternatives.get(0).map(|a| a.transcript.clone()))
                            };

                            if let Some(txt) = transcript_text {
                                if !txt.is_empty() {
                                    let data = SttData {
                                        text: txt,
                                        is_final: resp.is_final,
                                    };
                                    let _ = app_handle_clone.emit("stt_data", &data);
                                }
                            }
                        }
                    }
                }
            });

            let _ = futures_util::future::join(send_task, receive_task).await;
        });

        Ok(SttClient { tx })
    }

    pub fn clone_tx(&self) -> mpsc::Sender<Vec<u8>> {
        self.tx.clone()
    }
}
