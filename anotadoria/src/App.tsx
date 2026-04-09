import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSession } from "./hooks/useSession";
import WaveformCanvas from "./components/WaveformCanvas";
import TranscriptDisplay from "./components/TranscriptDisplay";
import SessionHistory from "./components/SessionHistory";
import "./App.css";

interface Session {
  id: string;
  date: string;
  text: string;
}

function App() {
  const { sessionStatus, waveform, stableTranscript, interimTranscript, setStableTranscript } = useSession();
  const [loadingConfig, setLoadingConfig] = useState(true);
  const [configError, setConfigError] = useState<string | null>(null);
  const [sessions, setSessions] = useState<Session[]>([]);

  // Cargar historial al iniciar
  useEffect(() => {
    const saved = localStorage.getItem('anotador_sessions');
    if (saved) {
      try {
        setSessions(JSON.parse(saved));
      } catch (e) {
        console.error("Error cargando historial:", e);
      }
    }

    const isTauri = !!(window as any).__TAURI_INTERNALS__;
    
    if (isTauri) {
      invoke("get_config")
        .then((config) => {
          console.log("Configuración cargada (Tauri):", config);
          setLoadingConfig(false);
          setConfigError(null);
        })
        .catch((err) => {
          console.error("Error cargando config en Tauri:", err);
          setLoadingConfig(false);
          setConfigError(err.toString());
        });
    } else {
      setLoadingConfig(false);
      setConfigError(null);
    }
  }, []);

  const handleToggleRecording = async () => {
    if (sessionStatus === 'idle') {
      invoke('start_recording').catch(console.error);
    } else if (sessionStatus === 'recording') {
      await invoke('stop_recording').catch(console.error);
      
      // Guardar sesión si hay texto
      if (stableTranscript.trim()) {
        const newSession: Session = {
          id: Date.now().toString(),
          date: new Date().toLocaleString(),
          text: stableTranscript.trim()
        };
        const updated = [newSession, ...sessions];
        setSessions(updated);
        localStorage.setItem('anotador_sessions', JSON.stringify(updated));
        // Opcional: limpiar transcripción actual después de guardar
        // setStableTranscript(""); 
      }
    }
  };

  const deleteSession = (id: string) => {
    const updated = sessions.filter(s => s.id !== id);
    setSessions(updated);
    localStorage.setItem('anotador_sessions', JSON.stringify(updated));
  };

  const clearSessions = () => {
    if (window.confirm("¿Seguro que quieres borrar todo el historial?")) {
      setSessions([]);
      localStorage.removeItem('anotador_sessions');
    }
  };

  return (
    <main 
      data-tauri-drag-region
      className={`container ${sessionStatus === 'recording' ? 'recording' : ''}`}
    >
      <div className="status-bar">
        {loadingConfig ? (
          <span className="state-pill">● LOADING...</span>
        ) : configError ? (
          <span className="state-pill error" title={configError}>● CONFIG ERROR</span>
        ) : (
          <span className={`state-pill ${sessionStatus === 'recording' ? 'recording' : ''}`}>
            {sessionStatus === 'recording' ? '● RECORDING' : '● READY'}
          </span>
        )}
      </div>

      <div className="main-content">
        {sessionStatus === 'recording' || stableTranscript ? (
            <TranscriptDisplay stable={stableTranscript} interim={interimTranscript} />
        ) : (
            <div className="welcome-area">
                <span className="welcome-text">AnotadorIA</span>
                <span className="subtitle">Listo para escuchar</span>
            </div>
        )}

        <div className="recorder-section">
            <div className="waveform-wrap">
                <WaveformCanvas data={waveform} />
            </div>
            
            <div className="controls">
                <button 
                  onClick={handleToggleRecording} 
                  disabled={loadingConfig || !!configError}
                  className={`record-btn ${sessionStatus === 'recording' ? 'active' : ''}`}
                  title={sessionStatus === 'recording' ? 'Stop Recording' : 'Start Recording'}
                >
                  <div className="inner-circle"></div>
                </button>
            </div>
        </div>

        <SessionHistory 
          sessions={sessions} 
          onDelete={deleteSession} 
          onClear={clearSessions} 
        />
      </div>
    </main>
  );
}

export default App;
