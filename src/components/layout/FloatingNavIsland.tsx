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

import React from 'react';
import { BrandMark } from '../ui/BrandMark';
import {
  Calendar,
  Mail,
  Inbox,
  CheckSquare2,
  Users,
  Building2,
  FolderKanban,
  Settings,
} from 'lucide-react';
import { capitalize } from '@/lib/utils';
import { smoothScrollTo } from '@/lib/smooth-scroll';
import styles from './FloatingNavIsland.module.css';

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
  activePage?: 'today' | 'week' | 'emails' | 'dropbox' | 'actions' | 'people' | 'accounts' | 'projects' | 'settings';

  /**
   * Color of active state indicator
   * Default: 'turmeric'
   */
  activeColor?: 'turmeric' | 'terracotta' | 'larkspur' | 'olive';

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
  id: 'week' | 'emails' | 'dropbox' | 'actions' | 'people' | 'accounts' | 'projects' | 'settings';
  label: string;
  icon: React.ReactNode;
  group: 'main' | 'entity' | 'admin';
}

export const FloatingNavIsland: React.FC<FloatingNavIslandProps> = ({
  mode = 'app',
  activePage = 'today',
  activeColor = 'turmeric',
  onNavigate,
  onHome,
  chapters,
  activeChapterId,
  onChapterClick,
}) => {
  const activeClass = styles[`active${capitalize(activeColor)}`] || '';

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
  const items: NavItem[] = [
    { id: 'week', label: 'This Week', icon: <Calendar size={18} strokeWidth={1.8} />, group: 'main' },
    { id: 'emails', label: 'Mail', icon: <Mail size={18} strokeWidth={1.8} />, group: 'main' },
    { id: 'dropbox', label: 'Dropbox', icon: <Inbox size={18} strokeWidth={1.8} />, group: 'entity' },
    { id: 'actions', label: 'Actions', icon: <CheckSquare2 size={18} strokeWidth={1.8} />, group: 'entity' },
    { id: 'people', label: 'People', icon: <Users size={18} strokeWidth={1.8} />, group: 'entity' },
    { id: 'accounts', label: 'Accounts', icon: <Building2 size={18} strokeWidth={1.8} />, group: 'entity' },
    { id: 'projects', label: 'Projects', icon: <FolderKanban size={18} strokeWidth={1.8} />, group: 'entity' },
    { id: 'settings', label: 'Settings', icon: <Settings size={18} strokeWidth={1.8} />, group: 'admin' },
  ];

  const isItemActive = (itemId: string) => itemId === activePage;

  return (
    <nav className={`${styles.navIsland} ${styles[`color${capitalize(activeColor)}`] || ''}`}>
      {/* Home / Today button — Brand mark */}
      <button
        className={`${styles.navIslandMark} ${activePage === 'today' ? styles.navIslandMarkActive : ''}`}
        onClick={onHome}
        aria-label="Today"
        title="Today"
      >
        <BrandMark size={16} />
      </button>

      {/* Main navigation items */}
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

      {/* Entity navigation items */}
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
          </button>
        ))}

      <div className={styles.navIslandDivider} aria-hidden="true" />

      {/* Admin navigation items */}
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
