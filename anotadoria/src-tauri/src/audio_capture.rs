//! # audio_capture
//!
//! Responsabilidad: abrir el stream de audio del micrófono por defecto usando cpal
//! y emitir frames PCM f32 a través de un callback.
//!
//! NO hace: análisis de señal, transcripción, escritura de archivos.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

pub struct AudioCapture {
    stream: cpal::Stream,
}

impl AudioCapture {
    /// Inicia la captura de audio en un hilo separado.
    /// 
    /// # Arguments
    /// * `callback` - Función que recibe un buffer de muestras f32 promediadas o crudas.
    pub fn start<F>(callback: F) -> Result<Self, Box<dyn std::error::Error>>
    where
        F: Fn(Vec<f32>) + Send + 'static,
    {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or("No se encontró dispositivo de entrada por defecto")?;
        
        let config = device.default_input_config()?;
        let sample_format = config.sample_format();
        let config_stream: cpal::StreamConfig = config.into();

        let callback = Arc::new(Mutex::new(callback));

        let stream = match sample_format {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config_stream,
                move |data: &[f32], _| {
                    if let Ok(cb) = callback.lock() {
                        cb(data.to_vec());
                    }
                },
                |err| eprintln!("Error en stream de audio: {}", err),
                None
            )?,
            _ => return Err("Formato de audio no soportado (solo f32)".into()),
        };

        stream.play()?;

        Ok(AudioCapture { stream })
    }

    /// Detiene el stream de audio (se llama automáticamente al soltar la struct).
    pub fn stop(&self) {
        let _ = self.stream.pause();
    }
}
