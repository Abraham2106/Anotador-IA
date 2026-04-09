import React from 'react';

/**
 * ClippingAlert Component
 * Responsabilidad: Mostrar un indicador visual cuando el audio satura.
 */
const ClippingAlert: React.FC = () => {
  return (
    <div className="clipping-alert" style={{ display: 'none' }}>
      ⚠️ Clipping Detected
    </div>
  );
};

export default ClippingAlert;
