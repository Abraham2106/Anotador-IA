# AnotadorIA — Integración con gemini-proxy-balancer

> **Contexto para cualquier LLM que trabaje en este módulo:**
> Los agentes de limpieza y resumen de AnotadorIA NO llaman directamente a ninguna
> API de LLM. Toda comunicación LLM pasa por un proxy local corriendo en
> `http://localhost:8000`. Este proxy es `gemini-proxy-balancer`, un servidor
> FastAPI que rota entre múltiples API keys de Gemini y expone el endpoint
> `/v1/chat/completions` en formato 100% compatible con OpenAI.
> Fuente: https://github.com/Abraham2106/gemini-proxy-balancer

---

## Por qué este enfoque

### El problema que resuelve

La API gratuita de Gemini tiene límites estrictos por key: RPM (requests per minute)
y TPM (tokens per minute) bajos. En una sesión de 30 minutos transcribiendo,
el agente de limpieza puede recibir un texto de 5,000+ tokens, y el de resumen otro
llamado inmediatamente después. Con una sola key, el segundo agente casi siempre
recibe un `429 Resource Exhausted`.

### La solución

El proxy agrupa N API keys de Gemini detrás de un único endpoint local. Cuando una
key recibe un 429, la pone en cooldown usando el tiempo exacto que Google especifica
en la respuesta (`retryDelay`), y rota automáticamente a la siguiente. Para
`llm_agents.rs`, todo esto es invisible — solo ve un endpoint HTTP local que
siempre responde.

### Ventaja adicional: costo cero

Gemini 2.5 Flash tiene tier gratuito generoso. Con 3-5 API keys rotando, las sesiones
de uso personal nunca deberían alcanzar los límites. Costo operativo: $0.

---

## Arquitectura con el proxy incluido

```
AnotadorIA (Tauri app)
│
├── session_manager.rs
│   └── on_stop() →
│       ├── llm_agents::clean_transcript(raw_text)
│       │   └── POST http://localhost:8000/v1/chat/completions
│       │       { model: "gemini-2.5-flash", messages: [...] }
│       │
│       └── llm_agents::generate_summary(clean_text)
│           └── POST http://localhost:8000/v1/chat/completions
│               { model: "gemini-2.5-flash", messages: [...] }
│
gemini-proxy-balancer (proceso separado, localhost:8000)
│
├── Recibe el POST
├── Selecciona la mejor (key, modelo) disponible
├── Llama a la API real de Gemini
├── Si 429 → cooldown + rota a siguiente key
├── Si todo falla → 503 con detalle en logs
└── Devuelve respuesta en formato OpenAI
    { choices: [{ message: { content: "..." } }] }
```

---

## Setup del proxy (hacer una sola vez)

### 1. Clonar y preparar el proxy

```bash
git clone https://github.com/Abraham2106/gemini-proxy-balancer.git
cd gemini-proxy-balancer
pip install -r requirements.txt
```

### 2. Configurar las API keys

Crea un archivo `.env` en el directorio del proxy:

```bash
# gemini-proxy-balancer/.env
# Agrega tantas keys como tengas (mínimo 2 recomendado para uso con AnotadorIA)
GEMINI_API_KEYS="AIzaSy...,AIzaSy...,AIzaSy..."

# Opcional: definir el orden de modelos preferido
# Por defecto usa: gemini-2.5-flash → gemini-2.0-flash-lite → otros
GEMINI_MODELS="gemini-2.5-flash,gemini-2.0-flash-lite"
```

Para obtener API keys gratuitas: https://aistudio.google.com/apikey
Cada cuenta de Google puede generar una key. Usa cuentas distintas.

### 3. Iniciar el proxy

```bash
# Desde el directorio gemini-proxy-balancer/
uvicorn main:app --port 8000 --host 127.0.0.1
```

El proxy corre en `http://127.0.0.1:8000`. Solo acepta conexiones locales (127.0.0.1),
nunca expuesto a la red.

### 4. Verificar que funciona

```bash
curl http://localhost:8000/health

# Respuesta esperada:
# { "status": "ok", "keys_count": 3, "models": ["gemini-2.5-flash", ...] }
```

### 5. Automatizar el inicio (opcional pero recomendado)

Para que el proxy se inicie automáticamente cuando abras AnotadorIA:

