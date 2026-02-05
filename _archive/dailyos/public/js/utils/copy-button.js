/**
 * DailyOS Copy Button Utility
 * Reusable copy-to-clipboard button component
 *
 * @module utils/copy-button
 * @requires Constants
 */

const CopyButton = {
  /**
   * Create a copy button element
   * @param {Object} options - Button configuration
   * @param {string} options.top - Top position (default: '8px')
   * @param {string} options.right - Right position (default: '8px')
   * @returns {HTMLButtonElement}
   */
  create(options = {}) {
    const { top = '8px', right = '8px' } = options;
    const btn = document.createElement('button');
    btn.className = 'copy-btn btn btn-ghost btn-icon';
    btn.innerHTML = Constants.ICONS.COPY;
    btn.style.cssText = `position: absolute; top: ${top}; right: ${right}; opacity: 0; transition: opacity 0.2s;`;
    return btn;
  },

  /**
   * Attach copy behavior to a button
   * @param {HTMLButtonElement} btn - The button element
   * @param {Function} getContent - Function that returns content to copy
   */
  attachBehavior(btn, getContent) {
    btn.addEventListener('click', async () => {
      try {
        await navigator.clipboard.writeText(getContent());
        btn.innerHTML = Constants.ICONS.CHECK;
        setTimeout(() => {
          btn.innerHTML = Constants.ICONS.COPY;
        }, 2000);
      } catch (err) {
        console.error('[DailyOS] Copy failed:', err);
      }
    });
  },

  /**
   * Add hover show/hide behavior
   * @param {HTMLElement} wrapper - Container element for hover detection
   * @param {HTMLButtonElement} btn - The button to show/hide
   */
  addHoverBehavior(wrapper, btn) {
    wrapper.addEventListener('mouseenter', () => {
      btn.style.opacity = '1';
    });
    wrapper.addEventListener('mouseleave', () => {
      btn.style.opacity = '0';
    });
  },

  /**
   * Create and attach a complete copy button to a wrapper
   * @param {HTMLElement} wrapper - Container element
   * @param {Function} getContent - Function that returns content to copy
   * @param {Object} options - Button options
   * @returns {HTMLButtonElement}
   */
  attach(wrapper, getContent, options = {}) {
    const btn = this.create(options);
    this.attachBehavior(btn, getContent);
    this.addHoverBehavior(wrapper, btn);
    wrapper.appendChild(btn);
    return btn;
  }
};

// Make available globally
window.CopyButton = CopyButton;
