//! # waveform_analyzer
//!
//! Responsabilidad: análisis de señal (RMS, clipping) y procesamiento de datos
//! para visualización (downsampling).

use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct WaveformFrame {
    pub samples: Vec<f32>,
    pub rms: f32,
    pub is_clipping: bool,
}

/// Analiza un buffer de audio y devuelve métricas de señal y muestras promediadas.
pub fn analyze(samples: &[f32], target_bins: usize) -> WaveformFrame {
    if samples.is_empty() {
        return WaveformFrame {
            samples: vec![0.0; target_bins],
            rms: 0.0,
            is_clipping: false,
        };
    }

    // 1. Detección de clipping y cálculo de RMS
    let mut sum_sq = 0.0;
    let mut is_clipping = false;

    for &s in samples {
        sum_sq += s * s;
        if s.abs() > 0.95 {
            is_clipping = true;
        }
    }

    let rms = (sum_sq / samples.len() as f32).sqrt();

    // 2. Downsampling (promediado simple para visualización)
    // Dividimos el buffer en 'target_bins' secciones.
    let bin_size = samples.len() / target_bins;
    let mut downsampled = Vec::with_capacity(target_bins);

    if bin_size > 0 {
        for chunk in samples.chunks(bin_size) {
            if downsampled.len() >= target_bins { break; }
            let max_abs = chunk.iter().fold(0.0, |max, &s| max.max(s.abs()));
            downsampled.push(max_abs);
        }
    }

    // Asegurar que tenemos exactamente el número de bins solicitado
    while downsampled.len() < target_bins {
        downsampled.push(0.0);
    }

    WaveformFrame {
        samples: downsampled,
        rms,
        is_clipping,
    }
}
