# AnotadorIA — Plan de Sprints

> Metodología: sprints de 1 semana. Cada sprint tiene scope cerrado, criterios de
> aceptación verificables y un logbook que se llena conforme se completa el trabajo.
> El sprint activo se marca con `▶`. Los completados con `✓`.

---

## Índice de Sprints

| # | Nombre | Estado | Objetivo |
|---|--------|--------|----------|
| 1 | Scaffolding y configuración | ✓ Completado | Proyecto compilando, config.toml leyéndose |
| 2 | Captura de audio y waveform | ▶ Activo | Audio capturado, waveform en UI |
| 3 | Integración STT — Deepgram | Pendiente | Transcripción en tiempo real al vault |
| 4 | Filtro de disfluencias y escritura estable | Pendiente | Solo tokens finales al .md |
| 5 | Agentes LLM — limpieza y resumen | Pendiente | Nota completa con resumen al detener |
| 6 | Polish de UI y waveform avanzado | Pendiente | Clipping detection, indicadores visuales |
| 7 | Empaquetado y distribución personal | Pendiente | Binario instalable en macOS/Windows |

---

## Sprint 1 — Scaffolding y Configuración ✓

**Objetivo:** El proyecto compila sin errores. La app se abre como ventana flotante.
El archivo `config.toml` se lee correctamente al iniciar.

**Duración:** Semana 1

### Tareas

- [x] Crear proyecto Tauri v2 con `create-tauri-app` (template: react-ts)
- [x] Configurar ventana en `tauri.conf.json`: `alwaysOnTop: true`, sin decoraciones
- [x] Crear `config.rs` con struct `AppConfig` y deserialización con `serde`
- [x] Crear `config.toml` con campos: `vault_path`, `deepgram_api_key`, `claude_api_key`
- [x] Agregar `config.toml` al `.gitignore`
- [x] Exponer comando Tauri `get_config` que devuelve la config al frontend
- [x] Verificar que el frontend puede llamar `invoke('get_config')` sin error
- [x] Crear estructura de carpetas definitiva en `src-tauri/src/`

### Criterios de aceptación

- `cargo build` pasa sin warnings
- La app se abre y la ventana aparece siempre encima de otras ventanas
- `console.log` del resultado de `invoke('get_config')` muestra los valores del TOML
- `.gitignore` ignora `config.toml`

### Cómo se logra paso a paso

**1. Inicializar el proyecto:**
```bash
npm create tauri-app@latest anotadoria -- --template react-ts
cd anotadoria
```

**2. Configurar la ventana en `tauri.conf.json`:**
```json
{
  "app": {
    "windows": [{
      "label": "main",
      "title": "AnotadorIA",
      "width": 320,
      "height": 120,
      "alwaysOnTop": true,
      "decorations": false,
      "resizable": false,
      "transparent": true
    }]
  }
}
```

**3. Crear `config.rs`:**
```rust
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub vault_path: String,
    pub deepgram_api_key: String,
    pub claude_api_key: String,
    pub language: String,
    pub model_stt: String,
    pub model_llm: String,
    pub note_subfolder: Option<String>,
}

pub fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string("config.toml")?;
    let config: AppConfig = toml::from_str(&content)?;
    Ok(config)
}
```

**4. Registrar el comando en `main.rs`:**
```rust
#[tauri::command]
fn get_config() -> Result<AppConfig, String> {
    config::load_config().map_err(|e| e.to_string())
}
```

---

### Logbook Sprint 1

```
[2026-04-09] INICIO
Estado inicial: proyecto vacío.

[2026-04-09] COMPLETADO: Scaffolding con create-tauri-app
Resultado: carpetas src/ y src-tauri/ creadas. App compila y abre ventana básica.

[2026-04-09] COMPLETADO: config.rs + config.toml
Resultado: invoke('get_config') devuelve los campos correctamente en el frontend.
Nota: se necesitó agregar toml = "0.8" y serde = { features = ["derive"] } al Cargo.toml.

[2026-04-09] COMPLETADO: ventana always-on-top sin decoraciones
Resultado: la ventana flota sobre Obsidian correctamente.

[2026-04-09] SPRINT COMPLETADO ✓
Todos los criterios de aceptación verificados.
```

---

## Sprint 2 — Captura de Audio y Waveform ▶

**Objetivo:** El micrófono se captura con `cpal`. Los datos PCM se envían al frontend
en tiempo real. El `WaveformCanvas` dibuja la forma de onda activa.

**Duración:** Semana 2

### Tareas

