import React from 'react';

interface Session {
  id: string;
  date: string;
  text: string;
}

interface Props {
  sessions: Session[];
  onDelete: (id: string) => void;
  onClear: () => void;
}

const SessionHistory: React.FC<Props> = ({ sessions, onDelete, onClear }) => {
  if (sessions.length === 0) return null;

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    // Podríamos añadir una notificación sutil aquí
  };

  return (
    <div className="session-history">
      <div className="history-header">
        <h3 className="history-title">Historial de Sesiones</h3>
        <button className="clear-btn" onClick={onClear}>Limpiar Todo</button>
      </div>
      
      <div className="history-list">
        {sessions.map((session) => (
          <div 
            key={session.id} 
            className="session-card"
            onClick={() => handleCopy(session.text)}
            title="Haga clic para copiar al portapapeles"
          >
            <div className="session-card-header">
              <span className="session-date">{session.date}</span>
              <div className="session-actions">
                <span className="copy-hint">Copiar</span>
                <button 
                  className="delete-card-btn" 
                  onClick={(e) => {
                    e.stopPropagation();
                    onDelete(session.id);
                  }}
                  title="Eliminar sesión"
                >
                  ✕
                </button>
              </div>
            </div>
            <p className="session-text">{session.text}</p>
          </div>
        ))}
      </div>
    </div>
  );
};

export default SessionHistory;
