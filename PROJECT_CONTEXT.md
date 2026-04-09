# AnotadorIA — Contexto del Proyecto

> **Uso:** Copia este archivo completo como contexto inicial a cualquier LLM antes de trabajar en el proyecto. Contiene el prompt de recreación, la descripción funcional y el log histórico de decisiones.

---

## Prompt de Recreación del Proyecto

Pega esto al inicio de cualquier sesión con un LLM nuevo:

```
Eres un Senior Software Engineer trabajando en "AnotadorIA", una aplicación de escritorio
de uso personal construida con Tauri v2 (Rust backend + React/Vite frontend).

La app es un transcriptor y resumidor de audio que se integra directamente con un vault
local de Obsidian. Captura audio del micrófono, transcribe en tiempo real vía Deepgram
Nova-3 (con soporte bilingüe Español/Inglés), y escribe el texto estabilizado directamente
a un archivo .md en el vault. Al detener la grabación, dos agentes LLM secuenciales
(limpieza de transcripción y generación de resumen con action items) completan la nota.

Stack técnico:
- Backend: Rust (Tauri v2), cpal (audio), tokio-tungstenite (WebSocket Deepgram),
  reqwest (HTTP Claude API), serde (config.toml)
- Frontend: React + Vite (TypeScript), Canvas 2D para waveform
- Config: archivo config.toml local, gitignoreado
- Escritura: std::fs::OpenOptions append directo al vault, sin worker threads adicionales

Principios de diseño que DEBES respetar:
1. Bajo acoplamiento: cada módulo Rust expone una interfaz mínima, sin dependencias cruzadas
2. Alta cohesión: un archivo = una responsabilidad
3. Uso personal: sin multi-usuario, sin autenticación, sin settings UI compleja
4. El frontend NUNCA tiene lógica de negocio; solo emite eventos y renderiza estado

Antes de escribir código, consulta siempre:
- PROJECT_CONTEXT.md (este archivo) para entender decisiones pasadas
- ARCHITECTURE.md para principios de diseño y patrones aprobados
- SPRINTS.md para el estado actual del sprint y lo que está en scope

Estructura de carpetas activa:
anotadoria/
├── src-tauri/src/
│   ├── main.rs
│   ├── session_manager.rs
│   ├── audio_capture.rs
│   ├── waveform_analyzer.rs
│   ├── stt_client.rs
│   ├── disfluency_filter.rs
│   ├── llm_agents.rs
│   ├── prompt_templates.rs
│   ├── vault_writer.rs
│   └── config.rs
├── src/
│   ├── App.tsx
│   ├── components/
│   │   ├── WaveformCanvas.tsx
│   │   ├── RecordButton.tsx
│   │   └── ClippingAlert.tsx
│   └── hooks/
│       └── useSession.ts
└── config.toml  (gitignoreado)
```

---

## Descripción Funcional Completa

### Qué hace la app

AnotadorIA es una herramienta de productividad personal que convierte conversaciones,
reuniones o sesiones de pensamiento en voz en notas estructuradas de Obsidian, sin
fricción y en tiempo real.

### Flujo de usuario

1. El usuario abre AnotadorIA (ventana flotante, siempre visible sobre Obsidian).
2. Presiona el botón de grabación.
3. La app crea automáticamente un archivo `YYYY-MM-DD_HHmm_Sesion.md` en el vault.
4. El waveform visualizer confirma que el audio se está capturando.
5. Mientras habla, el texto transcrito aparece en la nota de Obsidian en tiempo real.
6. Al detener la grabación:
   a. Agente 1 (Limpieza): corrige errores acústicos sin alterar el significado.
   b. Agente 2 (Resumen): agrega un bloque `## Resumen` con action items concisos.
7. La nota queda lista en Obsidian.

### Casos especiales manejados

- **Disfluencias:** el sistema solo escribe tokens `is_final: true` de Deepgram.
  Los interim tokens se muestran en la UI como preview gris, nunca se persisten.