- [ ] Agregar dependencia `cpal` al `Cargo.toml`
- [ ] Crear `audio_capture.rs`: abrir stream de entrada por defecto, emitir frames PCM
- [ ] Crear `waveform_analyzer.rs`: calcular RMS por frame, detectar clipping > 0.95
- [ ] Emitir evento Tauri `waveform_data` con payload `Vec<f32>` (muestreado a 60fps)
- [ ] Crear `WaveformCanvas.tsx`: canvas que dibuja la onda usando `requestAnimationFrame`
- [ ] Crear `useSession.ts`: hook que suscribe al evento `waveform_data`
- [ ] Mostrar `ClippingAlert` cuando el evento incluye flag `is_clipping: true`
- [ ] Crear `RecordButton.tsx` con estados: `idle` / `recording`

### Criterios de aceptación

- Al presionar Record, el waveform anima en tiempo real
- El waveform se detiene al presionar Stop
- Si se habla muy fuerte, aparece el indicador de clipping
- Sin pérdida de frames perceptible (la onda es fluida)

### Cómo se logra paso a paso

**1. Agregar cpal a Cargo.toml:**
```toml
[dependencies]
cpal = "0.15"
tauri = { version = "2", features = ["protocol-asset"] }
```

**2. Estructura de `audio_capture.rs`:**
```rust
// Responsabilidad única: abrir el stream de audio y emitir frames PCM.
// NO hace análisis. NO escribe archivos. Solo captura.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct AudioCapture {
    stream: cpal::Stream,
}

impl AudioCapture {
    pub fn start<F>(callback: F) -> Result<Self, cpal::BuildStreamError>
    where
        F: Fn(Vec<f32>) + Send + 'static,
    {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .expect("No se encontró dispositivo de entrada");
        let config = device.default_input_config()?;

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _| callback(data.to_vec()),
            |err| eprintln!("Error de stream: {}", err),
            None,
        )?;

        stream.play()?;
        Ok(AudioCapture { stream })
    }
}
```

**3. `waveform_analyzer.rs` — análisis separado de captura:**
```rust
// Responsabilidad única: análisis de señal. No sabe nada de Tauri ni de archivos.

pub struct WaveformFrame {
    pub samples: Vec<f32>,
    pub rms: f32,
    pub is_clipping: bool,
}

pub fn analyze(samples: &[f32]) -> WaveformFrame {
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    let is_clipping = samples.iter().any(|&s| s.abs() > 0.95);
    WaveformFrame {
        samples: samples.to_vec(),
        rms,
        is_clipping,
    }
}
```

**4. Emisión de eventos desde `session_manager.rs`:**
```rust
// Emitir a 60fps: throttle con std::time::Instant
app_handle.emit("waveform_data", &frame).unwrap();
```

**5. `WaveformCanvas.tsx` — solo renderiza, sin lógica:**
```tsx
// Hook externo provee los datos. El canvas solo pinta.
const WaveformCanvas = ({ samples }: { samples: Float32Array }) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const ctx = canvasRef.current?.getContext('2d');
    if (!ctx || !samples.length) return;
    // dibujar waveform centrado
  }, [samples]);

  return <canvas ref={canvasRef} width={280} height={60} />;
};
```

### Logbook Sprint 2

```
[2026-04-09] INICIO
Partiendo desde Sprint 1 completado. La app abre y lee config.toml.

[ ] Pendiente: captura de audio con cpal
[ ] Pendiente: waveform en frontend
```

> Completa las entradas conforme avances:
```
[FECHA] COMPLETADO / PROBLEMA / SOLUCION: descripción
```

---

## Sprint 3 — Integración STT — Deepgram

**Objetivo:** El audio capturado se envía a Deepgram via WebSocket. Los fragmentos
transcritos aparecen en la UI como preview y se emiten al siguiente módulo.

**Duración:** Semana 3

### Tareas

- [ ] Agregar `tokio-tungstenite` y `tokio` al `Cargo.toml`
- [ ] Crear `stt_client.rs`: conectar WebSocket a Deepgram, enviar frames PCM
- [ ] Parsear respuesta JSON de Deepgram: extraer `transcript` e `is_final`
- [ ] Emitir evento Tauri `transcript_interim` (preview en UI) y `transcript_final`
- [ ] Crear `bilingual_config.rs`: parámetros de conexión (language, model, keywords)
- [ ] Mostrar texto interim en la UI con estilo "preview" (opacidad reducida)

### Criterios de aceptación

- Al hablar, aparece texto en la UI (interim)
- El texto se estabiliza al terminar la frase (final)
- Términos técnicos en inglés se transcriben correctamente
- La conexión WebSocket se cierra limpiamente al detener la grabación

### Cómo se logra paso a paso

