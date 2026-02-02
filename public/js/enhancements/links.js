/**
 * DailyOS Link Enhancement
 * SPA navigation and external link handling
 *
 * Features:
 * - External links open in new tabs with security attributes
 * - Internal markdown links use Router for SPA navigation
 * - Relative path resolution for .md file links
 *
 * @module enhancements/links
 * @requires Router (optional)
 */

const EnhanceLinks = {
  /**
   * Enhance links for SPA navigation
   * @param {Element} container
   */
  apply(container) {
    const links = container.querySelectorAll('a');
    links.forEach(link => {
      const href = link.getAttribute('href');

      // Skip external links - open in new tab
      if (!href || href.startsWith('http') || href.startsWith('mailto:')) {
        link.target = '_blank';
        link.rel = 'noopener noreferrer';
        return;
      }

      // Handle relative markdown links
      if (href.endsWith('.md')) {
        link.addEventListener('click', (e) => {
          e.preventDefault();
          const path = this.resolveRelativePath(window.location.pathname, href);
          if (window.Router) {
            Router.navigate(`/file?path=${encodeURIComponent(path)}`);
          }
        });
      }

      // Handle internal navigation links
      if (href.startsWith('/')) {
        link.addEventListener('click', (e) => {
          e.preventDefault();
          if (window.Router) {
            Router.navigate(href);
          }
        });
      }
    });
  },

  /**
   * Resolve relative path from current location
   * @param {string} currentPath
   * @param {string} relativePath
   * @returns {string}
   */
  resolveRelativePath(currentPath, relativePath) {
    // Remove leading ./
    relativePath = relativePath.replace(/^\.\//, '');

    // Handle ../ navigation
    const parts = currentPath.split('/').filter(Boolean);
    const relParts = relativePath.split('/');

    for (const part of relParts) {
      if (part === '..') {
        parts.pop();
      } else if (part !== '.') {
        parts.push(part);
      }
    }

    return parts.join('/');
  }
};

// Make available globally
window.EnhanceLinks = EnhanceLinks;
