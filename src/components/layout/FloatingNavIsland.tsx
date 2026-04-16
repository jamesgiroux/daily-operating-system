/**
 * FloatingNavIsland.tsx
 *
 * Right-margin floating navigation toolbar — dual-pill "Dynamic Island" design.
 *
 * Two merged pills displayed simultaneously:
 * - Global pill (right): Always-visible icon-based page navigation
 * - Local pill (left): Chapter/section scroll navigation, appears when chapters exist
 *
 * The pills merge visually where they overlap — shared edge loses border-radius.
 * Local pill aligns vertically so its top matches the active icon in the global pill.
 *
 * When no `onNavigate` is provided (e.g. OnboardingFlow), only chapters render
 * in a single pill — backwards-compatible with chapter-only usage.
 */

import React, { useState, useEffect, useRef, useCallback, useLayoutEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { BrandMark } from '../ui/BrandMark';
import {
  Calendar,
  Mail,
  Inbox,
  CheckSquare2,
  UserCircle,
  Users,
  Building2,
  FolderKanban,
  Settings,
} from 'lucide-react';
import { capitalize } from '@/lib/utils';
import { smoothScrollTo } from '@/lib/smooth-scroll';
import styles from './FloatingNavIsland.module.css';
import type { UserEntity } from '@/types';

export interface ChapterItem {
  id: string;
  label: string;
  icon: React.ReactNode;
}

export interface FloatingNavIslandProps {
  /**
   * Currently active page for visual highlighting (global pill)
   * Default: 'today'
   */
  activePage?: 'today' | 'week' | 'emails' | 'dropbox' | 'actions' | 'me' | 'people' | 'accounts' | 'projects' | 'settings';

  /**
   * Color of active state indicator
   * Default: 'turmeric'
   */
  activeColor?: 'turmeric' | 'terracotta' | 'larkspur' | 'olive' | 'eucalyptus';

  /**
   * Entity mode from active role preset.
   * Controls whether 'accounts' or 'projects' appears first in entity nav.
   * 'account' = accounts first, 'project' = projects first, 'both' = default order
   */
  entityMode?: 'account' | 'project' | 'both';

  /**
   * Callback when nav item is clicked (global pill).
   * When absent, the global pill is hidden (chapter-only mode for onboarding).
   */
  onNavigate?: (page: string) => void;

  /**
   * Callback when home (asterisk mark) is clicked
   */
  onHome?: () => void;

  /**
   * Chapter definitions for local pill
   */
  chapters?: ChapterItem[];

  /**
   * Currently active chapter ID
   */
  activeChapterId?: string;

  /**
   * Callback when a chapter is clicked — sets active state immediately
   */
  onChapterClick?: (id: string) => void;

  /**
   * @deprecated Use presence/absence of `onNavigate` to control global pill visibility.
   * Kept temporarily for backwards compatibility — ignored internally.
   */
  mode?: 'app' | 'chapters';
}

interface NavItem {
  id: 'week' | 'emails' | 'dropbox' | 'actions' | 'me' | 'people' | 'accounts' | 'projects' | 'settings';
  label: string;
  icon: React.ReactNode;
  group: 'main' | 'work' | 'entity' | 'admin';
}

export const FloatingNavIsland: React.FC<FloatingNavIslandProps> = ({
  activePage = 'today',
  activeColor = 'turmeric',
  entityMode,
  onNavigate,
  onHome,
  chapters,
  activeChapterId,
  onChapterClick,
}) => {
  const activeClass = styles[`active${capitalize(activeColor)}`] || '';
  const hasGlobalPill = !!onNavigate;
  const hasChapters = !!(chapters && chapters.length > 0);

  // ─── Me content dot ───────────────────────────────────────────────────
  const [meNeedsContent, setMeNeedsContent] = useState(true);
  useEffect(() => {
    if (!hasGlobalPill) return;
    invoke<UserEntity>('get_user_entity')
      .then((entity) => {
        const hasContent = !!(
          entity.name || entity.company || entity.title ||
          entity.valueProposition || entity.successDefinition ||
          entity.productContext || entity.companyBio || entity.roleDescription ||
          entity.howImMeasured || entity.pricingModel || entity.competitiveContext ||
          entity.differentiators || entity.objections ||
          entity.annualPriorities || entity.quarterlyPriorities ||
          (entity.playbooks && entity.playbooks !== '{}')
        );
        setMeNeedsContent(!hasContent);
      })
      .catch(() => { /* user entity not available */ });
  }, [hasGlobalPill]);

  // ─── Active item ref for chapter pill Y alignment ─────────────────────
  const globalPillRef = useRef<HTMLElement>(null);
  const activeItemRef = useRef<HTMLButtonElement | null>(null);
  const [localPillTop, setLocalPillTop] = useState(0);

  // Callback ref: assigned to whichever global nav button is active
  const setActiveRef = useCallback((node: HTMLButtonElement | null) => {
    activeItemRef.current = node;
  }, []);

  // Compute the local pill's top position relative to the global pill
  useLayoutEffect(() => {
    if (!hasChapters || !hasGlobalPill || !activeItemRef.current || !globalPillRef.current) {
      setLocalPillTop(0);
      return;
    }
    const containerRect = globalPillRef.current.getBoundingClientRect();
    const activeRect = activeItemRef.current.getBoundingClientRect();
    // Align top of local pill with the active icon's top, with pill padding offset
    const offset = activeRect.top - containerRect.top - 8; // subtract local pill's own padding
    setLocalPillTop(Math.max(0, offset));
  }, [hasChapters, hasGlobalPill, activePage, activeChapterId]);

  // ─── Chapter-only mode (OnboardingFlow) ───────────────────────────────
  if (!hasGlobalPill && hasChapters) {
    return (
      <nav className={`${styles.navIslandGlobal} ${styles[`color${capitalize(activeColor)}`] || ''}`}>
        {/* Home button — Brand mark */}
        <button
          className={styles.navIslandMark}
          data-label="Today"
          onClick={onHome}
          aria-label="Today"
          title="Today"
        >
          <BrandMark size={16} />
        </button>

        <div className={styles.navIslandDivider} aria-hidden="true" />

        {chapters!.map((chapter) => {
          const isActive = chapter.id === activeChapterId;
          return (
            <button
              key={chapter.id}
              className={`${styles.navIslandItem} ${isActive ? activeClass : ''}`}
              data-label={chapter.label}
              onClick={() => {
                onChapterClick?.(chapter.id);
                smoothScrollTo(chapter.id);
              }}
              aria-label={chapter.label}
              title={chapter.label}
            >
              {chapter.icon}
            </button>
          );
        })}
      </nav>
    );
  }

  // ─── Nav items ────────────────────────────────────────────────────────
  const accountsItem: NavItem = { id: 'accounts', label: 'Accounts', icon: <Building2 size={18} strokeWidth={1.8} />, group: 'entity' };
  const projectsItem: NavItem = { id: 'projects', label: 'Projects', icon: <FolderKanban size={18} strokeWidth={1.8} />, group: 'entity' };
  const entityPair = entityMode === 'project' ? [projectsItem, accountsItem] : [accountsItem, projectsItem];

  const items: NavItem[] = [
    // Time — schedule views
    { id: 'week', label: 'This Week', icon: <Calendar size={18} strokeWidth={1.8} />, group: 'main' },
    // Work — mail + actions
    { id: 'emails', label: 'Mail', icon: <Mail size={18} strokeWidth={1.8} />, group: 'work' },
    { id: 'actions', label: 'Actions', icon: <CheckSquare2 size={18} strokeWidth={1.8} />, group: 'work' },
    // Entities — me, people, accounts/projects
    { id: 'me', label: 'Me', icon: <UserCircle size={18} strokeWidth={1.8} />, group: 'entity' },
    { id: 'people', label: 'People', icon: <Users size={18} strokeWidth={1.8} />, group: 'entity' },
    ...entityPair.map(item => ({ ...item, group: 'entity' as const })),
    // Tools — inbox + settings
    { id: 'dropbox', label: 'Inbox', icon: <Inbox size={18} strokeWidth={1.8} />, group: 'admin' },
    { id: 'settings', label: 'Settings', icon: <Settings size={18} strokeWidth={1.8} />, group: 'admin' },
  ];

  const isItemActive = (itemId: string) => itemId === activePage;

  const renderNavButton = (item: NavItem) => {
    const active = isItemActive(item.id);
    return (
      <button
        key={item.id}
        ref={active ? setActiveRef : undefined}
        className={`${styles.navIslandItem} ${active ? activeClass : ''}`}
        data-label={item.label}
        onClick={() => onNavigate?.(item.id)}
        aria-label={item.label}
        title={item.label}
      >
        {item.icon}
        {item.id === 'me' && meNeedsContent && (
          <span className={styles.meContentDot} aria-hidden="true" />
        )}
      </button>
    );
  };

  // ─── Dual-pill render ─────────────────────────────────────────────────
  return (
    <div className={styles.navIslandContainer}>
      {/* LOCAL PILL — chapter/section navigation (left side) */}
      <nav
        className={`${styles.navIslandLocal} ${hasChapters ? '' : styles.navIslandLocalHidden}`}
        style={{ '--local-pill-top': `${localPillTop}px` } as React.CSSProperties}
        aria-label="Section navigation"
      >
        {hasChapters && chapters!.map((chapter) => {
          const isActive = chapter.id === activeChapterId;
          return (
            <button
              key={chapter.id}
              className={`${styles.navIslandLocalItem} ${isActive ? activeClass : ''}`}
              data-label={chapter.label}
              onClick={() => {
                onChapterClick?.(chapter.id);
                smoothScrollTo(chapter.id);
              }}
              aria-label={chapter.label}
              title={chapter.label}
            >
              {chapter.icon}
            </button>
          );
        })}
      </nav>

      {/* GLOBAL PILL — app page navigation (right side) */}
      <nav
        ref={globalPillRef}
        className={`${styles.navIslandGlobal} ${styles[`color${capitalize(activeColor)}`] || ''} ${hasChapters ? styles.navIslandGlobalMerged : ''}`}
        aria-label="App navigation"
      >
        {/* Home / Today button — Brand mark */}
        <button
          ref={activePage === 'today' ? setActiveRef : undefined}
          className={`${styles.navIslandMark} ${activePage === 'today' ? styles.navIslandMarkActive : ''}`}
          data-label="Today"
          onClick={onHome}
          aria-label="Today"
        >
          <BrandMark size={16} />
        </button>

        {/* Time — This Week */}
        {items
          .filter((item) => item.group === 'main')
          .map(renderNavButton)}

        <div className={styles.navIslandDivider} aria-hidden="true" />

        {/* Work — Mail, Actions */}
        {items
          .filter((item) => item.group === 'work')
          .map(renderNavButton)}

        <div className={styles.navIslandDivider} aria-hidden="true" />

        {/* Entities — Me, People, Accounts/Projects */}
        {items
          .filter((item) => item.group === 'entity')
          .map(renderNavButton)}

        <div className={styles.navIslandDivider} aria-hidden="true" />

        {/* Tools — Inbox, Settings */}
        {items
          .filter((item) => item.group === 'admin')
          .map(renderNavButton)}
      </nav>
    </div>
  );
};

export default FloatingNavIsland;