**macOS — LaunchAgent:**
```xml
<!-- ~/Library/LaunchAgents/com.anotadoria.proxy.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "...">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.anotadoria.proxy</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/local/bin/uvicorn</string>
    <string>main:app</string>
    <string>--port</string><string>8000</string>
    <string>--host</string><string>127.0.0.1</string>
  </array>
  <key>WorkingDirectory</key>
  <string>/ruta/a/gemini-proxy-balancer</string>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
</dict>
</plist>
```
```bash
launchctl load ~/Library/LaunchAgents/com.anotadoria.proxy.plist
```

**Windows — Task Scheduler o script de inicio:**
```batch
:: proxy-start.bat (agregar al startup de Windows)
cd C:\ruta\a\gemini-proxy-balancer
start /B uvicorn main:app --port 8000 --host 127.0.0.1
```

---

## Cambios en config.toml de AnotadorIA

Reemplazar la línea de `claude_api_key` por la URL del proxy:

```toml
# config.toml — AnotadorIA
vault_path        = "/Users/tu_usuario/Documents/ObsidianVault/Notas"
deepgram_api_key  = "dg_..."
language          = "es"
model_stt         = "nova-3-general"

# LLM — ya no se necesita API key directa, el proxy la maneja
llm_proxy_url     = "http://127.0.0.1:8000/v1/chat/completions"
llm_model         = "gemini-2.5-flash"   # El proxy hará fallback si está rate-limited

# Opcional: timeout en segundos para las llamadas a los agentes
llm_timeout_secs  = 60
```

---

## Cambios en `config.rs`

```rust
//! config.rs — actualizado para proxy local en lugar de API key directa

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
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
```

---

## Implementación de `llm_agents.rs`

Este es el módulo que más cambia. Antes apuntaba a la API de Claude directamente;
ahora hace un POST estándar al proxy local. El contrato del endpoint es
OpenAI-compatible, por lo que el código es simple y estable.

```rust
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
```

---

## `prompt_templates.rs` — sin cambios de lógica

Los prompts no cambian al cambiar de proveedor. Gemini 2.5 Flash entiende las
mismas instrucciones que Claude. Únicas consideraciones:

```rust
//! prompt_templates.rs
//!
//! Los prompts están en español porque el contenido a procesar es Spanglish.
//! Gemini 2.5 Flash maneja español nativo sin degradación de calidad.

pub const CLEANER_SYSTEM: &str = "\
Eres un editor técnico. Recibirás una transcripción de voz en español \
con posibles términos técnicos en inglés (code-switching natural). \
Tu tarea es ÚNICAMENTE corregir errores de reconocimiento acústico obvios: \
palabras que no tienen sentido en contexto, o fragmentos incompletos por corte de audio. \

NO debes: \
- Cambiar el significado de ninguna decisión mencionada. \
- Reescribir frases que tengan sentido aunque suenen informales. \
- Agregar información que no esté en el original. \
- Corregir gramática si el mensaje es comprensible. \

Devuelve SOLO el texto corregido, sin comentarios, sin explicaciones, sin markdown.";

pub const SUMMARIZER_SYSTEM: &str = "\
Eres un asistente de productividad. Recibirás una transcripción de una sesión de trabajo. \
Genera un resumen con esta estructura exacta en Markdown:\n\
\n\
## Resumen\n\
\n\
### Temas tratados\n\
- [tema conciso]\n\
\n\
### Decisiones tomadas\n\
- [decisión específica]\n\
\n\
### Action Items\n\
- [ ] [acción con verbo en infinitivo]\n\
\n\
Reglas: \
bullet = una oración máximo. \
Action items empiezan con verbo: Revisar, Crear, Enviar, Investigar, Definir. \
Máximo 8 bullets por sección. \
Si no hay decisiones o action items claros, omite esa sección completamente. \
No incluyas secciones vacías.";
```

---

## Verificación de salud del proxy desde Rust

Antes de iniciar una sesión de grabación, `session_manager.rs` debería verificar
que el proxy está vivo. Esto evita que el usuario grabe 20 minutos y descubra
al final que los agentes no pueden correr.

```rust
// En session_manager.rs, llamar antes de start_recording:

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
```

Si falla, emitir evento Tauri `llm_proxy_offline` y mostrar un banner en la UI:

