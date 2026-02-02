/**
 * DailyOS Constants
 * Centralized configuration and magic strings
 *
 * Provides centralized access to all magic strings, icons,
 * and configuration values used throughout the DailyOS application.
 *
 * Categories:
 * - RINGS: Lifecycle ring classifications
 * - ANIMATION: Timing values for animations
 * - PRIORITY: Task priority levels
 * - ICONS: SVG icon strings
 * - MEETING_TYPES: Meeting categorization
 *
 * @module constants
 */

const Constants = {
  RINGS: {
    SUMMIT: 'summit',
    INFLUENCE: 'influence',
    EVOLUTION: 'evolution',
    FOUNDATION: 'foundation',
    PROJECT: 'project'
  },

  ANIMATION: {
    BASE_DELAY: 0.1,
    STAGGER: 0.05
  },

  PRIORITY: {
    P1: 'p1',
    P2: 'p2',
    P3: 'p3'
  },

  ICONS: {
    COPY: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
      <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
    </svg>`,
    CHECK: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
      <polyline points="20 6 9 17 4 12"></polyline>
    </svg>`,
    WARNING: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="20" height="20">
      <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
      <line x1="12" y1="9" x2="12" y2="13"></line>
      <line x1="12" y1="17" x2="12.01" y2="17"></line>
    </svg>`,
    SUCCESS: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="20" height="20">
      <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
      <polyline points="22 4 12 14.01 9 11.01"></polyline>
    </svg>`,
    ERROR: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="20" height="20">
      <circle cx="12" cy="12" r="10"></circle>
      <line x1="15" y1="9" x2="9" y2="15"></line>
      <line x1="9" y1="9" x2="15" y2="15"></line>
    </svg>`
  },

  MEETING_TYPES: {
    CUSTOMER: 'customer',
    INTERNAL: 'internal',
    PROJECT: 'project',
    PERSONAL: 'personal'
  }
};

// Make available globally
window.Constants = Constants;
