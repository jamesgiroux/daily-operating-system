/**
 * FinisMarker — three spaced asterisks + enrichment timestamp.
 * Mockup: Montserrat 18px, turmeric color, 0.4em letter-spacing.
 * Marks the end of an editorial briefing. "When you've read it, you're briefed."
 */
import { BrandMark } from '../ui/BrandMark';
import s from './FinisMarker.module.css';

interface FinisMarkerProps {
  enrichedAt?: string;
}

export function FinisMarker({ enrichedAt }: FinisMarkerProps) {
  return (
    <div className={s.root}>
      <div className={s.marks}>
        <BrandMark size={18} />
        <BrandMark size={18} />
        <BrandMark size={18} />
      </div>
      {enrichedAt && (
        <div className={s.timestamp}>
          Last updated: {enrichedAt}
        </div>
      )}
    </div>
  );
}
