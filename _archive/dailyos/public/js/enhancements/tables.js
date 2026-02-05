/**
 * DailyOS Table Enhancement
 * Wraps tables for horizontal scrolling
 *
 * Automatically wraps markdown tables in scrollable
 * containers to prevent layout overflow on mobile.
 *
 * @module enhancements/tables
 */

const EnhanceTables = {
  /**
   * Wrap tables in scrollable containers
   * @param {Element} container
   */
  apply(container) {
    const tables = container.querySelectorAll('table');
    tables.forEach(table => {
      // Skip if already wrapped
      if (table.parentElement.classList.contains('table-wrapper')) return;

      const wrapper = document.createElement('div');
      wrapper.className = 'table-wrapper';
      wrapper.style.overflowX = 'auto';
      wrapper.style.marginBottom = 'var(--space-6)';

      table.parentNode.insertBefore(wrapper, table);
      wrapper.appendChild(table);
    });
  }
};

// Make available globally
window.EnhanceTables = EnhanceTables;
