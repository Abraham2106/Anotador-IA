import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSession } from "./hooks/useSession";
import WaveformCanvas from "./components/WaveformCanvas";
import TranscriptDisplay from "./components/TranscriptDisplay";
import "./App.css";

function App() {
  const { sessionStatus, waveform, stableTranscript, interimTranscript } = useSession();
  const [configLoaded, setConfigLoaded] = useState(false);

  useEffect(() => {
    invoke("get_config")
      .then((config) => {
        console.log("Configuración cargada:", config);
        setConfigLoaded(true);
      })
      .catch((err) => {
        console.error("Error al cargar configuración:", err);
      });
  }, []);

  const handleToggleRecording = () => {
    if (sessionStatus === 'idle') {
      invoke('start_recording').catch(console.error);
    } else if (sessionStatus === 'recording') {
      invoke('stop_recording').catch(console.error);
    }
  };

  return (
    <main className="container">
      <div className="status-bar">
        <span className={`state-pill ${sessionStatus === 'recording' ? 'recording' : ''}`}>
          {sessionStatus === 'recording' ? '● RECORDING' : '● READY'}
        </span>
        {!configLoaded && <span className="state-pill error">● CONFIG ERROR</span>}
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
            <WaveformCanvas data={waveform} />
            
            <div className="controls">
                <button 
                onClick={handleToggleRecording} 
                disabled={!configLoaded}
                className={`record-btn ${sessionStatus === 'recording' ? 'active' : ''}`}
                title={sessionStatus === 'recording' ? 'Stop Recording' : 'Start Recording'}
                >
                <div className="inner-circle"></div>
                </button>
            </div>
        </div>
      </div>
    </main>
  );
}

export default App;
