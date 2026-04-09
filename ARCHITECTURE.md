# AnotadorIA — Arquitectura y Principios de Calidad

> Este documento es la constitución técnica del proyecto. Antes de agregar un módulo,
> refactorizar código existente o tomar una decisión de diseño, consulta estas guías.
> No son sugerencias — son reglas que mantienen el proyecto mantenible por una sola persona
> durante meses sin perder contexto.

---

## Índice

1. Visión general de capas
2. Principio de responsabilidad única por módulo
3. Bajo acoplamiento — reglas de dependencias
4. Alta cohesión — qué va con qué
5. Patrones de diseño aplicados
6. Convenciones de código Rust
7. Convenciones de código TypeScript/React
8. Documentación inline
9. Testing por módulo
10. Qué nunca hacer — anti-patrones prohibidos

---

## 1. Visión General de Capas

La arquitectura tiene cuatro capas con dirección de dependencia estricta.
Las capas superiores conocen a las inferiores. Nunca al revés.

```
┌─────────────────────────────────┐
│  UI Layer (React)               │  ← Solo renderiza. Solo emite eventos.
│  WaveformCanvas, RecordButton   │     No tiene lógica de negocio.
└────────────┬────────────────────┘
             │ Tauri invoke() / listen()
┌────────────▼────────────────────┐
│  Orchestration Layer (Rust)     │  ← session_manager.rs coordina el flujo.
│  session_manager.rs             │     Conoce todos los módulos de dominio.
└────────────┬────────────────────┘
             │ llamadas directas de función
┌────────────▼────────────────────┐
│  Domain Layer (Rust)            │  ← Módulos de dominio. NO se conocen entre sí.
│  audio_capture, stt_client,     │     Solo conocen sus propias structs.
│  disfluency_filter, llm_agents, │
│  vault_writer, waveform_analyzer│
└────────────┬────────────────────┘
             │ lectura de config
┌────────────▼────────────────────┐
│  Infrastructure Layer (Rust)    │  ← config.rs, filesystem, red externa.
│  config.rs, std::fs, reqwest    │     Sin lógica de negocio.
└─────────────────────────────────┘
```

**Regla de oro:** si un módulo de dominio necesita importar otro módulo de dominio,
es una señal de que la coordinación pertenece al `session_manager`, no al módulo.

---

## 2. Responsabilidad Única por Módulo

Cada archivo tiene exactamente una razón para cambiar. Si encuentras que un archivo
cambia por dos razones diferentes, debe dividirse.

| Archivo | Responsabilidad | Cambia si... |
|---------|----------------|--------------|
| `config.rs` | Deserializar config.toml | Cambia la estructura del TOML |
| `audio_capture.rs` | Abrir stream PCM con cpal | Cambia la librería de audio o el formato |
| `waveform_analyzer.rs` | Calcular RMS y detectar clipping | Cambia la lógica de análisis de señal |
| `stt_client.rs` | Conexión WebSocket a Deepgram | Cambia el proveedor de STT |
| `disfluency_filter.rs` | Descartar interim tokens | Cambia la lógica de estabilización |
| `llm_agents.rs` | Llamadas HTTP a la API del LLM | Cambia el proveedor de LLM |
| `prompt_templates.rs` | Texto de los prompts | Cambia la calidad de los prompts |
| `vault_writer.rs` | Crear y escribir el archivo .md | Cambia el formato de la nota |
| `template_builder.rs` | Frontmatter YAML y estructura | Cambia el template de la nota |
| `session_manager.rs` | Orquestar el flujo completo | Cambia la lógica de negocio principal |

---

## 3. Bajo Acoplamiento — Reglas de Dependencias

### Regla 1: Los módulos de dominio no se importan entre sí

```rust
// ❌ PROHIBIDO: audio_capture importando stt_client
// src-tauri/src/audio_capture.rs
use crate::stt_client::SttClient;  // NUNCA

// ✅ CORRECTO: session_manager coordina ambos
// src-tauri/src/session_manager.rs
use crate::audio_capture::AudioCapture;
use crate::stt_client::SttClient;
// session_manager conecta el output de uno con el input del otro
```

### Regla 2: Los módulos reciben datos, no referencias a otros módulos