**1. URL de conexión a Deepgram:**
```
wss://api.deepgram.com/v1/listen
  ?model=nova-3-general
  &language=es
  &smart_format=true
  &interim_results=true
  &encoding=linear16
  &sample_rate=16000
```

**2. Estructura del mensaje de respuesta a parsear:**
```json
{
  "channel": {
    "alternatives": [{ "transcript": "texto aquí" }]
  },
  "is_final": true
}
```

**3. Separación de responsabilidades en `stt_client.rs`:**
```rust
// stt_client.rs SOLO maneja la conexión WebSocket y el parsing.
// No sabe nada de archivos ni de la UI.
// Expone dos callbacks: on_interim y on_final.

pub struct SttClient { /* ws_stream, config */ }

impl SttClient {
    pub async fn connect(config: &AppConfig) -> Result<Self, ...> { ... }
    pub async fn send_audio(&mut self, pcm: &[i16]) -> Result<(), ...> { ... }
    pub async fn recv_transcript(&mut self) -> Result<TranscriptEvent, ...> { ... }
}
```

### Logbook Sprint 3

```
[FECHA] INICIO
Partiendo desde Sprint 2 completado. Audio capturado y waveform funcionando.

[ ] Pendiente: todos los items del sprint
```

---

## Sprint 4 — Filtro de Disfluencias y Escritura al Vault

**Objetivo:** Solo los tokens `is_final` se escriben al archivo `.md` del vault.
Los interims se descartan del filesystem. La nota crece en tiempo real en Obsidian.

**Duración:** Semana 4

### Tareas

- [ ] Crear `disfluency_filter.rs`: buffer de interim, flush solo en is_final
- [ ] Crear `vault_writer.rs`: crear archivo al iniciar sesión, append por fragmento
- [ ] Crear `template_builder.rs`: frontmatter YAML inicial de la nota
- [ ] Integrar en `session_manager.rs`: conectar STT → filter → writer
- [ ] Verificar en Obsidian que el texto aparece en tiempo real (Live Preview)

### Criterios de aceptación

- El archivo `.md` se crea al presionar Record
- El texto final (no interim) aparece en Obsidian mientras se habla
- Al detener, el archivo tiene el texto completo sin disfluencias escritas
- El frontmatter incluye: `date`, `tags: [transcripcion]`, `duration`

### Cómo se logra paso a paso

**1. `disfluency_filter.rs` — lógica de descarte:**
```rust
// Solo deja pasar eventos is_final: true.
// Los interim se guardan en buffer solo para mostrar en UI, nunca para escritura.

pub struct DisfluencyFilter {
    pending_interim: String,
}

impl DisfluencyFilter {
    pub fn process(&mut self, event: TranscriptEvent) -> Option<String> {
        if event.is_final {
            self.pending_interim.clear();
            Some(event.transcript)  // Este sí va al archivo
        } else {
            self.pending_interim = event.transcript.clone();
            None  // Solo para UI, no para archivo
        }
    }
}
```

**2. `vault_writer.rs` — escritura directa:**
```rust
use std::fs::OpenOptions;
use std::io::Write;

pub struct VaultWriter {
    file: std::fs::File,
}

impl VaultWriter {
    pub fn create_session_file(vault_path: &str, subfolder: Option<&str>) -> Result<Self, ...> {
        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H%M");
        let filename = format!("{}_Sesion.md", timestamp);
        // construir path, crear archivo, escribir frontmatter
    }

    pub fn append(&mut self, text: &str) -> Result<(), std::io::Error> {
        writeln!(self.file, "{}", text)
    }
}
```

**3. Frontmatter template:**
```markdown
---
date: 2025-01-15
tags: [transcripcion, anotadoria]
duration: 
modelo_stt: nova-3-general
---

# Transcripción — 15 enero 2025

```

### Logbook Sprint 4

```
[FECHA] INICIO
Partiendo desde Sprint 3 completado. STT conectado y emitiendo eventos.

[ ] Pendiente: todos los items del sprint
```

---

## Sprint 5 — Agentes LLM — Limpieza y Resumen

**Objetivo:** Al detener la grabación, el texto completo pasa por dos agentes
secenciales a través del proxy local `gemini-proxy-balancer`. El resultado se
agrega al `.md` como bloque `## Resumen`.

**Duración:** Semana 5

### Tareas

- [ ] Implementar cliente HTTP OpenAI-compatible en `llm_agents.rs` (ya stubeado)
- [ ] Configurar prompts detallados en `prompt_templates.rs` (ya stubeado)
- [ ] Integrar en `session_manager.rs`: on_stop → clean → summarize → append
- [ ] Implementar lógica de fallback: si el proxy falla, conservar el texto raw
- [ ] Emitir evento `processing_status` a la UI durante el procesamiento
- [ ] Mostrar spinner en la UI mientras los agentes trabajan

