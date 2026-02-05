/**
 * DailyOS Section Utilities
 * DOM traversal and section finding helpers
 *
 * Provides utilities for:
 * - Finding content between headings
 * - Table row extraction
 * - Tier and meeting type classification
 * - Section keyword matching
 *
 * @module utils/section-utils
 * @requires Constants
 */

const SectionUtils = {
  /**
   * Find content elements between heading and next h2/h1
   * @param {Element} heading - The heading element to start from
   * @param {boolean} includeHr - If true, include HR elements and continue past them
   * @returns {Element[]}
   */
  findContent(heading, includeHr = false) {
    const content = [];
    let sibling = heading.nextElementSibling;
    while (sibling && !sibling.matches('h2, h1')) {
      if (sibling.tagName === 'HR' && !includeHr) {
        break;
      }
      content.push(sibling);
      sibling = sibling.nextElementSibling;
    }
    return content;
  },

  /**
   * Find the next table element after a heading
   * @param {Element} heading - The heading element to start from
   * @returns {HTMLTableElement|null}
   */
  findNextTable(heading) {
    let sibling = heading.nextElementSibling;
    while (sibling) {
      if (sibling.tagName === 'TABLE') return sibling;
      if (sibling.querySelector('table')) return sibling.querySelector('table');
      if (sibling.matches('h2, h1')) break;
      sibling = sibling.nextElementSibling;
    }
    return null;
  },

  /**
   * Get table body rows, handling tables with or without tbody
   * @param {HTMLTableElement} table
   * @returns {HTMLTableRowElement[]}
   */
  getTableRows(table) {
    const tbodyRows = table.querySelectorAll('tbody tr');
    return tbodyRows.length > 0
      ? Array.from(tbodyRows)
      : Array.from(table.querySelectorAll('tr')).slice(1);
  },

  /**
   * Classify text into lifecycle tier
   * @param {string} text - Text to classify
   * @returns {string} Tier classification
   */
  classifyTier(text) {
    const lower = (text || '').toLowerCase();

    // Load tiers from config if available
    const tiers = window.DailyOSConfig?.lifecycle?.tiers || [
      { id: 'tier-1', label: 'Tier 1' },
      { id: 'tier-2', label: 'Tier 2' },
      { id: 'tier-3', label: 'Tier 3' },
      { id: 'tier-4', label: 'Tier 4' }
    ];

    for (const tier of tiers) {
      if (lower.includes(tier.label.toLowerCase()) || lower.includes(tier.id)) {
        return tier.id;
      }
    }

    if (lower.includes('project')) return 'project';
    return tiers.length > 0 ? tiers[tiers.length - 1].id : 'tier-4';
  },

  // Backwards compatibility
  classifyRing(text) { return this.classifyTier(text); },

  /**
   * Classify meeting type from text
   * @param {string} text - Text to classify
   * @returns {string} Meeting type
   */
  classifyMeetingType(text) {
    const lower = (text || '').toLowerCase();
    if (lower.includes('customer') || lower.includes('external') || lower.includes('client')) {
      return Constants.MEETING_TYPES.CUSTOMER;
    }
    if (lower.includes('project')) return Constants.MEETING_TYPES.PROJECT;
    if (lower.includes('personal')) return Constants.MEETING_TYPES.PERSONAL;
    return Constants.MEETING_TYPES.INTERNAL;
  },

  /**
   * Find sections in container by keyword matching
   * @param {Element} container - Container to search
   * @param {Object} keywords - Map of section names to keyword arrays
   * @returns {Object} Map of section names to heading elements
   */
  findSectionsByKeywords(container, keywords) {
    const sections = {};
    const h2s = container.querySelectorAll('h2');

    h2s.forEach(h2 => {
      const text = h2.textContent.toLowerCase();
      for (const [name, kws] of Object.entries(keywords)) {
        if (kws.some(kw => text.includes(kw))) {
          sections[name] = h2;
          break;
        }
      }
    });

    return sections;
  },

  /**
   * Extract count from heading text (e.g., "Items (5)" -> 5)
   * @param {Element} heading
   * @returns {number}
   */
  extractCountFromHeading(heading) {
    const match = heading.textContent.match(/\((\d+)\)/);
    return match ? parseInt(match[1]) : 0;
  },

  /**
   * Count child elements matching selector
   * @param {Element} heading - Section heading
   * @param {string} selector - CSS selector for items to count
   * @returns {number}
   */
  countSectionItems(heading, selector) {
    const content = this.findContent(heading);
    let count = 0;
    content.forEach(el => {
      if (el.matches && el.matches(selector)) count++;
      count += el.querySelectorAll ? el.querySelectorAll(selector).length : 0;
    });
    return count;
  }
};

// Make available globally
window.SectionUtils = SectionUtils;
