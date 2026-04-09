/**
 * useSession Hook
 * Responsabilidad: Abstraer el flujo de la sesión y comunicación con Tauri.
 */
export const useSession = () => {
  return {
    status: 'idle',
    transcript: '',
    startRecording: () => console.log('Start recording'),
    stopRecording: () => console.log('Stop recording'),
  };
};
