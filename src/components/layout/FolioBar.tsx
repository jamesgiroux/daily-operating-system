/**
 * FolioBar.tsx
 *
 * Editorial masthead. Fixed top bar with brand identity, publication label,
 * date, readiness stats, search trigger.
 *
 * Props allow page-specific customization. Search button is clickable (onSearchClick).
 */

import React from 'react';
import { Link } from '@tanstack/react-router';
import { BrandMark } from '../ui/BrandMark';
import { capitalize } from '@/lib/utils';
import styles from './FolioBar.module.css';

export interface ReadinessStat {
  label: string;
  color: 'sage' | 'terracotta';
}

export interface FolioBarProps {
  /**
   * Publication label, e.g., "Daily Briefing", "Account", "Actions"
   * Default: "Daily Briefing"
   */
  publicationLabel?: string;

  /**
   * Date text, e.g., "Thu, Feb 14, 2026 · Briefed 6:00a"
   * Optional — can be empty if not needed
   */
  dateText?: string;

  /**
   * Readiness stats with color indicators
   * e.g., [{ label: '4/6 prepped', color: 'sage' }, { label: '2 overdue', color: 'terracotta' }]
   */
  readinessStats?: ReadinessStat[];

  /**
   * Status text, e.g., ">_ ready"
   * Optional
   */
  statusText?: string;

  /**
   * Callback when search button is clicked
   */
  onSearchClick?: () => void;

  /**
   * Back link — replaces publication label with a navigation link.
   * Used on detail pages to navigate back to list pages.
   */
  backLink?: { label: string; onClick: () => void };

  /**
   * Actions slot — rendered in right section before search button.
   * Used for page-specific actions like enrichment buttons.
   */
  actions?: React.ReactNode;
}

export const FolioBar: React.FC<FolioBarProps> = ({
  publicationLabel = 'Daily Briefing',
  dateText,
  readinessStats,
  statusText,
  onSearchClick,
  backLink,
  actions,
}) => {
  return (
    <header className={styles.folio}>
      {/* LEFT: Back link OR Brand mark + Publication label */}
      <div className={styles.folioLeft}>
        {backLink ? (
          <button onClick={backLink.onClick} className={styles.folioBackLink}>
            <span className={styles.folioBackArrow}>&#8592;</span>
            {backLink.label}
          </button>
        ) : (
          <>
            <Link to="/" className={styles.folioHomeLink}>
              <BrandMark className={styles.folioMark} size={18} />
            </Link>
            <span className={styles.folioPub}>{publicationLabel}</span>
          </>
        )}
      </div>

      {/* CENTER: Date (absolutely positioned) */}
      {dateText && <div className={styles.folioCenter}>{dateText}</div>}

      {/* RIGHT: Readiness stats, status, actions, search */}
      <div className={styles.folioRight}>
        {/* Readiness stats with colored dots */}
        {readinessStats && readinessStats.length > 0 && (
          <div className={styles.folioReadiness}>
            {readinessStats.map((stat, idx) => (
              <span
                key={idx}
                className={`${styles.folioStat} ${styles[`folioStat${capitalize(stat.color)}`] || ''}`}
              >
                <span className={`${styles.folioDot} ${styles[`folioDot${capitalize(stat.color)}`] || ''}`} />
                {stat.label}
              </span>
            ))}
          </div>
        )}

        {/* Status text (mono font) */}
        {statusText && <span className={styles.folioStatus}>{statusText}</span>}

        {/* Page-specific actions */}
        {actions && <div className={styles.folioActions}>{actions}</div>}

        {/* Search button (clickable) */}
        <button
          className={styles.folioSearch}
          onClick={onSearchClick}
          aria-label="Open search (⌘K)"
          title="Open search"
        >
          ⌘K
        </button>
      </div>
    </header>
  );
};

export default FolioBar;