### Criterios de aceptación

- Al detener, la UI muestra estado "Procesando..."
- Si el proxy está offline, se emite un warning pero se guarda el texto raw
- El `.md` final contiene la transcripción limpia + bloque `## Resumen`
- El resumen tiene action items en formato `- [ ] Acción concreta`
- El tiempo total de post-procesamiento es < 30 segundos usando Gemini Flash

### Prompts de los agentes

**Agente 1 — Limpieza:**
```
Eres un editor técnico. Recibirás una transcripción de voz en español con posibles
términos técnicos en inglés. Tu tarea es ÚNICAMENTE corregir:
- Errores de reconocimiento acústico obvios (palabras que no tienen sentido en contexto)
- Fragmentos incompletos por corte de audio

NO debes:
- Cambiar el significado de ninguna decisión mencionada
- Reescribir frases que tengan sentido aunque suenen informales
- Agregar información que no esté en el original

Devuelve SOLO el texto corregido, sin comentarios ni explicaciones.
```

**Agente 2 — Resumen:**
```
Eres un asistente de productividad. Recibirás una transcripción de una sesión de trabajo.
Genera un resumen con la siguiente estructura exacta en Markdown:

## Resumen

### Temas tratados
- [tema conciso]

### Decisiones tomadas
- [decisión específica]

### Action Items
- [ ] [acción concreta con verbo en infinitivo]

Reglas:
- Cada bullet debe ser una oración, no un párrafo
- Los action items deben ser accionables (empezar con verbo: Revisar, Crear, Enviar...)
- Máximo 10 bullets por sección
- Si no hay decisiones o action items claros, omite esa sección
```

### Logbook Sprint 5

```
[FECHA] INICIO
Partiendo desde Sprint 4 completado. Transcripción escribiéndose al vault.

[ ] Pendiente: todos los items del sprint
```

---

## Sprint 6 — Polish de UI

**Objetivo:** La UI refleja todos los estados de la app con claridad. El waveform
muestra clipping. Los errores de conexión se comunican al usuario.

**Duración:** Semana 6

### Tareas

- [ ] Indicador visual de clipping (borde rojo en waveform)
- [ ] Estado "Procesando agentes..." con barra de progreso indeterminada
- [ ] Manejo de error: Deepgram desconectado → mensaje en UI + retry automático
- [ ] Manejo de error: vault_path no existe → mensaje claro al iniciar
- [ ] Animación suave de transición entre estados (idle → recording → processing)
- [ ] Mostrar nombre del archivo .md activo en la UI

### Logbook Sprint 6

```
[FECHA] INICIO
[ ] Pendiente: todos los items del sprint
```

---

## Sprint 7 — Empaquetado y Distribución Personal

**Objetivo:** La app se puede instalar en la máquina personal sin herramientas de
desarrollo. El binario es autónomo.

**Duración:** Semana 7

### Tareas

- [ ] Configurar `tauri.conf.json` para bundle: macOS (.dmg) / Windows (.msi)
- [ ] Agregar ícono de app en todos los tamaños requeridos
- [ ] Crear script de instalación que copia `config.toml.example` al directorio correcto
- [ ] Verificar que el binario corre sin `cargo` instalado
- [ ] Documentar proceso de instalación en `README.md`

### Logbook Sprint 7

```
[FECHA] INICIO
[ ] Pendiente: todos los items del sprint
```

---

## Cómo usar este archivo

### Al iniciar un sprint

1. Cambia el estado del sprint anterior a `✓ Completado`
2. Cambia el sprint nuevo a `▶ Activo`
3. Agrega la fecha de inicio en el logbook correspondiente

### Durante el sprint

Después de cada sesión de trabajo, agrega una entrada al logbook del sprint activo:

```
[YYYY-MM-DD] COMPLETADO: nombre de la tarea
Resultado: qué funciona ahora que antes no funcionaba.
Notas: algo que otros deberían saber antes de tocar este código.

[YYYY-MM-DD] PROBLEMA: descripción del problema
Síntoma: qué comportamiento incorrecto se observó.
Contexto: en qué condiciones ocurre.

[YYYY-MM-DD] SOLUCION: para el problema anterior
Causa raíz: por qué pasaba.
Fix aplicado: qué se cambió.
Archivos modificados: lista de archivos.
```

### Al cerrar un sprint

Verifica cada criterio de aceptación. Si alguno no se cumplió, documéntalo y
decide si bloqueará el siguiente sprint o puede quedar como deuda técnica.