```
⚠️ Proxy LLM offline — la transcripción funcionará pero no habrá resumen al terminar.
   Para activarlo: uvicorn main:app --port 8000
```

Esto es una degradación elegante: la app sigue siendo útil aunque el proxy no esté.

---

## Flujo completo al detener la grabación

```rust
// session_manager.rs — método on_stop()

pub async fn on_stop(&mut self, app_handle: &AppHandle) -> Result<(), SessionError> {
    // 1. Detener captura de audio y STT
    self.audio_capture.stop();
    self.stt_client.close().await?;

    // 2. Notificar UI: procesando
    app_handle.emit("session_status", "processing").unwrap();

    // 3. Leer el texto completo escrito hasta ahora
    let raw_transcript = self.vault_writer.read_transcript()?;

    // 4. Agente 1: limpieza (puede tardar 5-15s según largo)
    let clean_text = match self.llm_agents.clean_transcript(&raw_transcript).await {
        Ok(text) => text,
        Err(AgentError::ProxyUnreachable(_)) => {
            // Degradación elegante: usar texto raw si el proxy está offline
            app_handle.emit("agent_warning", "Limpieza omitida: proxy offline").unwrap();
            raw_transcript.clone()
        },
        Err(e) => return Err(SessionError::AgentFailed(e.to_string())),
    };

    // 5. Sobreescribir la transcripción con la versión limpia
    self.vault_writer.replace_transcript(&clean_text)?;

    // 6. Agente 2: resumen
    let summary = match self.llm_agents.generate_summary(&clean_text).await {
        Ok(s) => s,
        Err(AgentError::ProxyUnreachable(_)) => {
            app_handle.emit("agent_warning", "Resumen omitido: proxy offline").unwrap();
            return Ok(());
        },
        Err(e) => return Err(SessionError::AgentFailed(e.to_string())),
    };

    // 7. Agregar el bloque de resumen al final del .md
    self.vault_writer.append_summary(&summary)?;

    // 8. Notificar UI: listo
    app_handle.emit("session_status", "done").unwrap();

    Ok(())
}
```

---

## Comparativa: antes vs después

| Aspecto | Con Claude API directo | Con gemini-proxy-balancer |
|---------|----------------------|--------------------------|
| Costo mensual | Variable (~$0.50-5 USD/mes) | $0 (Gemini free tier) |
| API key en config.toml | Sí (claude_api_key) | No (solo llm_proxy_url) |
| Manejo de rate limits | Manual en Rust | Automático en el proxy |
| Fallback de modelos | No implementado | Automático (2.5-flash → 2.0-flash-lite) |
| Cambiar de modelo | Editar config.toml | Editar .env del proxy |
| Complejidad de llm_agents.rs | Alta (headers auth, error handling) | Baja (POST estándar) |
| Dependencia externa en runtime | API de Claude (internet) | localhost:8000 + Gemini |

---

## Notas de seguridad

El proxy corre en `127.0.0.1:8000` (loopback only). Ningún otro dispositivo en la
red puede acceder a él. Las API keys de Gemini viven solo en el `.env` del proxy,
nunca en el código ni en el `config.toml` de AnotadorIA.

Si el proxy se inicia con `--host 0.0.0.0`, cualquier dispositivo en la red local
podría usarlo para consumir tus keys. Mantener siempre `--host 127.0.0.1`.

---

## Troubleshooting

**El proxy dice "503 All keys exhausted"**
Significa que todas tus keys están en cooldown simultáneamente. Soluciones:
- Agrega más keys al `.env` del proxy (mínimo 3 para uso intensivo)
- Espera 60 segundos y vuelve a intentar
- Verifica el estado con `GET http://localhost:8000/proxy-state`

**La app muestra "proxy offline" pero el proxy está corriendo**
Verifica que el proxy corre en el puerto 8000 y en `127.0.0.1`, no en `0.0.0.0`:
```bash
curl http://127.0.0.1:8000/health
```

**El resumen sale en inglés**
El `SUMMARIZER_SYSTEM` prompt ya incluye instrucciones en español. Si Gemini
responde en inglés, agrega al final del prompt: `"Responde siempre en español."`

**Latencia alta en los agentes (>30s)**
Normal para transcripciones largas (>10,000 tokens). El timeout en `config.toml`
puede aumentarse a 120 segundos. El proxy no tiene timeout propio por defecto.