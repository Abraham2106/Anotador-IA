//! # audio_capture
//!
//! Responsabilidad: abrir el stream de audio del micrófono por defecto usando cpal.
//! Para compatibilidad con Windows, el stream se maneja en un hilo dedicado.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;
use std::thread;

pub struct AudioHandle {
    stop_tx: mpsc::Sender<()>,
}

impl AudioHandle {
    pub fn stop(self) {
        let _ = self.stop_tx.send(());
    }
}

pub struct AudioCapture;

impl AudioCapture {
    /// Inicia la captura de audio en un hilo dedicado. Devuelve el handle y el sample_rate detectado.
    pub fn start<F>(callback: F) -> Result<(AudioHandle, u32), Box<dyn std::error::Error>>
    where
        F: Fn(Vec<f32>) + Send + 'static,
    {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or("No se encontró dispositivo de entrada por defecto")?;
        
        let config = device.default_input_config()?;
        let sample_rate = config.sample_rate().0;
        let sample_format = config.sample_format();
        let config_stream: cpal::StreamConfig = config.into();

        let (stop_tx, stop_rx) = mpsc::channel();

        thread::spawn(move || {
            let stream = match sample_format {
                cpal::SampleFormat::F32 => device.build_input_stream(
                    &config_stream,
                    move |data: &[f32], _| callback(data.to_vec()),
                    |err| eprintln!("Error en stream de audio: {}", err),
                    None
                ).expect("No se pudo construir el stream"),
                _ => panic!("Formato de audio no soportado"),
            };

            stream.play().expect("No se pudo iniciar el stream");

            // El hilo se mantiene vivo hasta recibir señal de parada
            let _ = stop_rx.recv();
            let _ = stream.pause();
            drop(stream);
        });

        Ok((AudioHandle { stop_tx }, sample_rate))
    }
}
