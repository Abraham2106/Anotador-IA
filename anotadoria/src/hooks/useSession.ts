import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

export interface WaveformFrame {
  samples: number[];
  rms: number;
  is_clipping: boolean;
}

export interface SttData {
  text: String;
  is_final: boolean;
}

/**
 * useSession Hook
 * Responsabilidad: Abstraer el flujo de la sesión, audio y transcripción.
 */
export const useSession = () => {
  const [sessionStatus, setSessionStatus] = useState<string>('idle');
  const [waveform, setWaveform] = useState<WaveformFrame>({
    samples: new Array(64).fill(0),
    rms: 0,
    is_clipping: false
  });
  
  // Transcripción estable (confirmada)
  const [stableTranscript, setStableTranscript] = useState<string>("");
  // Transcripción en vivo (predicción actual)
  const [interimTranscript, setInterimTranscript] = useState<string>("");

  useEffect(() => {
    // Escuchar cambios de estado de sesión
    const unlistenStatus = listen<string>('session_status', (event) => {
      setSessionStatus(event.payload);
      if (event.payload === 'idle') {
          // Limpiar al terminar
          // setStableTranscript(""); // Podríamos mantenerlo, pero para este sprint limpiamos
          setInterimTranscript("");
      }
    });

    // Escuchar datos de waveform (~60fps)
    const unlistenWaveform = listen<WaveformFrame>('waveform_data', (event) => {
      setWaveform(event.payload);
    });

    // Escuchar datos de STT
    const unlistenStt = listen<SttData>('stt_data', (event) => {
        const { text, is_final } = event.payload;
        if (is_final) {
            setStableTranscript(prev => prev + " " + text);
            setInterimTranscript("");
        } else {
            setInterimTranscript(text as string);
        }
    });

  return () => {
      unlistenStatus.then(f => f());
      unlistenWaveform.then(f => f());
      unlistenStt.then(f => f());
    };
  }, []);

  return {
    sessionStatus,
    waveform,
    stableTranscript,
    interimTranscript,
    setSessionStatus,
  };
};
