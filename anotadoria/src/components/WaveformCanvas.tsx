import React, { useEffect, useRef } from 'react';
import { WaveformFrame } from '../hooks/useSession';

interface WaveformCanvasProps {
  data: WaveformFrame;
}

/**
 * WaveformCanvas Component
 * Responsabilidad: Dibujar la forma de onda de audio en un canvas 2D.
 */
const WaveformCanvas: React.FC<WaveformCanvasProps> = ({ data }) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const { samples, is_clipping } = data;
    const { width, height } = canvas;
    const barWidth = width / samples.length;

    // Limpiar fondo
    ctx.clearRect(0, 0, width, height);

    // Color de la onda
    ctx.fillStyle = is_clipping ? '#ef4444' : '#22c55e'; // Rojo si hay clipping, verde si no

    samples.forEach((sample, i) => {
        // Normalizamos la altura (la mayoría de las muestras de voz son bajas)
        // Multiplicamos por 2 para que sea más visible.
        const amplitude = sample * height * 1.5;
        const x = i * barWidth;
        const y = (height - amplitude) / 2;

        // Dibujar barra redondeada
        ctx.beginPath();
        ctx.roundRect(x + 1, y, barWidth - 2, amplitude, 2);
        ctx.fill();
    });
  }, [data]);

  return (
    <div className="waveform-container">
      <canvas 
        ref={canvasRef} 
        width={280} 
        height={60} 
        style={{ width: '100%', height: '100%' }}
      />
    </div>
  );
};

export default WaveformCanvas;