```rust
// ❌ PROHIBIDO: pasar el VaultWriter al SttClient
impl SttClient {
    pub fn new(writer: &mut VaultWriter) -> Self { ... }  // NUNCA
}

// ✅ CORRECTO: SttClient devuelve datos, session_manager los escribe
let transcript = stt_client.recv_transcript().await?;
vault_writer.append(&transcript.text)?;
```

### Regla 3: Usa tipos de dominio propios, no tipos de librerías externas en interfaces públicas

```rust
// ❌ PROHIBIDO: exponer tipos de cpal en la interfaz pública
pub fn get_stream() -> cpal::Stream { ... }

// ✅ CORRECTO: wrapper propio que oculta la dependencia
pub struct AudioCapture { stream: cpal::Stream }  // cpal queda encapsulado
pub fn start<F: Fn(Vec<f32>)>(callback: F) -> Result<AudioCapture, AudioError> { ... }
```

### Regla 4: La UI no conoce nada de los tipos de Rust

El frontend solo trabaja con JSON serializable. Los tipos complejos de Rust se
aplanan antes de cruzar el bridge:

```typescript
// ❌ PROHIBIDO: tipos que reflejan estructura interna de Rust
interface TranscriptEvent { channel: { alternatives: [{ transcript: string }] } }

// ✅ CORRECTO: tipo de dominio plano y explícito
interface TranscriptUpdate {
  text: string
  isFinal: boolean
  timestamp: number
}
```

---

## 4. Alta Cohesión — Qué Va Con Qué

La cohesión mide qué tan relacionadas están las responsabilidades dentro de un módulo.
Alta cohesión = todo lo que está en el módulo pertenece ahí.

### `waveform_analyzer.rs` — ejemplo de alta cohesión

```rust
// Todo lo que está aquí es sobre análisis de señal de audio.
// Nada más.

pub struct WaveformFrame { pub samples: Vec<f32>, pub rms: f32, pub is_clipping: bool }

pub fn analyze(samples: &[f32]) -> WaveformFrame { ... }
pub fn normalize(samples: &[f32]) -> Vec<f32> { ... }  // también análisis de señal ✓
pub fn resample(samples: &[f32], target_len: usize) -> Vec<f32> { ... }  // ✓
```

### Señales de baja cohesión — mueve el código

Si en `waveform_analyzer.rs` encuentras:
- Código que escribe al filesystem → mover a `vault_writer.rs`
- Código que llama a Deepgram → mover a `stt_client.rs`
- Código que maneja la config → mover a `config.rs`

---

## 5. Patrones de Diseño Aplicados

### Observer Pattern — eventos Tauri

El `session_manager` emite eventos; la UI los observa. La UI nunca pregunta por estado
(polling). El estado llega cuando cambia.

```rust
// session_manager.rs emite, no retorna
app_handle.emit("transcript_update", &TranscriptUpdate { text, is_final }).unwrap();
app_handle.emit("waveform_data", &WaveformFrame { rms, is_clipping }).unwrap();
app_handle.emit("session_status", &SessionStatus::Processing).unwrap();
```

```typescript
// useSession.ts observa, no pregunta
useEffect(() => {
  const unlisten = listen<TranscriptUpdate>('transcript_update', (event) => {
    setInterimText(event.payload.text)
  })
  return () => { unlisten.then(f => f()) }
}, [])
```

### State Machine — ciclo de vida de la sesión

El `session_manager` implementa una máquina de estados explícita. No hay flags booleanos
sueltos. El estado actual determina qué operaciones son válidas.

```rust
// session_manager.rs

#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Idle,
    Recording { started_at: std::time::Instant },
    Processing { transcript: String },
    Error { message: String },
}

impl SessionManager {
    pub fn transition(&mut self, new_state: SessionState) {
        // validar transición antes de aplicarla
        match (&self.state, &new_state) {
            (SessionState::Idle, SessionState::Recording { .. }) => {},
            (SessionState::Recording { .. }, SessionState::Processing { .. }) => {},
            (SessionState::Processing { .. }, SessionState::Idle) => {},
            (_, SessionState::Error { .. }) => {},
            (from, to) => panic!("Transición inválida: {:?} → {:?}", from, to),
        }
        self.state = new_state;
    }
}
```

### Builder Pattern — construcción de la nota

`template_builder.rs` construye el frontmatter y la estructura de la nota de forma
fluida, sin concatenación de strings sucia en el `vault_writer`.

