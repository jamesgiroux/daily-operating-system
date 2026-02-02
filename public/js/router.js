/**
 * DailyOS Client-Side Router
 * Simple hash-based routing for SPA navigation
 */

const Router = {
  routes: {},
  currentRoute: null,

  /**
   * Initialize the router
   */
  init() {
    // Listen for popstate (back/forward)
    window.addEventListener('popstate', () => {
      this.handleRoute(window.location.pathname);
    });

    // Handle initial route
    this.handleRoute(window.location.pathname);
  },

  /**
   * Register a route handler
   */
  on(path, handler) {
    this.routes[path] = handler;
  },

  /**
   * Navigate to a new route
   */
  navigate(path) {
    if (path === this.currentRoute) return;

    window.history.pushState({}, '', path);
    this.handleRoute(path);
  },

  /**
   * Handle route change
   */
  async handleRoute(path) {
    this.currentRoute = path;

    // Strip query string for route matching (but keep full path for handler)
    const pathOnly = path.split('?')[0];

    // Update active nav link
    this.updateActiveNav(pathOnly);

    // Update breadcrumb
    this.updateBreadcrumb(pathOnly);

    // Find matching route handler (use path without query string)
    const handler = this.findHandler(pathOnly);

    if (handler) {
      try {
        await handler(path);
      } catch (error) {
        console.error('Route handler error:', error);
        this.showError('Failed to load content');
      }
    } else {
      this.showError('Page not found');
    }
  },

  /**
   * Find matching route handler (supports patterns)
   */
  findHandler(path) {
    // Exact match first
    if (this.routes[path]) {
      return this.routes[path];
    }

    // Pattern matching
    for (const [pattern, handler] of Object.entries(this.routes)) {
      if (pattern.includes(':')) {
        const regex = this.patternToRegex(pattern);
        if (regex.test(path)) {
          return handler;
        }
      }
    }

    // Default to 404
    return this.routes['*'] || null;
  },

  /**
   * Convert route pattern to regex
   */
  patternToRegex(pattern) {
    const escaped = pattern
      .replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
      .replace(/:[^/]+/g, '([^/]+)');
    return new RegExp(`^${escaped}$`);
  },

  /**
   * Extract params from path
   */
  extractParams(pattern, path) {
    const params = {};
    const patternParts = pattern.split('/');
    const pathParts = path.split('/');

    patternParts.forEach((part, i) => {
      if (part.startsWith(':')) {
        const key = part.slice(1);
        params[key] = decodeURIComponent(pathParts[i] || '');
      }
    });

    return params;
  },

  /**
   * Update active nav link
   */
  updateActiveNav(path) {
    const links = document.querySelectorAll('.sidebar-link');
    links.forEach(link => {
      const href = link.getAttribute('href');
      const isActive = path === href ||
                      (href !== '/' && path.startsWith(href));
      link.classList.toggle('active', isActive);
    });
  },

  /**
   * Update breadcrumb
   */
  updateBreadcrumb(path) {
    const breadcrumb = document.getElementById('breadcrumb');
    if (!breadcrumb) return;

    const parts = path.split('/').filter(Boolean);
    let html = '<a href="/">DailyOS</a>';

    let currentPath = '';
    parts.forEach((part, i) => {
      currentPath += '/' + part;
      const isLast = i === parts.length - 1;

      html += ' <span class="header-breadcrumb-sep">/</span> ';

      if (isLast) {
        html += `<span>${this.formatBreadcrumbPart(part)}</span>`;
      } else {
        html += `<a href="${currentPath}">${this.formatBreadcrumbPart(part)}</a>`;
      }
    });

    breadcrumb.innerHTML = html;
  },

  /**
   * Format breadcrumb part for display
   */
  formatBreadcrumbPart(part) {
    // Remove file extensions
    part = part.replace(/\.md$/, '');

    // Remove number prefixes like 00- 01-
    part = part.replace(/^\d+-/, '');

    // Replace dashes with spaces and capitalize
    return part
      .replace(/-/g, ' ')
      .replace(/\b\w/g, c => c.toUpperCase());
  },

  /**
   * Show error message
   */
  showError(message) {
    const content = document.getElementById('content');
    if (content) {
      content.innerHTML = `
        <div class="empty-state">
          <div class="empty-state-icon">😕</div>
          <h3 class="empty-state-title">${message}</h3>
          <p class="empty-state-description">Try navigating back or using the sidebar.</p>
        </div>
      `;
    }
  },

  /**
   * Show loading state
   */
  showLoading() {
    const content = document.getElementById('content');
    if (content) {
      content.innerHTML = `
        <div class="loading">
          <div class="loading-spinner"></div>
        </div>
      `;
    }
  }
};

// Export for use in other modules
window.Router = Router;
