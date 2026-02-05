/**
 * DailyOS Checkbox Enhancement
 * Makes markdown checkboxes interactive with state persistence
 *
 * Features:
 * - Enables disabled checkboxes from markdown
 * - Persists state to localStorage per-page
 * - Visual strikethrough on completion
 *
 * @module enhancements/checkboxes
 */

const EnhanceCheckboxes = {
  /**
   * Enhance all checkboxes in container with interactivity
   * @param {Element} container
   */
  apply(container) {
    const checkboxes = container.querySelectorAll('input[type="checkbox"]');
    checkboxes.forEach((checkbox, index) => {
      // Make interactive
      checkbox.disabled = false;
      checkbox.dataset.index = index;

      // Add change listener
      checkbox.addEventListener('change', (e) => {
        const li = e.target.closest('li');
        if (li) {
          const textSpan = li.querySelector('.task-text') || li;
          if (e.target.checked) {
            textSpan.style.textDecoration = 'line-through';
            textSpan.style.color = 'var(--text-muted)';
          } else {
            textSpan.style.textDecoration = 'none';
            textSpan.style.color = '';
          }
        }
        // Store state in localStorage
        this.saveState(window.location.pathname, index, e.target.checked);
      });

      // Restore saved state
      const savedState = this.getState(window.location.pathname, index);
      if (savedState !== null) {
        checkbox.checked = savedState;
        if (savedState) {
          const li = checkbox.closest('li');
          if (li) {
            const textSpan = li.querySelector('.task-text') || li;
            textSpan.style.textDecoration = 'line-through';
            textSpan.style.color = 'var(--text-muted)';
          }
        }
      }
    });
  },

  /**
   * Save checkbox state to localStorage
   * @param {string} path
   * @param {number} index
   * @param {boolean} checked
   */
  saveState(path, index, checked) {
    const normalizedPath = this.normalizePath(path);
    const key = `dailyos-checkbox-${normalizedPath}`;
    const states = JSON.parse(localStorage.getItem(key) || '{}');
    states[index] = checked;
    localStorage.setItem(key, JSON.stringify(states));
  },

  /**
   * Get checkbox state from localStorage
   * @param {string} path
   * @param {number} index
   * @returns {boolean|null}
   */
  getState(path, index) {
    const normalizedPath = this.normalizePath(path);
    const key = `dailyos-checkbox-${normalizedPath}`;
    const states = JSON.parse(localStorage.getItem(key) || '{}');
    return states[index] !== undefined ? states[index] : null;
  },

  /**
   * Normalize path for consistent localStorage keys
   * Removes trailing slashes to prevent /path and /path/ being different keys
   * @param {string} path
   * @returns {string}
   */
  normalizePath(path) {
    return path.replace(/\/+$/, '') || '/';
  },

  /**
   * Clear checkbox states for a path
   * @param {string} path
   */
  clearStates(path) {
    const normalizedPath = this.normalizePath(path);
    const key = `dailyos-checkbox-${normalizedPath}`;
    localStorage.removeItem(key);
  }
};

// Make available globally
window.EnhanceCheckboxes = EnhanceCheckboxes;