```rust
pub struct NoteBuilder {
    date: String,
    tags: Vec<String>,
    model: String,
}

impl NoteBuilder {
    pub fn new() -> Self { ... }
    pub fn with_tag(mut self, tag: &str) -> Self { self.tags.push(tag.to_string()); self }
    pub fn build_frontmatter(&self) -> String { ... }
    pub fn build_header(&self) -> String { ... }
}

// uso en vault_writer.rs:
let note = NoteBuilder::new()
    .with_tag("transcripcion")
    .with_tag("anotadoria")
    .build_frontmatter();
```

### Strategy Pattern — intercambio de proveedor STT/LLM

Los clientes externos se abstraen detrás de traits, permitiendo cambiar Deepgram por
otro proveedor sin tocar `session_manager`.

```rust
// Definir el contrato
pub trait SttProvider: Send {
    async fn send_audio(&mut self, pcm: &[i16]) -> Result<(), SttError>;
    async fn recv_transcript(&mut self) -> Result<TranscriptEvent, SttError>;
}

// Implementación concreta
pub struct DeepgramClient { ... }
impl SttProvider for DeepgramClient { ... }

// session_manager trabaja con el trait, no con la implementación
pub struct SessionManager {
    stt: Box<dyn SttProvider>,
}
```

---

## 6. Convenciones de Código Rust

### Nombrado

```rust
// Módulos y archivos: snake_case
audio_capture.rs
vault_writer.rs

// Structs y Enums: PascalCase
pub struct AudioCapture { ... }
pub enum SessionState { ... }

// Funciones y métodos: snake_case, verbos
pub fn start_recording() { ... }
pub fn analyze_frame() { ... }

// Constantes: SCREAMING_SNAKE_CASE
const MAX_CLIPPING_THRESHOLD: f32 = 0.95;
const WAVEFORM_EMIT_INTERVAL_MS: u64 = 16; // ~60fps
```

### Manejo de errores

```rust
// ❌ PROHIBIDO: unwrap() en código de producción
let config = load_config().unwrap();

// ✅ CORRECTO: propagar con ? o manejar explícitamente
let config = load_config().map_err(|e| format!("Error al leer config.toml: {}", e))?;

// ✅ CORRECTO: errores de dominio propios
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("El vault path no existe: {0}")]
    PathNotFound(String),
    #[error("Sin permisos de escritura en: {0}")]
    PermissionDenied(String),
    #[error("Error de IO: {0}")]
    Io(#[from] std::io::Error),
}
```

### Documentación inline

```rust
/// Analiza un frame de audio y devuelve métricas de señal.
///
/// # Arguments
/// * `samples` - Muestras PCM f32 normalizadas en rango [-1.0, 1.0]
///
/// # Returns
/// `WaveformFrame` con RMS calculado y flag de clipping si alguna muestra supera 0.95
///
/// # Example
/// ```
/// let frame = analyze(&audio_samples);
/// if frame.is_clipping { warn!("Saturación detectada"); }
/// ```
pub fn analyze(samples: &[f32]) -> WaveformFrame { ... }
```

Regla mínima: toda función pública `pub fn` lleva comentario `///`. Las privadas
solo si la lógica no es obvia.

---

## 7. Convenciones de Código TypeScript/React

### Un componente = un archivo = una responsabilidad

```
WaveformCanvas.tsx    → solo dibuja la onda
RecordButton.tsx      → solo el botón con sus estados
ClippingAlert.tsx     → solo el indicador de saturación
```

### Los hooks abstraen la comunicación con Tauri

```typescript
// ❌ PROHIBIDO: invoke() suelto en un componente
const RecordButton = () => {
  const handleClick = () => invoke('start_recording')  // acoplado a Tauri
}

// ✅ CORRECTO: toda comunicación Tauri va en hooks
// hooks/useSession.ts
export const useSession = () => {
  const startRecording = () => invoke<void>('start_recording')
  const stopRecording = () => invoke<void>('stop_recording')
  return { startRecording, stopRecording, ... }
}

// RecordButton solo usa el hook
const RecordButton = () => {
  const { startRecording, stopRecording } = useSession()
  // ...
}
```

### Props tipadas, sin `any`

```typescript
// ❌ PROHIBIDO
const WaveformCanvas = ({ samples }: { samples: any }) => { ... }

// ✅ CORRECTO
interface WaveformCanvasProps {
  samples: Float32Array
  width?: number
  height?: number
}
const WaveformCanvas = ({ samples, width = 280, height = 60 }: WaveformCanvasProps) => { ... }
```

