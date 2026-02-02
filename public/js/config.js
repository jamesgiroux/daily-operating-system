/**
 * DailyOS Configuration Loader
 * Fetches and caches configuration from the server
 */

const Config = {
  _config: null,
  _loaded: false,
  _loading: null,

  /**
   * Load configuration from server
   * Returns cached config if already loaded
   */
  async load() {
    if (this._loaded && this._config) {
      return this._config;
    }

    // If already loading, wait for that promise
    if (this._loading) {
      return this._loading;
    }

    this._loading = fetch('/api/config')
      .then(response => response.json())
      .then(data => {
        if (data.success) {
          this._config = data.config;
          this._loaded = true;
          return this._config;
        }
        throw new Error('Failed to load configuration');
      })
      .finally(() => {
        this._loading = null;
      });

    return this._loading;
  },

  /**
   * Get cached config (synchronous, may be null if not loaded)
   */
  get() {
    return this._config;
  },

  /**
   * Get workspace info
   */
  getWorkspace() {
    return this._config?.workspace || { name: 'DailyOS' };
  },

  /**
   * Get all sections
   */
  getSections() {
    return this._config?.sections || [];
  },

  /**
   * Get a specific section by ID
   */
  getSection(sectionId) {
    return this.getSections().find(s => s.id === sectionId);
  },

  /**
   * Get today configuration
   */
  getToday() {
    return this._config?.today || { directory: '_today' };
  },

  /**
   * Get feature flags
   */
  getFeatures() {
    return this._config?.features || {};
  },

  /**
   * Check if a feature is enabled
   */
  hasFeature(featureName) {
    return this.getFeatures()[featureName] === true;
  },

  /**
   * Get display configuration
   */
  getDisplay() {
    return this._config?.display || {};
  },

  /**
   * Get icon for a folder based on section config
   */
  getFolderIcon(sectionId, folderName) {
    const section = this.getSection(sectionId);
    if (!section?.subsections) return null;

    const subsection = section.subsections.find(s => s.pattern === folderName);
    return subsection?.icon || null;
  },

  /**
   * Get icon for an item (project, account, etc.) based on section config
   */
  getItemIcon(sectionId, itemName) {
    const section = this.getSection(sectionId);
    if (!section) return null;

    // Check for custom item icons (e.g., projectIcons)
    const iconMaps = ['projectIcons', 'areaIcons', 'itemIcons'];
    for (const mapName of iconMaps) {
      if (section[mapName] && section[mapName][itemName]) {
        return section[mapName][itemName];
      }
    }

    return null;
  },

  /**
   * Get rings configuration for display
   */
  getRings() {
    return this.getDisplay()?.rings || [];
  },

  /**
   * Get ring by tag
   */
  getRingByTag(tag) {
    return this.getRings().find(r => r.tag === tag);
  },

  /**
   * Get health indicators
   */
  getHealthIndicators() {
    return this.getDisplay()?.healthIndicators || {
      healthy: ['Healthy'],
      attention: ['Attention'],
      critical: ['Critical']
    };
  },

  /**
   * Parse health status from content
   */
  parseHealthStatus(content) {
    if (!content) return 'unknown';

    const indicators = this.getHealthIndicators();

    // Check for emoji indicators first
    if (content.includes('🟢')) return 'healthy';
    if (content.includes('🟡')) return 'attention';
    if (content.includes('🔴')) return 'critical';

    // Check for text indicators
    for (const text of (indicators.healthy || [])) {
      if (content.includes(text)) return 'healthy';
    }
    for (const text of (indicators.attention || [])) {
      if (content.includes(text)) return 'attention';
    }
    for (const text of (indicators.critical || [])) {
      if (content.includes(text)) return 'critical';
    }

    return 'unknown';
  },

  /**
   * Parse ring from tags array
   */
  parseRingFromTags(tags) {
    if (!tags || !Array.isArray(tags)) return null;

    const rings = this.getRings();
    for (const ring of rings) {
      if (tags.includes(ring.tag)) {
        return ring;
      }
    }

    return null;
  },

  /**
   * Get sidebar links for today section
   */
  getTodaySidebarLinks() {
    return this.getToday()?.sidebarLinks || [
      { route: '/', label: 'Overview', icon: 'calendar' }
    ];
  },

  /**
   * Get week links for sidebar
   * Supports both 'weekLinks' and 'weekSidebarLinks' naming
   */
  getWeekLinks() {
    const today = this.getToday();
    return today?.weekLinks || today?.weekSidebarLinks || [];
  },

  /**
   * Get stages for a section (if staged structure)
   */
  getStages(sectionId) {
    const section = this.getSection(sectionId);
    return section?.stages || [];
  },

  /**
   * Check if section supports multi-BU
   */
  sectionSupportsMultiBU(sectionId) {
    const section = this.getSection(sectionId);
    return section?.supportsMultiBU === true && this.hasFeature('multiBusinessUnit');
  }
};

// Export for use in other modules
window.Config = Config;
