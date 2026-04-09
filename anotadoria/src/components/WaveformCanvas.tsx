import React from 'react';

/**
 * WaveformCanvas Component
 * Responsabilidad: Dibujar la forma de onda de audio en un canvas 2D.
 */
const WaveformCanvas: React.FC = () => {
  return (
    <div className="waveform-container">
      <canvas width={280} height={60} />
    </div>
  );
};

export default WaveformCanvas;
