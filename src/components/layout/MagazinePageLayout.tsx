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

import React, { useMemo, useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import styles from './MagazinePageLayout.module.css';
import AtmosphereLayer from './AtmosphereLayer';
import FolioBar from './FolioBar';
import FloatingNavIsland from './FloatingNavIsland';
import { UpdateBanner } from '@/components/notifications/UpdateBanner';
import { useMagazineShellConfig, useFolioVolatile } from '@/hooks/useMagazineShell';
import { useChapterObserver } from '@/hooks/useChapterObserver';
import { useTauriEvent } from '@/hooks/useTauriEvent';
import { useAppState } from '@/hooks/useAppState';

export interface MagazinePageLayoutProps {
  /** Main page content */
  children: React.ReactNode;

  /** Callback when folio search button is clicked */
  onFolioSearch?: () => void;

  /** Callback when nav item is clicked */
  onNavigate?: (page: string) => void;

  /** Callback when nav home (asterisk) is clicked */
  onNavHome?: () => void;

  /** Callback when "What's New" is clicked in the update banner */
  onWhatsNew?: () => void;
}

export const MagazinePageLayout: React.FC<MagazinePageLayoutProps> = ({
  children,
  onFolioSearch,
  onNavigate,
  onNavHome,
  onWhatsNew,
}) => {
  // Page-specific config registered via useRegisterMagazineShell()
  const pageConfig = useMagazineShellConfig();

  // Entity mode from app config — controls nav ordering (accounts vs projects first).
  // Fetched once on mount and invalidated via config-updated event so preset changes
  // in Settings take effect immediately (I389 acceptance criterion 4) without
  // firing an IPC call on every page navigation.
  const [entityMode, setEntityMode] = useState<'account' | 'project' | 'both'>('account');
  const activePage = pageConfig?.activePage ?? 'today';
  const configCacheRef = useRef<{ entityMode: 'account' | 'project' | 'both' } | null>(null);

  const fetchConfig = React.useCallback(() => {
    invoke<{ entityMode?: string }>('get_config')
      .then((c) => {
        const mode = c.entityMode;
        if (mode === 'account' || mode === 'project' || mode === 'both') {
          configCacheRef.current = { entityMode: mode };
          setEntityMode(mode);
        }
      })
      .catch(() => { /* fallback to default 'account' */ });
  }, []);

  useEffect(() => {
    if (configCacheRef.current) {
      setEntityMode(configCacheRef.current.entityMode);
      return;
    }
    fetchConfig();
  }, [fetchConfig]);

  // Invalidate cache and re-fetch when config changes (e.g. Settings page).
  // Wrapped in useCallback so useTauriEvent doesn't re-subscribe on every render.
  const onConfigUpdated = React.useCallback(() => {
    configCacheRef.current = null;
    fetchConfig();
  }, [fetchConfig]);
  useTauriEvent('config-updated', onConfigUpdated);

  const atmosphereColor = pageConfig?.atmosphereColor ?? 'turmeric';
  const folioLabel = pageConfig?.folioLabel ?? 'Daily Briefing';
  const backLink = pageConfig?.backLink;
  const chapters = pageConfig?.chapters;
  // I563: Read volatile folio state from ref — falls back to config for backwards compat.
  const volatile = useFolioVolatile();
  const folioActions = volatile.folioActions ?? pageConfig?.folioActions;
  const { appState, clearDemo } = useAppState();

  // Demo mode badge — renders in folio bar actions slot
  const demoBadge = appState.demoModeActive ? (
    <button
      onClick={clearDemo}
      style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 10,
        fontWeight: 500,
        textTransform: 'uppercase',
        letterSpacing: '0.08em',
        color: 'var(--color-spice-terracotta)',
        background: 'none',
        border: 'none',
        cursor: 'pointer',
        padding: '2px 0',
        whiteSpace: 'nowrap',
      }}
    >
      DEMO &middot; Connect real data &rarr;
    </button>
  ) : null;

  const combinedActions = demoBadge || folioActions ? (
    <>
      {demoBadge}
      {folioActions}
    </>
  ) : undefined;

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
        readinessStats={volatile.folioReadinessStats ?? pageConfig?.folioReadinessStats}
        statusText={volatile.folioStatusText ?? pageConfig?.folioStatusText}
        onSearchClick={onFolioSearch}
        backLink={backLink}
        actions={combinedActions}
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
        {onWhatsNew && <UpdateBanner onWhatsNew={onWhatsNew} />}
        {children}
      </main>
    </div>
  );
};

export default MagazinePageLayout;
