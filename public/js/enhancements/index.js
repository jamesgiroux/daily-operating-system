/**
 * DailyOS Enhancements Index
 * Aggregates all enhancement modules with error boundaries
 *
 * Provides a unified interface for applying all page enhancements
 * with graceful error handling to prevent individual failures
 * from breaking the entire page.
 *
 * @module enhancements/index
 * @requires EnhanceCheckboxes
 * @requires EnhanceTables
 * @requires EnhanceLinks
 * @requires EnhanceCodeBlocks
 * @requires EnhanceBlockquotes
 */

const Enhancements = {
  /**
   * Apply checkbox enhancements with error handling
   * @param {Element} container
   */
  checkboxes(container) {
    try {
      if (window.EnhanceCheckboxes) {
        EnhanceCheckboxes.apply(container);
      }
    } catch (e) {
      console.error('[DailyOS] Checkbox enhancement failed:', e);
    }
  },

  /**
   * Apply table enhancements with error handling
   * @param {Element} container
   */
  tables(container) {
    try {
      if (window.EnhanceTables) {
        EnhanceTables.apply(container);
      }
    } catch (e) {
      console.error('[DailyOS] Table enhancement failed:', e);
    }
  },

  /**
   * Apply link enhancements with error handling
   * @param {Element} container
   */
  links(container) {
    try {
      if (window.EnhanceLinks) {
        EnhanceLinks.apply(container);
      }
    } catch (e) {
      console.error('[DailyOS] Link enhancement failed:', e);
    }
  },

  /**
   * Apply code block enhancements with error handling
   * @param {Element} container
   */
  codeBlocks(container) {
    try {
      if (window.EnhanceCodeBlocks) {
        EnhanceCodeBlocks.apply(container);
      }
    } catch (e) {
      console.error('[DailyOS] Code block enhancement failed:', e);
    }
  },

  /**
   * Apply blockquote enhancements with error handling
   * @param {Element} container
   */
  blockquotes(container) {
    try {
      if (window.EnhanceBlockquotes) {
        EnhanceBlockquotes.apply(container);
      }
    } catch (e) {
      console.error('[DailyOS] Blockquote enhancement failed:', e);
    }
  },

  /**
   * Apply all default enhancements
   * @param {Element} container
   */
  applyAll(container) {
    this.checkboxes(container);
    this.tables(container);
    this.links(container);
    this.codeBlocks(container);
    this.blockquotes(container);
  }
};

// Make available globally
window.Enhancements = Enhancements;
