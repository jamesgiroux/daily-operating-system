/**
 * BrandMark — Montserrat ExtraBold asterisk as SVG path.
 * Extracted from Montserrat-ExtraBold.ttf glyph outline via fonttools.
 * Uses `fill="currentColor"` so it inherits from parent CSS color.
 */
import React from 'react';

interface BrandMarkProps {
  className?: string;
  /** Width/height in px (square). Default: '1em' (inherits font-size). */
  size?: string | number;
  style?: React.CSSProperties;
}

export function BrandMark({ className, size = '1em', style }: BrandMarkProps) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 433 407"
      width={size}
      height={size}
      className={className}
      style={style}
      aria-hidden="true"
    >
      <path
        d="M159 407 161 292 57 355 0 259 102 204 0 148 57 52 161 115 159 0H273L271 115L375 52L433 148L331 204L433 259L375 355L271 292L273 407Z"
        fill="currentColor"
      />
    </svg>
  );
}