---

## 8. Documentación Inline — Estándar Mínimo

### En Rust

Cada módulo tiene un comentario de cabecera que explica su responsabilidad:

```rust
//! # audio_capture
//!
//! Responsabilidad: abrir el stream de audio del micrófono por defecto usando cpal
//! y emitir frames PCM f32 a través de un callback.
//!
//! NO hace: análisis de señal, transcripción, escritura de archivos.
//! Para análisis ver: waveform_analyzer.rs
//! Para transcripción ver: stt_client.rs
```

### En TypeScript

```typescript
/**
 * Hook que encapsula toda comunicación con el backend Tauri para la sesión de grabación.
 * 
 * Emite: start_recording, stop_recording
 * Escucha: transcript_update, waveform_data, session_status, clipping_alert
 * 
 * @returns Estado de la sesión y funciones de control
 */
export const useSession = () => { ... }
```

---

## 9. Testing por Módulo

Para uso personal, prioriza tests donde los bugs serían silenciosos o costosos.

### Alta prioridad (siempre testear)

```rust
// disfluency_filter.rs — si falla, texto incorrecto llega al vault
#[cfg(test)]
mod tests {
    #[test]
    fn interim_never_passes_through() { ... }

    #[test]
    fn final_token_passes_through() { ... }

    #[test]
    fn multiple_interims_before_final() { ... }
}

// prompt_templates.rs — si cambia accidentalmente, los agentes producen basura
#[test]
fn cleaner_prompt_contains_no_alter_meaning_instruction() { ... }

// vault_writer.rs — si falla, se pierden notas
#[test]
fn creates_file_in_correct_path() { ... }

#[test]
fn append_does_not_overwrite() { ... }
```

### Baja prioridad (solo si hay tiempo)

- `waveform_analyzer.rs` — fácil de verificar visualmente
- `config.rs` — serde ya está testeado por la librería
- UI components — verificar visualmente durante desarrollo

---

## 10. Anti-patrones Prohibidos

### En Rust

```rust
// ❌ unwrap() en producción
config.unwrap()

// ❌ clone() para evitar entender el borrow checker
let data = expensive_data.clone();  // si no entiendes por qué, pregunta primero

// ❌ tipos genéricos excesivos cuando un tipo concreto es suficiente
fn process<T: AsRef<str> + Clone + Debug>(input: T) -> T  // si solo necesitas &str

// ❌ lógica de negocio en main.rs
// main.rs solo inicializa; toda lógica va en módulos

// ❌ múltiples responsabilidades por función
fn capture_and_transcribe_and_write() { ... }  // dividir en tres funciones
```

### En TypeScript/React

```typescript
// ❌ Estado en componentes que debería estar en hooks
const [waveformData, setWaveformData] = useState()  // dentro de WaveformCanvas.tsx

// ❌ Llamadas directas a invoke() fuera de hooks
await invoke('start_recording')  // dentro de un componente

// ❌ useEffect sin cleanup para listeners de Tauri
// Siempre retornar la función de unlisten para evitar memory leaks

// ❌ Props drilling profundo
// Si necesitas pasar props más de 2 niveles, usar context o mover al hook
```

### En general

```
❌ Comentarios que explican QUÉ hace el código (el código ya lo dice)
   // incrementa el contador en 1
   count += 1;

✅ Comentarios que explican POR QUÉ
   // Deepgram requiere que el primer frame sea enviado dentro de los primeros 250ms
   // de abrir el WebSocket, de lo contrario cierra la conexión silenciosamente
   stt_client.send_keepalive().await?;

❌ Variables de una letra excepto en iteradores cortos
   let x = load_config();  // qué es x?

✅ Nombres que se leen como prosa
   let app_config = load_config()?;
```

---

## Checklist antes de hacer commit

Antes de cada commit, verifica:

- [ ] `cargo build` pasa sin warnings
- [ ] `cargo test` pasa
- [ ] Ningún `unwrap()` nuevo sin justificación en comentario
- [ ] Toda función pública nueva tiene comentario `///`
- [ ] Ningún módulo de dominio importa otro módulo de dominio
- [ ] El `config.toml` real no está en el commit (verificar con `git status`)
- [ ] Si se agregó una dependencia externa, está justificada en el commit message
- [ ] El logbook del sprint activo en `SPRINTS.md` tiene una entrada para este trabajo
