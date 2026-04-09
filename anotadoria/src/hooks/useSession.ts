import { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

export interface WaveformFrame {
  samples: number[];
  rms: number;
  is_clipping: boolean;
}

/**
 * useSession Hook
 * Responsabilidad: Abstraer el flujo de la sesión y comunicación con Tauri.
 */
export const useSession = () => {
  const [sessionStatus, setSessionStatus] = useState<string>('idle');
  const [waveform, setWaveform] = useState<WaveformFrame>({
    samples: new Array(64).fill(0),
    rms: 0,
    is_clipping: false
  });

  useEffect(() => {
    // Escuchar cambios de estado de sesión
    const unlistenStatus = listen<string>('session_status', (event) => {
      setSessionStatus(event.payload);
    });

    // Escuchar datos de waveform (~60fps)
    const unlistenWaveform = listen<WaveformFrame>('waveform_data', (event) => {
      setWaveform(event.payload);
    });

    return () => {
      unlistenStatus.then(f => f());
      unlistenWaveform.then(f => f());
    };
  }, []);

  return {
    sessionStatus,
    waveform,
    setSessionStatus,
  };
};
