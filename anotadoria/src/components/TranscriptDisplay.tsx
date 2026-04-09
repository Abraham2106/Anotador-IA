import React, { useEffect, useRef } from 'react';

interface TranscriptDisplayProps {
  stable: string;
  interim: string;
}

/**
 * TranscriptDisplay Component
 * Responsabilidad: Mostrar el texto procesado y el texto en curso (interim).
 */
const TranscriptDisplay: React.FC<TranscriptDisplayProps> = ({ stable, interim }) => {
  const containerRef = useRef<HTMLDivElement>(null);

  // Auto-scroll al final cuando llega texto nuevo
  useEffect(() => {
    if (containerRef.current) {
        containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [stable, interim]);

  return (
    <div className="transcript-container" ref={containerRef}>
      <span className="stable-text">{stable}</span>
      <span className="interim-text">{interim}</span>
      {(stable || interim) ? null : <span className="placeholder-text">Empieza a hablar para transcribir...</span>}
    </div>
  );
};

export default TranscriptDisplay;
