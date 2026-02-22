/**
 * MagazinePageLayout.tsx
 *
 * Wrapper component combining FolioBar, FloatingNavIsland, AtmosphereLayer,
 * and page container. Provides the complete magazine-layout editorial shell.
 *
 * Shell configuration comes from two sources:
 * 1. Props (router-level concerns: onFolioSearch, onNavigate, onNavHome)
 * 2. MagazineShellContext (page-level concerns: chapters, folioLabel, atmosphereColor)
 *
 * Pages register their config via useRegisterMagazineShell(). This inverts
 * the dependency so the router doesn't need to import page internals.
 */

import React, { useMemo, useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import styles from './MagazinePageLayout.module.css';
import AtmosphereLayer from './AtmosphereLayer';
import FolioBar from './FolioBar';
import FloatingNavIsland from './FloatingNavIsland';
import { useMagazineShellConfig } from '@/hooks/useMagazineShell';
import { useChapterObserver } from '@/hooks/useChapterObserver';

export interface MagazinePageLayoutProps {
  /** Main page content */
  children: React.ReactNode;

  /** Callback when folio search button is clicked */
  onFolioSearch?: () => void;

  /** Callback when nav item is clicked */
  onNavigate?: (page: string) => void;

  /** Callback when nav home (asterisk) is clicked */
  onNavHome?: () => void;
}

export const MagazinePageLayout: React.FC<MagazinePageLayoutProps> = ({
  children,
  onFolioSearch,
  onNavigate,
  onNavHome,
}) => {
  // Page-specific config registered via useRegisterMagazineShell()
  const pageConfig = useMagazineShellConfig();

  // Entity mode from app config — controls nav ordering (accounts vs projects first).
  // Re-fetched on every page navigation so preset changes in Settings take effect
  // immediately without a full app restart (I389 acceptance criterion 4).
  const [entityMode, setEntityMode] = useState<'account' | 'project' | 'both'>('account');
  const activePage = pageConfig?.activePage ?? 'today';
  useEffect(() => {
    invoke<{ entityMode?: string }>('get_config')
      .then((c) => {
        const mode = c.entityMode;
        if (mode === 'account' || mode === 'project' || mode === 'both') {
          setEntityMode(mode);
        }
      })
      .catch(() => { /* fallback to default 'account' */ });
  }, [activePage]);

  const atmosphereColor = pageConfig?.atmosphereColor ?? 'turmeric';
  const folioLabel = pageConfig?.folioLabel ?? 'Daily Briefing';
  const backLink = pageConfig?.backLink;
  const chapters = pageConfig?.chapters;
  const folioActions = pageConfig?.folioActions;

  // Chapter tracking — runs internally so pages don't need to manage it.
  // Memoize chapterIds so useChapterObserver doesn't reset active chapter on every render.
  const chapterIds = useMemo(() => chapters?.map((c) => c.id) ?? [], [chapters]);
  const [activeChapterId, setActiveChapterId] = useChapterObserver(chapterIds, chapterIds.length > 0);

  return (
    <div className={styles.magazinePage}>
      {/* Atmospheric background wash */}
      <AtmosphereLayer color={atmosphereColor} />

      {/* Fixed folio bar — top masthead */}
      <FolioBar
        publicationLabel={folioLabel}
        dateText={pageConfig?.folioDateText}
        readinessStats={pageConfig?.folioReadinessStats}
        statusText={pageConfig?.folioStatusText}
        onSearchClick={onFolioSearch}
        backLink={backLink}
        actions={folioActions}
      />

      {/* Fixed floating nav island — right margin */}
      <FloatingNavIsland
        mode={chapters && chapters.length > 0 ? 'chapters' : 'app'}
        activePage={activePage}
        activeColor={atmosphereColor}
        entityMode={entityMode}
        onNavigate={onNavigate}
        onHome={onNavHome}
        chapters={chapters}
        activeChapterId={activeChapterId}
        onChapterClick={setActiveChapterId}
      />

      {/* Main page container — content above atmosphere layer */}
      <main className={styles.pageContainer}>
        {children}
      </main>
    </div>
  );
};

export default MagazinePageLayout;
