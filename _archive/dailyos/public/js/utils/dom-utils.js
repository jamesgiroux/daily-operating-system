/**
 * DailyOS DOM Utilities
 * Helper functions for DOM manipulation
 *
 * Provides utilities for:
 * - Creating elements with attributes
 * - Safe HTML setting with error boundaries
 * - Animation staggering
 * - Element wrapping and replacement
 *
 * @module utils/dom-utils
 */

const DOMUtils = {
  /**
   * Create an element with optional attributes and children
   * @param {string} tag - Element tag name
   * @param {Object} [attrs] - Attributes to set
   * @param {(string|Node)[]} [children] - Child nodes or text content
   * @returns {HTMLElement}
   */
  createElement(tag, attrs = {}, children = []) {
    const el = document.createElement(tag);

    for (const [key, value] of Object.entries(attrs)) {
      if (key === 'className') {
        el.className = value;
      } else if (key === 'style' && typeof value === 'object') {
        Object.assign(el.style, value);
      } else if (key.startsWith('data')) {
        el.dataset[key.slice(4).toLowerCase()] = value;
      } else {
        el.setAttribute(key, value);
      }
    }

    children.forEach(child => {
      if (typeof child === 'string') {
        el.appendChild(document.createTextNode(child));
      } else if (child instanceof Node) {
        el.appendChild(child);
      }
    });

    return el;
  },

  /**
   * Safely set innerHTML with error boundary
   * @param {HTMLElement} el - Element to set content on
   * @param {string} html - HTML content
   */
  setHTML(el, html) {
    try {
      el.innerHTML = html;
    } catch (err) {
      console.error('[DailyOS] Failed to set HTML:', err);
      el.textContent = 'Error rendering content';
    }
  },

  /**
   * Add animation delay styles for staggered animations
   * @param {NodeList|HTMLElement[]} elements - Elements to animate
   * @param {number} [baseDelay=0.1] - Base delay in seconds
   * @param {number} [stagger=0.05] - Stagger increment in seconds
   */
  staggerAnimations(elements, baseDelay = 0.1, stagger = 0.05) {
    elements.forEach((el, i) => {
      el.style.animationDelay = `${baseDelay + i * stagger}s`;
    });
  },

  /**
   * Wrap an element in a container
   * @param {HTMLElement} el - Element to wrap
   * @param {string} wrapperTag - Tag for wrapper element
   * @param {string} [wrapperClass] - Class for wrapper
   * @returns {HTMLElement} The wrapper element
   */
  wrap(el, wrapperTag, wrapperClass = '') {
    const wrapper = document.createElement(wrapperTag);
    if (wrapperClass) wrapper.className = wrapperClass;
    el.parentNode.insertBefore(wrapper, el);
    wrapper.appendChild(el);
    return wrapper;
  },

  /**
   * Replace element with new content
   * @param {HTMLElement} oldEl - Element to replace
   * @param {HTMLElement} newEl - New element
   */
  replace(oldEl, newEl) {
    if (oldEl.parentNode) {
      oldEl.parentNode.replaceChild(newEl, oldEl);
    }
  }
};

// Make available globally
window.DOMUtils = DOMUtils;
