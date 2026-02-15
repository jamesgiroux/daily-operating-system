/**
 * AtmosphereLayer.tsx
 *
 * Fixed-position atmospheric background with page-specific radial gradients
 * and breathing animation. Renders behind all content (z-index: 0).
 *
 * Includes a watermark asterisk that floats behind hero content.
 */

import React from 'react';
import styles from './AtmosphereLayer.module.css';

export interface AtmosphereLayerProps {
  /**
   * Color scheme for the gradient and watermark
   * Default: 'turmeric'
   */
  color?: 'turmeric' | 'terracotta' | 'larkspur';

  /**
   * Optional: custom class name for styling
   */
  className?: string;
}

export const AtmosphereLayer: React.FC<AtmosphereLayerProps> = ({
  color = 'turmeric',
  className = '',
}) => {
  const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

  return (
    <div className={`${styles.atmosphere} ${styles[color]} ${className}`}>
      {/* Watermark asterisk — subtly visible, for visual interest */}
      <div className={`${styles.watermark} ${styles[`watermark${cap(color)}`] || ''}`}>
        *
      </div>
    </div>
  );
};

export default AtmosphereLayer;
