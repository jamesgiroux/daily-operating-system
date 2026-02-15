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
import {
  Grid3x3,
  Calendar,
  Inbox,
  CheckSquare2,
  Users,
  Building2,
  Settings,
} from 'lucide-react';
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
  activePage?: 'today' | 'week' | 'inbox' | 'actions' | 'people' | 'accounts' | 'settings';

  /**
   * Color of active state indicator
   * Default: 'turmeric'
   */
  activeColor?: 'turmeric' | 'terracotta' | 'larkspur';

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
}

interface NavItem {
  id: 'today' | 'week' | 'inbox' | 'actions' | 'people' | 'accounts' | 'settings';
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
}) => {
  const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);
  const activeClass = styles[`active${cap(activeColor)}`] || '';

  // Chapter mode — icon-based scroll navigation (same visual style as app mode)
  if (mode === 'chapters' && chapters && chapters.length > 0) {
    return (
      <nav className={`${styles.navIsland} ${styles[`color${cap(activeColor)}`] || ''}`}>
        {/* Home button — Brand mark */}
        <button
          className={styles.navIslandMark}
          onClick={onHome}
          aria-label="Home"
          title="Home"
        >
          *
        </button>

        <div className={styles.navIslandDivider} aria-hidden="true" />

        {chapters.map((chapter) => {
          const isActive = chapter.id === activeChapterId;
          return (
            <button
              key={chapter.id}
              className={`${styles.navIslandItem} ${isActive ? activeClass : ''}`}
              data-label={chapter.label}
              onClick={() => smoothScrollTo(chapter.id)}
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
    { id: 'today', label: 'Today', icon: <Grid3x3 size={18} strokeWidth={1.8} />, group: 'main' },
    { id: 'week', label: 'This Week', icon: <Calendar size={18} strokeWidth={1.8} />, group: 'main' },
    { id: 'inbox', label: 'Inbox', icon: <Inbox size={18} strokeWidth={1.8} />, group: 'main' },
    { id: 'actions', label: 'Actions', icon: <CheckSquare2 size={18} strokeWidth={1.8} />, group: 'entity' },
    { id: 'people', label: 'People', icon: <Users size={18} strokeWidth={1.8} />, group: 'entity' },
    { id: 'accounts', label: 'Accounts', icon: <Building2 size={18} strokeWidth={1.8} />, group: 'entity' },
    { id: 'settings', label: 'Settings', icon: <Settings size={18} strokeWidth={1.8} />, group: 'admin' },
  ];

  const isItemActive = (itemId: string) => itemId === activePage;

  return (
    <nav className={`${styles.navIsland} ${styles[`color${cap(activeColor)}`] || ''}`}>
      {/* Home button — Brand mark */}
      <button
        className={styles.navIslandMark}
        onClick={onHome}
        aria-label="Home"
        title="Home"
      >
        *
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
