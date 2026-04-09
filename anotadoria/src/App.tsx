import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
  const [sessionStatus, setSessionStatus] = useState("idle");
  const [configLoaded, setConfigLoaded] = useState(false);

  useEffect(() => {
    // Verificar que la configuración se puede leer desde el backend
    invoke("get_config")
      .then((config) => {
        console.log("Configuración cargada:", config);
        setConfigLoaded(true);
      })
      .catch((err) => {
        console.error("Error al cargar configuración:", err);
      });
  }, []);

  return (
    <main className="container">
      <div className="status-bar">
        <span>Estado: {sessionStatus}</span>
        {configLoaded ? (
          <span style={{ color: 'green' }}> ● Config OK</span>
        ) : (
          <span style={{ color: 'red' }}> ● Config Error</span>
        )}
      </div>

      <div className="waveform-placeholder">
        {/* WaveformCanvas irá aquí en el Sprint 2 */}
        <div style={{ height: '60px', background: 'rgba(255,255,255,0.1)', borderRadius: '8px' }}></div>
      </div>

      <div className="controls">
        {/* RecordButton irá aquí en el Sprint 2 */}
        <button disabled={!configLoaded}>
          Record
        </button>
      </div>
    </main>
  );
}

export default App;
