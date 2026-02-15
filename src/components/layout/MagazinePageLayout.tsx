/**
 * MagazinePageLayout.tsx
 *
 * Wrapper component combining FolioBar, FloatingNavIsland, AtmosphereLayer,
 * and page container. Provides the complete magazine-layout editorial shell
 * for any page using the new design system.
 *
 * Usage:
 *   @example
 *   &lt;MagazinePageLayout
 *     folioLabel="Daily Briefing"
 *     folioDate="Thu, Feb 14, 2026"
 *     activePage="today"
 *     atmosphereColor="turmeric"
 *     heroSection={heroContent}
 *   &gt;
 *     [Page content sections]
 *   &lt;/MagazinePageLayout&gt;
 */

import React from 'react';
import styles from './MagazinePageLayout.module.css';
import AtmosphereLayer from './AtmosphereLayer';
import FolioBar, { ReadinessStat } from './FolioBar';
import FloatingNavIsland, { ChapterItem } from './FloatingNavIsland';

export interface MagazinePageLayoutProps {
  /**
   * Hero section — rendered at top of page, above main content
   */
  heroSection: React.ReactNode;

  /**
   * Main page content — rendered below hero section
   */
  children: React.ReactNode;

  /**
   * Atmosphere color scheme
   * Default: 'turmeric'
   */
  atmosphereColor?: 'turmeric' | 'terracotta' | 'larkspur';

  /**
   * Currently active page for nav highlighting
   * Default: 'today'
   */
  activePage?: 'today' | 'week' | 'inbox' | 'actions' | 'people' | 'accounts' | 'settings';

  /**
   * Publication label for folio bar, e.g., "Daily Briefing"
   * Default: "Daily Briefing"
   */
  folioLabel?: string;

  /**
   * Date text for folio bar, e.g., "Thu, Feb 14, 2026 · Briefed 6:00a"
   * Optional
   */
  folioDate?: string;

  /**
   * Readiness stats for folio bar
   */
  readinessStats?: ReadinessStat[];

  /**
   * Status text for folio bar, e.g., ">_ ready"
   * Optional
   */
  statusText?: string;

  /**
   * Callback when folio search button is clicked
   */
  onFolioSearch?: () => void;

  /**
   * Callback when nav item is clicked
   */
  onNavigate?: (page: string) => void;

  /**
   * Callback when nav home (asterisk) is clicked
   */
  onNavHome?: () => void;

  /**
   * Back link for detail pages — replaces publication label in FolioBar
   */
  backLink?: { label: string; href: string };

  /**
   * Chapter definitions — when provided, FloatingNavIsland switches to chapter mode
   * with text-based scroll navigation instead of icon-based page navigation
   */
  chapters?: ChapterItem[];

  /**
   * Currently active chapter ID (from IntersectionObserver)
   */
  activeChapterId?: string;
}

export const MagazinePageLayout: React.FC<MagazinePageLayoutProps> = ({
  heroSection,
  children,
  atmosphereColor = 'turmeric',
  activePage = 'today',
  folioLabel = 'Daily Briefing',
  folioDate,
  readinessStats,
  statusText,
  onFolioSearch,
  onNavigate,
  onNavHome,
  backLink,
  chapters,
  activeChapterId,
}) => {
  return (
    <div className={styles.magazinePage}>
      {/* Atmospheric background wash */}
      <AtmosphereLayer color={atmosphereColor} />

      {/* Fixed folio bar — top masthead */}
      <FolioBar
        publicationLabel={folioLabel}
        dateText={folioDate}
        readinessStats={readinessStats}
        statusText={statusText}
        onSearchClick={onFolioSearch}
        backLink={backLink}
      />

      {/* Fixed floating nav island — right margin */}
      <FloatingNavIsland
        mode={chapters && chapters.length > 0 ? 'chapters' : 'app'}
        activePage={activePage}
        activeColor={atmosphereColor}
        onNavigate={onNavigate}
        onHome={onNavHome}
        chapters={chapters}
        activeChapterId={activeChapterId}
      />

      {/* Main page container — content above atmosphere layer */}
      <main className={styles.pageContainer}>
        {/* Hero section — usually a headline + narrative */}
        <section className={styles.heroSection}>{heroSection}</section>

        {/* Page content sections */}
        {children}
      </main>
    </div>
  );
};

export default MagazinePageLayout;
