/**
 * FloatingNavIsland.tsx
 *
 * Right-margin floating navigation toolbar. Two modes:
 * - 'app' (default): Icon-based page navigation with tooltips
 * - 'chapters': Icon-based chapter navigation with scroll-to, tooltips, and active state
 *
 * Both modes use the same visual style: 36px icon buttons, data-label tooltips on hover,
 * brand mark at top. Chapter mode smooth-scrolls instead of navigating routes.
 */

import React, { useState, useEffect } from 'react';
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
   * Navigation mode.
   * 'app' = icon-based page navigation (default)
   * 'chapters' = icon-based chapter navigation with smooth scroll
   */
  mode?: 'app' | 'chapters';

  /**
   * Currently active page for visual highlighting (app mode)
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
   * Callback when nav item is clicked (app mode)
   */
  onNavigate?: (page: string) => void;

  /**
   * Callback when home (asterisk mark) is clicked
   */
  onHome?: () => void;

  /**
   * Chapter definitions for chapter mode
   */
  chapters?: ChapterItem[];

  /**
   * Currently active chapter ID (chapter mode)
   */
  activeChapterId?: string;

  /**
   * Callback when a chapter is clicked — sets active state immediately
   */
  onChapterClick?: (id: string) => void;
}

interface NavItem {
  id: 'week' | 'emails' | 'dropbox' | 'actions' | 'me' | 'people' | 'accounts' | 'projects' | 'settings';
  label: string;
  icon: React.ReactNode;
  group: 'main' | 'work' | 'entity' | 'admin';
}

export const FloatingNavIsland: React.FC<FloatingNavIslandProps> = ({
  mode = 'app',
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

  // Check if user entity is empty — drives dot indicator on Me nav item (prompt to fill in)
  const [meNeedsContent, setMeNeedsContent] = useState(true);
  useEffect(() => {
    if (mode !== 'app') return;
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
  }, [mode]);

  // Chapter mode — icon-based scroll navigation (same visual style as app mode)
  if (mode === 'chapters' && chapters && chapters.length > 0) {
    return (
      <nav className={`${styles.navIsland} ${styles[`color${capitalize(activeColor)}`] || ''}`}>
        {/* Home button — Brand mark */}
        <button
          className={styles.navIslandMark}
          onClick={onHome}
          aria-label="Home"
          title="Home"
        >
          <BrandMark size={16} />
        </button>

        <div className={styles.navIslandDivider} aria-hidden="true" />

        {chapters.map((chapter) => {
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

  // App mode — icon-based page navigation
  // Entity group ordering depends on entityMode: 'project' puts projects before accounts
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

  return (
    <nav className={`${styles.navIsland} ${styles[`color${capitalize(activeColor)}`] || ''}`}>
      {/* Home / Today button — Brand mark */}
      <button
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
        .map((item) => (
          <button
            key={item.id}
            className={`${styles.navIslandItem} ${isItemActive(item.id) ? activeClass : ''}`}
            data-label={item.label}
            onClick={() => onNavigate?.(item.id)}
            aria-label={item.label}
            title={item.label}
          >
            {item.icon}
          </button>
        ))}

      <div className={styles.navIslandDivider} aria-hidden="true" />

      {/* Work — Mail, Actions */}
      {items
        .filter((item) => item.group === 'work')
        .map((item) => (
          <button
            key={item.id}
            className={`${styles.navIslandItem} ${isItemActive(item.id) ? activeClass : ''}`}
            data-label={item.label}
            onClick={() => onNavigate?.(item.id)}
            aria-label={item.label}
            title={item.label}
          >
            {item.icon}
          </button>
        ))}

      <div className={styles.navIslandDivider} aria-hidden="true" />

      {/* Entities — Me, People, Accounts/Projects */}
      {items
        .filter((item) => item.group === 'entity')
        .map((item) => (
          <button
            key={item.id}
            className={`${styles.navIslandItem} ${isItemActive(item.id) ? activeClass : ''}`}
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
        ))}

      <div className={styles.navIslandDivider} aria-hidden="true" />

      {/* Tools — Inbox, Settings */}
      {items
        .filter((item) => item.group === 'admin')
        .map((item) => (
          <button
            key={item.id}
            className={`${styles.navIslandItem} ${isItemActive(item.id) ? activeClass : ''}`}
            data-label={item.label}
            onClick={() => onNavigate?.(item.id)}
            aria-label={item.label}
            title={item.label}
          >
            {item.icon}
          </button>
        ))}
    </nav>
  );
};

export default FloatingNavIsland;
