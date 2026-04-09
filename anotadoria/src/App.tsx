import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSession } from "./hooks/useSession";
import WaveformCanvas from "./components/WaveformCanvas";
import "./App.css";

function App() {
  const { sessionStatus, waveform } = useSession();
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
        <span className="state-pill">
          {sessionStatus === 'recording' ? '● GRABANDO' : '● LISTO'}
        </span>
        {!configLoaded && <span style={{ color: '#ef4444' }}> (Error Config)</span>}
      </div>

      <div className="waveform-wrapper">
        <WaveformCanvas data={waveform} />
      </div>

      <div className="controls">
        <button 
          onClick={handleToggleRecording} 
          disabled={!configLoaded}
          className={`record-btn ${sessionStatus === 'recording' ? 'active' : ''}`}
        >
          <div className="inner-circle"></div>
        </button>
      </div>
    </main>
  );
}

export default App;
