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
import type { BackgroundWorkState } from '@/hooks/useBackgroundStatus';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import styles from './FolioBar.module.css';

export interface ReadinessStat {
  label: string;
  color: 'sage' | 'terracotta';
}

export interface FolioBreadcrumbItem {
  label: string;
  onClick?: () => void;
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
   * Persistent orientation trail rendered beside the brand mark.
   * Used on detail/report pages in place of a separate back affordance.
   */
  breadcrumbs?: FolioBreadcrumbItem[];

  /**
   * Actions slot — rendered in right section before search button.
   * Used for page-specific actions like enrichment buttons.
   */
  actions?: React.ReactNode;

  /**
   * Background work state — when active, the brand mark asterisk
   * pulses to indicate background intelligence processing.
   */
  backgroundWork?: BackgroundWorkState;

  /**
   * Mode badge — rendered between brand mark and publication label.
   * Used for LIVE/DEV mode indicator in debug builds.
   */
  modeBadge?: React.ReactNode;
}

export const FolioBar: React.FC<FolioBarProps> = ({
  publicationLabel = 'Daily Briefing',
  dateText,
  readinessStats,
  statusText,
  onSearchClick,
  breadcrumbs,
  actions,
  backgroundWork,
  modeBadge,
}) => {
  const markClass = backgroundWork?.phase === 'started'
    ? `${styles.folioMark} ${styles.folioMarkPulsing}`
    : styles.folioMark;
  return (
    <header className={styles.folio}>
      {/* LEFT: Brand mark + publication label or persistent breadcrumbs */}
      <div className={styles.folioLeft}>
        {backgroundWork?.phase === 'started' ? (
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Link to="/" className={styles.folioHomeLink}>
                  <BrandMark className={markClass} size={18} />
                </Link>
              </TooltipTrigger>
              <TooltipContent side="bottom" className="text-xs">
                {backgroundWork.message || 'Updating…'}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        ) : (
          <Link to="/" className={styles.folioHomeLink}>
            <BrandMark className={markClass} size={18} />
          </Link>
        )}
        {modeBadge}
        {breadcrumbs && breadcrumbs.length > 0 ? (
          <nav className={styles.folioBreadcrumbs} aria-label="Breadcrumb">
            {breadcrumbs.map((crumb, idx) => {
              const isLast = idx === breadcrumbs.length - 1;
              return (
                <React.Fragment key={`${crumb.label}-${idx}`}>
                  {idx > 0 && <span className={styles.folioBreadcrumbSeparator}>/</span>}
                  {crumb.onClick && !isLast ? (
                    <button
                      type="button"
                      onClick={crumb.onClick}
                      className={styles.folioBreadcrumbButton}
                    >
                      {crumb.label}
                    </button>
                  ) : (
                    <span
                      className={isLast ? styles.folioBreadcrumbCurrent : styles.folioBreadcrumbLabel}
                    >
                      {crumb.label}
                    </span>
                  )}
                </React.Fragment>
              );
            })}
          </nav>
        ) : (
          <span className={styles.folioPub}>{publicationLabel}</span>
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