- **Code-switching:** Deepgram Nova-3 con `language: es` maneja Spanglish nativo.
  Términos técnicos en inglés se transcriben correctamente sin configuración adicional.
- **Saturación de micrófono:** `waveform_analyzer.rs` detecta picos > 0.95 RMS y
  emite un evento `clipping_alert` que la UI muestra como indicador visual rojo.

---

## Log de Cambios y Decisiones

> Formato de entrada: `[YYYY-MM-DD] [TIPO] Descripción — Razón`
> Tipos: DECISION, CAMBIO, REFACTOR, PROBLEMA, SOLUCION, DESCARTADO

---

### 2025-01 — Fase de Diseño Inicial

```
[2025-01-XX] DECISION  Stack: Tauri v2 sobre Electron
Razón: Electron requiere compilar node-portaudio contra headers nativos del SO,
lo que rompía en cada actualización de Node.js. Tauri usa cpal (Rust puro) que
compila sin dependencias de sistema. Binario resultante: ~8 MB vs ~150 MB.

[2025-01-XX] DECISION  Audio: cpal sobre PortAudio bindings de Node.js
Razón: cpal es cross-platform (macOS CoreAudio, Windows WASAPI, Linux ALSA) sin
requerir instalación de headers externos. Una sola dependencia en Cargo.toml.

[2025-01-XX] DECISION  STT: Deepgram Nova-3 sobre Whisper local
Razón: Whisper local requiere Python runtime y modelos de varios GB. Para uso
personal con conexión a internet, Deepgram ofrece latencia <300ms y soporte
nativo de code-switching ES/EN sin fine-tuning.

[2025-01-XX] DECISION  Config: config.toml sobre UI de settings
Razón: Uso personal. Editar un TOML es más rápido y menos código que construir
una pantalla de configuración. El archivo se gitignora para proteger las API keys.

[2025-01-XX] DECISION  Escritura: std::fs append síncrono sobre worker threads
Razón: El session_manager tiene ownership exclusivo del file handle. Rust garantiza
seguridad de concurrencia en tiempo de compilación, eliminando la necesidad del
patrón worker thread que se usaría en Node.js.

[2025-01-XX] DESCARTADO  Electron como framework de escritorio
Razón: Ver decisión de Tauri arriba. Adicionalmente, el overhead de RAM de Electron
(~180 MB idle) es innecesario para una app de uso personal que corre en segundo plano.

[2025-01-XX] DESCARTADO  Integración via Obsidian Plugin API
Razón: Requeriría que el usuario instale un plugin, aprender la API de plugins de
Obsidian, y manejar compatibilidad entre versiones. La escritura directa al filesystem
es más simple y más robusta para uso personal.

---

### 2026-04 — Fase de Implementación

[2026-04-09] DECISION  LLM: gemini-proxy-balancer sobre Claude directo
Razón: Evitar rate limits de la API gratuita de Gemini mediante rotación de keys y
cooldowns automáticos. Permite costo $0 para uso personal con alta disponibilidad.
Impacto: `config.rs`, `llm_agents.rs`, `config.toml`.

[2026-04-09] CAMBIO  Scaffolding inicial y Configuración
Razón: Inicio del desarrollo físico del proyecto con Tauri v2 y React-TS.
Impacto: Toda la estructura de carpetas y el bridge inicial de comandos.
```

---

### Cómo agregar una entrada al log

Cuando hagas un cambio significativo, agrega una entrada aquí con el formato:

```
[FECHA] TIPO  Título corto
Razón: Explicación de por qué se tomó esta decisión o hizo este cambio.
Impacto: Qué archivos o módulos afecta.
```

Tipos disponibles:
- `DECISION` — se elige una opción sobre otra
- `CAMBIO` — modificación de funcionalidad existente
- `REFACTOR` — cambio interno sin cambio de comportamiento externo
- `PROBLEMA` — bug o limitación encontrada
- `SOLUCION` — fix para un PROBLEMA documentado
- `DESCARTADO` — opción que se evaluó y se rechazó (importante documentar el por qué)
