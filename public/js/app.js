/**
 * DailyOS Main Application
 * Config-driven routing, data fetching, and view rendering
 */

const App = {
  /**
   * Initialize the application
   */
  async init() {
    // Load configuration first
    await Config.load();

    // Build sidebar from config
    this.buildSidebar();

    // Setup routes dynamically
    this.setupRoutes();
    this.setupSearch();
    this.setupMobileMenu();

    // Start router
    Router.init();
  },

  /**
   * Build sidebar navigation from config
   */
  buildSidebar() {
    const nav = document.querySelector('.sidebar-nav');
    if (!nav) return;

    let html = '';

    // Today section
    html += `
      <div class="sidebar-section">
        <div class="sidebar-section-title">Today</div>
    `;

    const todayLinks = Config.getTodaySidebarLinks();
    for (const link of todayLinks) {
      const icon = Icons[link.icon] ? Icons[link.icon]('md') : Icons.file('md');
      html += `
        <a href="${link.route}" class="sidebar-link" data-route="${link.route.replace('/', '')}">
          <span class="sidebar-link-icon">${icon}</span>
          ${link.label}
        </a>
      `;
    }

    html += '</div>';

    // Sections from config
    const sections = Config.getSections();
    if (sections.length > 0) {
      html += `
        <div class="sidebar-section">
          <div class="sidebar-section-title">PARA</div>
      `;

      for (const section of sections) {
        const icon = Icons[section.icon] ? Icons[section.icon]('md') : Icons.folder('md');
        html += `
          <a href="/${section.id}" class="sidebar-link" data-route="${section.id}">
            <span class="sidebar-link-icon">${icon}</span>
            ${section.label}
          </a>
        `;
      }

      html += '</div>';
    }

    // Weekly section
    const weekLinks = Config.getWeekLinks();
    if (weekLinks.length > 0) {
      html += `
        <div class="sidebar-section">
          <div class="sidebar-section-title">Weekly</div>
      `;

      for (const link of weekLinks) {
        const icon = Icons[link.icon] ? Icons[link.icon]('md') : Icons.list('md');
        html += `
          <a href="${link.route}" class="sidebar-link" data-route="${link.route.replace('/', '')}">
            <span class="sidebar-link-icon">${icon}</span>
            ${link.label}
          </a>
        `;
      }

      html += '</div>';
    }

    nav.innerHTML = html;
  },

  /**
   * Setup route handlers dynamically from config
   */
  setupRoutes() {
    // Today routes
    Router.on('/', () => this.loadTodayOverview());
    Router.on('/today', () => this.loadTodayOverview());
    Router.on('/today/actions', () => this.loadTodayFile('actions'));
    Router.on('/today/focus', () => this.loadTodayFile('suggested-focus'));
    Router.on('/today/email', () => this.loadTodayFile('email-summary'));
    Router.on('/today/week-overview', () => this.loadTodayFile('week-00-overview'));
    Router.on('/today/week-meetings', () => this.loadWeekDetailPage('week-01-customer-meetings', 'Customer Meetings'));
    Router.on('/today/week-actions', () => this.loadWeekDetailPage('week-02-actions', 'Actions'));
    Router.on('/today/week-hygiene', () => this.loadWeekDetailPage('week-03-hygiene-alerts', 'Hygiene Alerts'));
    Router.on('/today/week-focus', () => this.loadWeekDetailPage('week-04-focus', 'Focus Areas'));

    // Catch-all for other today files (prep docs, etc.)
    Router.on('/today/:file', (path) => {
      const filename = path.split('/').pop();
      this.loadTodayFile(filename);
    });

    // Dynamic section routes from config
    const sections = Config.getSections();
    for (const section of sections) {
      // List route
      Router.on(`/${section.id}`, () => this.loadSectionList(section));

      // Item route
      Router.on(`/${section.id}/:item`, (path) => this.loadSectionItem(section, path));

      // Item folder route
      Router.on(`/${section.id}/:item/:folder`, (path) => this.loadSectionItemFolder(section, path));
    }

    // File route
    Router.on('/file', () => this.loadFile());

    // 404
    Router.on('*', () => Router.showError('Page not found'));
  },

  /**
   * Setup search functionality
   */
  setupSearch() {
    const searchInput = document.getElementById('searchInput');
    const searchModal = document.getElementById('searchModal');
    const searchResults = document.getElementById('searchResults');
    const closeSearch = document.getElementById('closeSearch');

    let debounceTimer;

    if (searchInput) {
      searchInput.addEventListener('input', (e) => {
        clearTimeout(debounceTimer);
        const query = e.target.value.trim();

        if (query.length < 2) {
          searchModal.classList.add('hidden');
          return;
        }

        debounceTimer = setTimeout(async () => {
          await this.performSearch(query);
        }, 300);
      });

      // Open with keyboard shortcut
      document.addEventListener('keydown', (e) => {
        if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
          e.preventDefault();
          searchInput.focus();
        }
        if (e.key === 'Escape') {
          searchModal.classList.add('hidden');
          searchInput.blur();
        }
      });
    }

    if (closeSearch) {
      closeSearch.addEventListener('click', () => {
        searchModal.classList.add('hidden');
      });
    }

    // Close on click outside
    if (searchModal) {
      searchModal.addEventListener('click', (e) => {
        if (e.target === searchModal) {
          searchModal.classList.add('hidden');
        }
      });
    }
  },

  /**
   * Perform search
   */
  async performSearch(query) {
    const searchModal = document.getElementById('searchModal');
    const searchResults = document.getElementById('searchResults');

    try {
      const response = await fetch(`/api/search?q=${encodeURIComponent(query)}`);
      const data = await response.json();

      if (!data.success) {
        searchResults.innerHTML = '<div class="empty-state"><p>Search failed</p></div>';
        searchModal.classList.remove('hidden');
        return;
      }

      if (data.results.length === 0) {
        searchResults.innerHTML = `
          <div class="empty-state">
            <p>No results for "${query}"</p>
          </div>
        `;
        searchModal.classList.remove('hidden');
        return;
      }

      let html = '';
      for (const result of data.results) {
        const route = this.pathToRoute(result.path);
        html += `
          <a href="${route}" class="search-result-item" data-path="${result.path}">
            <div class="search-result-name">${result.name.replace('.md', '')}</div>
            <div class="search-result-path">${result.path}</div>
            ${result.matchContext ? `<div class="search-result-context">${this.escapeHtml(result.matchContext)}</div>` : ''}
          </a>
        `;
      }

      searchResults.innerHTML = html;
      searchModal.classList.remove('hidden');

      // Add click handlers for navigation
      searchResults.querySelectorAll('.search-result-item').forEach(item => {
        item.addEventListener('click', (e) => {
          e.preventDefault();
          const href = item.getAttribute('href');
          searchModal.classList.add('hidden');
          document.getElementById('searchInput').value = '';
          Router.navigate(href);
        });
      });

    } catch (error) {
      console.error('Search error:', error);
      searchResults.innerHTML = '<div class="empty-state"><p>Search error</p></div>';
      searchModal.classList.remove('hidden');
    }
  },

  /**
   * Convert file path to route
   */
  pathToRoute(path) {
    const todayDir = Config.getToday().directory;

    if (path.startsWith(todayDir + '/')) {
      const file = path.replace(todayDir + '/', '').replace('.md', '');
      return `/today/${file}`;
    }

    // Check each section
    for (const section of Config.getSections()) {
      if (path.startsWith(section.directory + '/')) {
        return '/' + path.toLowerCase().replace('.md', '');
      }
    }

    return `/file?path=${encodeURIComponent(path)}`;
  },

  /**
   * Setup mobile menu toggle
   */
  setupMobileMenu() {
    const menuToggle = document.getElementById('menuToggle');
    const sidebar = document.getElementById('sidebar');

    if (menuToggle && sidebar) {
      menuToggle.addEventListener('click', () => {
        sidebar.classList.toggle('open');
      });

      // Close on navigation
      sidebar.querySelectorAll('.sidebar-link').forEach(link => {
        link.addEventListener('click', () => {
          sidebar.classList.remove('open');
        });
      });
    }
  },

  /**
   * Load today's overview
   */
  async loadTodayOverview() {
    Router.showLoading();

    try {
      const response = await fetch('/api/today');
      const data = await response.json();

      if (!data.success) {
        Router.showError('Failed to load today\'s data');
        return;
      }

      const content = document.getElementById('content');
      const overview = data.files.overview;

      if (overview) {
        content.innerHTML = `
          <div class="markdown-body">
            ${MarkdownUtils.renderFrontmatter(overview.frontmatter)}
            ${overview.html}
          </div>
        `;
        MarkdownUtils.enhance(content);
        this.applyPageAnimations(content);
      } else {
        content.innerHTML = `
          <div class="empty-state">
            <div class="empty-state-icon">${Icons.calendar('xl')}</div>
            <h3 class="empty-state-title">No overview for today</h3>
            <p class="empty-state-description">Run /today to generate today's dashboard</p>
          </div>
        `;
      }

    } catch (error) {
      console.error('Load error:', error);
      Router.showError('Failed to load content');
    }
  },

  /**
   * Load a specific today file
   */
  async loadTodayFile(filename) {
    Router.showLoading();

    try {
      const response = await fetch(`/api/today/${filename}`);
      const data = await response.json();

      if (!data.success) {
        Router.showError('File not found');
        return;
      }

      const content = document.getElementById('content');
      content.innerHTML = `
        <div class="markdown-body">
          ${MarkdownUtils.renderFrontmatter(data.frontmatter)}
          ${data.html}
        </div>
      `;
      MarkdownUtils.enhance(content);
      this.applyPageAnimations(content);

    } catch (error) {
      console.error('Load error:', error);
      Router.showError('Failed to load content');
    }
  },

  /**
   * Load a week detail page (meetings, actions, hygiene, focus)
   */
  async loadWeekDetailPage(filename, title) {
    Router.showLoading();

    try {
      const response = await fetch(`/api/today/${filename}`);
      const data = await response.json();

      if (!data.success) {
        Router.showError('File not found');
        return;
      }

      const content = document.getElementById('content');

      // Icon mapping
      const iconMap = {
        'Customer Meetings': Icons.calendar('lg'),
        'Actions': Icons.checkSquare('lg'),
        'Hygiene Alerts': Icons.bell('lg'),
        'Focus Areas': Icons.target('lg')
      };
      const icon = iconMap[title] || Icons.clipboard('lg');

      let html = `
        <div class="dashboard" data-page-type="week-detail">
          <div class="week-detail-header animate-in-fast" style="animation-delay: 0.1s">
            <div class="card-icon card-icon-lg">${icon}</div>
            <div class="week-detail-info">
              <div class="week-detail-title">${title}</div>
              <a href="/today/week-overview" class="week-back-link">← Back to Week Overview</a>
            </div>
          </div>

          <div class="week-detail-content animate-in-fast markdown-body" style="animation-delay: 0.15s">
            ${data.html}
          </div>
        </div>
      `;

      content.innerHTML = html;
      MarkdownUtils.enhance(content);
      this.applyPageAnimations(content);

      // Add click handler for back link
      content.querySelector('.week-back-link')?.addEventListener('click', (e) => {
        e.preventDefault();
        Router.navigate('/today/week-overview');
      });

    } catch (error) {
      console.error('Load error:', error);
      Router.showError('Failed to load week content');
    }
  },

  /**
   * Load section list (accounts, projects, leadership, etc.)
   */
  async loadSectionList(section) {
    Router.showLoading();

    try {
      const response = await fetch(`/api/${section.id}`);
      const data = await response.json();

      if (!data.success) {
        Router.showError(`Failed to load ${section.label.toLowerCase()}`);
        return;
      }

      const content = document.getElementById('content');

      // Calculate stats
      const multiBUCount = data.items.filter(a => a.isMultiBU).length;
      const standaloneCount = data.items.filter(a => !a.isMultiBU).length;
      const parentCompanies = [...new Set(data.items.filter(a => a.parent).map(a => a.parent))];

      let html = `
        <div class="dashboard">
          <div class="dashboard-header">
            <div>
              <h1 class="dashboard-title">${section.label}</h1>
              <p class="dashboard-subtitle">${data.count} ${section.label.toLowerCase()}</p>
            </div>
          </div>
      `;

      // Stats row (only for sections with multi-BU support)
      if (Config.sectionSupportsMultiBU(section.id) && parentCompanies.length > 0) {
        html += `
          <div class="stats-row">
            <div class="stat-card animate-in" style="animation-delay: 0.1s">
              <div class="stat-label">Total</div>
              <div class="stat-value">${data.count}</div>
            </div>
            <div class="stat-card animate-in" style="animation-delay: 0.15s">
              <div class="stat-label">Multi-BU</div>
              <div class="stat-value">${multiBUCount}</div>
            </div>
            <div class="stat-card animate-in" style="animation-delay: 0.2s">
              <div class="stat-label">Standalone</div>
              <div class="stat-value">${standaloneCount}</div>
            </div>
            <div class="stat-card animate-in" style="animation-delay: 0.25s">
              <div class="stat-label">Parent Companies</div>
              <div class="stat-value">${parentCompanies.length}</div>
            </div>
          </div>
        `;
      }

      // Filter bar
      if (Config.sectionSupportsMultiBU(section.id)) {
        html += `
          <div class="accounts-filter-bar animate-in" style="animation-delay: 0.3s">
            <input type="text" id="itemSearch" class="accounts-search" placeholder="Search ${section.label.toLowerCase()}..." />
            <div class="accounts-filter-buttons">
              <button class="filter-btn active" data-filter="all">All</button>
              <button class="filter-btn" data-filter="standalone">Standalone</button>
              <button class="filter-btn" data-filter="multi-bu">Multi-BU</button>
            </div>
          </div>
        `;
      } else {
        html += `
          <div class="accounts-filter-bar animate-in" style="animation-delay: 0.3s">
            <input type="text" id="itemSearch" class="accounts-search" placeholder="Search ${section.label.toLowerCase()}..." />
          </div>
        `;
      }

      html += `<div class="accounts-grid" id="itemsGrid">`;

      for (const item of data.items) {
        // Get health status if feature enabled
        let healthStatus = 'unknown';
        if (Config.hasFeature('healthStatus') && item.summary) {
          healthStatus = Config.parseHealthStatus(item.summary);
        }

        // Get ring if feature enabled
        let ring = null;
        if (Config.hasFeature('ringBadges') && item.frontmatter?.tags) {
          ring = Config.parseRingFromTags(item.frontmatter.tags);
        }

        // Clean summary
        let displaySummary = '';
        if (item.summary) {
          displaySummary = item.summary
            .replace(/\*\*/g, '')
            .replace(/🟢|🟡|🔴/g, '')
            .slice(0, 100);
          if (item.summary.length > 100) displaySummary += '...';
        }

        const itemType = item.isMultiBU ? 'multi-bu' : 'standalone';
        const parentClass = item.parent ? `parent-${item.parent.toLowerCase()}` : '';

        // Get item icon
        const customIcon = Config.getItemIcon(section.id, item.name);
        const iconFn = customIcon && Icons[customIcon] ? Icons[customIcon] : (item.isMultiBU ? Icons.building : Icons[section.icon] || Icons.folder);

        html += `
          <a href="/${section.id}/${item.path}" class="account-card animate-in-fast ${itemType} ${parentClass}"
             data-name="${item.name.toLowerCase()}"
             data-type="${itemType}"
             style="animation-delay: ${0.35 + data.items.indexOf(item) * 0.02}s">
            <div class="account-card-header">
              <div class="card-icon">${iconFn('md')}</div>
              ${Config.hasFeature('healthStatus') ? `<div class="account-card-health health-dot ${healthStatus}" title="${healthStatus}"></div>` : ''}
            </div>
            <div class="account-card-body">
              <h3 class="account-card-name">${item.name}</h3>
              ${ring ? `<span class="ring-badge ${ring.tag}">${ring.label}</span>` : ''}
              ${displaySummary ? `<p class="account-card-summary">${displaySummary}</p>` : ''}
            </div>
            ${item.parent ? `<div class="account-card-parent">${item.parent}</div>` : ''}
          </a>
        `;
      }

      html += `
          </div>
        </div>
      `;

      content.innerHTML = html;
      this.applyPageAnimations(content);

      // Setup search and filters
      this.setupListFilters(section.id);

      // Add click handlers
      content.querySelectorAll('.account-card').forEach(item => {
        item.addEventListener('click', (e) => {
          e.preventDefault();
          Router.navigate(item.getAttribute('href'));
        });
      });

    } catch (error) {
      console.error('Load error:', error);
      Router.showError(`Failed to load ${section.label.toLowerCase()}`);
    }
  },

  /**
   * Setup search and filter for list views
   */
  setupListFilters(sectionId) {
    const searchInput = document.getElementById('itemSearch');
    const grid = document.getElementById('itemsGrid');
    const filterBtns = document.querySelectorAll('.filter-btn');

    if (searchInput && grid) {
      searchInput.addEventListener('input', (e) => {
        const query = e.target.value.toLowerCase();
        const cards = grid.querySelectorAll('.account-card');
        cards.forEach(card => {
          const name = card.dataset.name;
          card.style.display = name.includes(query) ? '' : 'none';
        });
      });
    }

    filterBtns.forEach(btn => {
      btn.addEventListener('click', () => {
        filterBtns.forEach(b => b.classList.remove('active'));
        btn.classList.add('active');

        const filter = btn.dataset.filter;
        const cards = grid.querySelectorAll('.account-card');
        cards.forEach(card => {
          if (filter === 'all') {
            card.style.display = '';
          } else {
            card.style.display = card.dataset.type === filter ? '' : 'none';
          }
        });
      });
    });
  },

  /**
   * Load section item detail (account, project, etc.)
   */
  async loadSectionItem(section, path) {
    Router.showLoading();

    const itemPath = path.replace(`/${section.id}/`, '');
    const itemName = itemPath.split('/').pop().replace(/-/g, ' ');

    try {
      const response = await fetch(`/api/${section.id}/${encodeURIComponent(itemPath)}`);
      const data = await response.json();

      if (!data.success) {
        Router.showError(`${section.label.slice(0, -1)} not found`);
        return;
      }

      const content = document.getElementById('content');

      // Get metadata from index
      let ring = null, healthStatus = 'unknown', lastUpdated = '';
      if (data.index?.frontmatter) {
        const fm = data.index.frontmatter;
        if (Config.hasFeature('ringBadges') && fm.tags) {
          ring = Config.parseRingFromTags(fm.tags);
        }
        if (fm.date) lastUpdated = fm.date;
      }

      // Parse health from content
      if (Config.hasFeature('healthStatus') && data.index?.content) {
        healthStatus = Config.parseHealthStatus(data.index.content);
      }

      // Build folder icon mapping from config
      const folderIcons = {};
      if (section.subsections) {
        for (const sub of section.subsections) {
          folderIcons[sub.pattern] = Icons[sub.icon] ? Icons[sub.icon]('md') : Icons.folder('md');
        }
      }

      let html = `
        <div class="dashboard">
          <div class="account-detail-header animate-in">
            <div class="account-detail-info">
              <div class="account-detail-breadcrumb">
                <a href="/${section.id}">${section.label}</a> / ${itemPath.includes('/') ? itemPath.split('/')[0] + ' / ' : ''}
              </div>
              <h1 class="dashboard-title">${itemName}</h1>
              <div class="account-detail-meta">
                ${ring ? `<span class="ring-badge ${ring.tag}">${ring.label}</span>` : ''}
                ${Config.hasFeature('healthStatus') ? `<span class="health-badge ${healthStatus}"><span class="health-dot ${healthStatus}"></span> ${healthStatus.charAt(0).toUpperCase() + healthStatus.slice(1)}</span>` : ''}
                ${lastUpdated ? `<span class="last-updated">Updated: ${lastUpdated}</span>` : ''}
              </div>
            </div>
          </div>
      `;

      // Show index content if available
      if (data.index?.html) {
        html += `
          <div class="account-index-card section-card animate-in" style="animation-delay: 0.15s">
            <div class="section-card-header">
              <h3 class="section-card-title">${section.label.slice(0, -1)} Overview</h3>
            </div>
            <div class="section-card-body markdown-body">
              ${data.index.html}
            </div>
          </div>
        `;
      }

      // Show folders
      if (data.folders.length > 0) {
        html += `
          <div class="section-card animate-in" style="animation-delay: 0.2s">
            <div class="section-card-header">
              <h3 class="section-card-title">Folders</h3>
              <span class="section-card-badge">${data.folders.length}</span>
            </div>
            <div class="section-card-body">
              <div class="folder-grid">
        `;

        data.folders.forEach((folder, i) => {
          const icon = folderIcons[folder.name] || Icons.folder('md');
          const displayName = folder.name.replace(/^\d+-/, '').replace(/-/g, ' ');

          html += `
            <a href="/${section.id}/${itemPath}/${folder.name}" class="folder-card animate-in-fast" style="animation-delay: ${0.25 + i * 0.03}s">
              <span class="folder-card-icon">${icon}</span>
              <span class="folder-card-name">${displayName}</span>
            </a>
          `;
        });

        html += `
              </div>
            </div>
          </div>
        `;
      }

      // Show files (excluding index)
      const indexFile = section.indexFile || '00-Index.md';
      const otherFiles = data.files.filter(f => f.name !== indexFile);
      if (otherFiles.length > 0) {
        html += `
          <div class="section-card animate-in" style="animation-delay: 0.3s">
            <div class="section-card-header">
              <h3 class="section-card-title">Files</h3>
              <span class="section-card-badge">${otherFiles.length}</span>
            </div>
            <div class="section-card-body">
              <div class="file-list">
        `;

        otherFiles.forEach((file, i) => {
          const displayName = file.name.replace('.md', '').replace(/^\d+-/, '').replace(/-/g, ' ');
          html += `
            <a href="/file?path=${section.directory}/${itemPath}/${file.name}" class="file-list-item animate-in-fast" style="animation-delay: ${0.35 + i * 0.03}s">
              <span class="file-list-item-icon">${Icons.fileText('sm')}</span>
              <span class="file-list-item-name">${displayName}</span>
            </a>
          `;
        });

        html += `
              </div>
            </div>
          </div>
        `;
      }

      html += '</div>';
      content.innerHTML = html;

      // Enhance markdown in index card
      const indexCard = content.querySelector('.account-index-card .markdown-body');
      if (indexCard) {
        MarkdownUtils.enhanceCheckboxes(indexCard);
        MarkdownUtils.enhanceTables(indexCard);
        MarkdownUtils.enhanceLinks(indexCard);
      }

      this.applyPageAnimations(content);

      // Add click handlers
      content.querySelectorAll('.folder-card, .file-list-item').forEach(item => {
        item.addEventListener('click', (e) => {
          e.preventDefault();
          Router.navigate(item.getAttribute('href'));
        });
      });

    } catch (error) {
      console.error('Load error:', error);
      Router.showError(`Failed to load ${section.label.slice(0, -1).toLowerCase()}`);
    }
  },

  /**
   * Load section item folder contents
   */
  async loadSectionItemFolder(section, path) {
    Router.showLoading();

    const parts = path.replace(`/${section.id}/`, '').split('/');
    const folder = parts.pop();
    const item = parts.join('/');
    const itemName = item.split('/').pop().replace(/-/g, ' ');
    const folderName = folder.replace(/^\d+-/, '').replace(/-/g, ' ');

    // Get folder icon from config
    let folderIcon = Icons.folder('md');
    if (section.subsections) {
      const sub = section.subsections.find(s => s.pattern === folder);
      if (sub && Icons[sub.icon]) {
        folderIcon = Icons[sub.icon]('md');
      }
    }

    try {
      const response = await fetch(`/api/${section.id}/${encodeURIComponent(item)}/folder/${encodeURIComponent(folder)}`);
      const data = await response.json();

      if (!data.success) {
        Router.showError('Folder not found');
        return;
      }

      const content = document.getElementById('content');

      let html = `
        <div class="dashboard">
          <div class="folder-detail-header animate-in">
            <div class="folder-detail-breadcrumb">
              <a href="/${section.id}">${section.label}</a> /
              <a href="/${section.id}/${item}">${itemName}</a> /
            </div>
            <div class="folder-detail-title">
              <span class="card-icon">${folderIcon}</span>
              <h1 class="dashboard-title">${folderName}</h1>
            </div>
            <p class="dashboard-subtitle">${data.files.length} document${data.files.length !== 1 ? 's' : ''}</p>
          </div>
      `;

      if (data.files.length === 0) {
        html += `
          <div class="empty-state animate-in" style="animation-delay: 0.15s">
            <div class="empty-state-icon">${Icons.inbox('xl')}</div>
            <h3 class="empty-state-title">No documents yet</h3>
            <p class="empty-state-description">Documents added to this folder will appear here.</p>
          </div>
        `;
      } else {
        html += `
          <div class="section-card animate-in" style="animation-delay: 0.15s">
            <div class="section-card-header">
              <h3 class="section-card-title">Documents</h3>
              <span class="section-card-badge">${data.files.length}</span>
            </div>
            <div class="section-card-body">
              <div class="document-list">
        `;

        data.files.forEach((file, i) => {
          const displayName = file.name.replace('.md', '').replace(/^\d+-/, '').replace(/-/g, ' ');
          const fileDate = file.date || '';

          // Determine file type icon
          let fileIcon = Icons.fileText('sm');
          const nameLower = file.name.toLowerCase();
          if (nameLower.includes('transcript')) fileIcon = Icons.mic('sm');
          else if (nameLower.includes('meeting') || nameLower.includes('summary')) fileIcon = Icons.edit('sm');
          else if (nameLower.includes('action')) fileIcon = Icons.checkSquare('sm');
          else if (nameLower.includes('decision')) fileIcon = Icons.layers('sm');

          html += `
            <a href="/file?path=${section.directory}/${item}/${folder}/${file.name}" class="document-list-item animate-in-fast" style="animation-delay: ${0.2 + i * 0.03}s">
              <span class="document-list-icon">${fileIcon}</span>
              <div class="document-list-content">
                <span class="document-list-name">${displayName}</span>
                ${fileDate ? `<span class="document-list-date">${fileDate}</span>` : ''}
              </div>
              <svg class="document-list-arrow" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="9 18 15 12 9 6"></polyline>
              </svg>
            </a>
          `;
        });

        html += `
              </div>
            </div>
          </div>
        `;
      }

      html += '</div>';
      content.innerHTML = html;
      this.applyPageAnimations(content);

      // Add click handlers
      content.querySelectorAll('.document-list-item').forEach(item => {
        item.addEventListener('click', (e) => {
          e.preventDefault();
          Router.navigate(item.getAttribute('href'));
        });
      });

    } catch (error) {
      console.error('Load error:', error);
      Router.showError('Failed to load folder');
    }
  },

  /**
   * Load arbitrary file
   */
  async loadFile() {
    Router.showLoading();

    const urlParams = new URLSearchParams(window.location.search);
    const filePath = urlParams.get('path');

    if (!filePath) {
      Router.showError('No file specified');
      return;
    }

    try {
      const response = await fetch(`/api/file?path=${encodeURIComponent(filePath)}`);
      const data = await response.json();

      if (!data.success) {
        Router.showError('File not found');
        return;
      }

      const content = document.getElementById('content');

      // Parse the file path to extract context
      const pathParts = filePath.split('/');
      const fileName = pathParts[pathParts.length - 1];
      const sectionDir = pathParts[0];

      // Find matching section from config
      const section = Config.getSections().find(s => s.directory === sectionDir);
      const todayDir = Config.getToday().directory;

      // Determine back navigation
      let backPath = '/';
      let backLabel = 'Home';
      let sectionIcon = Icons.file('md');

      if (sectionDir === todayDir) {
        backPath = '/';
        backLabel = 'Today';
        sectionIcon = Icons.calendar('md');
      } else if (section) {
        if (pathParts.length > 2) {
          backPath = `/${section.id}/${pathParts[1]}`;
          backLabel = pathParts[1].replace(/-/g, ' ');
        } else {
          backPath = `/${section.id}`;
          backLabel = section.label;
        }
        sectionIcon = Icons[section.icon] ? Icons[section.icon]('md') : Icons.folder('md');
      }

      // Format the document title
      let documentTitle = fileName
        .replace('.md', '')
        .replace(/^\d+-/, '')
        .replace(/-/g, ' ');

      // Extract type/category from frontmatter or path
      let documentType = data.frontmatter?.type || data.frontmatter?.category || '';

      // Document type icons
      const typeIcons = {
        'meeting': Icons.calendar('md'),
        'meeting-prep': Icons.clipboard('md'),
        'prep': Icons.clipboard('md'),
        'transcript': Icons.mic('md'),
        'summary': Icons.fileText('md'),
        'action': Icons.checkSquare('md'),
        'report': Icons.barChart('md'),
        'project': Icons.folder('md'),
        'overview': Icons.eye('md')
      };

      if (!documentType) {
        // Guess type from filename
        if (fileName.includes('prep')) documentType = 'prep';
        else if (fileName.includes('transcript')) documentType = 'transcript';
        else if (fileName.includes('summary')) documentType = 'summary';
        else if (fileName.includes('meeting')) documentType = 'meeting';
        else if (fileName.includes('overview')) documentType = 'overview';
      }

      const docIcon = typeIcons[documentType?.toLowerCase()] || sectionIcon;

      let html = `
        <div class="dashboard">
          <div class="file-detail-header animate-in-fast" style="animation-delay: 0.1s">
            <div class="file-detail-icon">${docIcon}</div>
            <div class="file-detail-info">
              <h1 class="file-detail-name">${documentTitle}</h1>
              <a href="${backPath}" class="file-back-link">← Back to ${backLabel}</a>
            </div>
            ${documentType ? `
              <div class="file-detail-meta">
                <span class="file-type-badge">${documentType}</span>
              </div>
            ` : ''}
          </div>

          <div class="file-content-card animate-in-fast markdown-body" style="animation-delay: 0.15s">
            ${data.html}
          </div>
        </div>
      `;

      content.innerHTML = html;
      MarkdownUtils.enhance(content);
      this.applyPageAnimations(content);

      // Add click handler for back link
      content.querySelector('.file-back-link')?.addEventListener('click', (e) => {
        e.preventDefault();
        Router.navigate(backPath);
      });

    } catch (error) {
      console.error('Load error:', error);
      Router.showError('Failed to load file');
    }
  },

  /**
   * Escape HTML entities
   */
  escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  },

  /**
   * Apply page-level animations for new content
   */
  applyPageAnimations(container) {
    // Animate h1 with main entrance
    const h1 = container.querySelector('h1');
    if (h1) {
      h1.classList.add('animate-in');
    }

    // Animate paragraph after h1
    const firstP = container.querySelector('h1 + p, h1 + .text-secondary');
    if (firstP) {
      firstP.classList.add('animate-in');
      firstP.style.animationDelay = '0.1s';
    }

    // Stagger folder items
    const folderItems = container.querySelectorAll('.folder-item');
    folderItems.forEach((item, i) => {
      item.classList.add('animate-in-fast');
      item.style.animationDelay = `${0.15 + i * 0.04}s`;
    });

    // Stagger file list items
    const fileItems = container.querySelectorAll('.file-list-item');
    fileItems.forEach((item, i) => {
      item.classList.add('animate-in-fast');
      item.style.animationDelay = `${0.15 + i * 0.03}s`;
    });

    // Animate cards
    const cards = container.querySelectorAll('.card, .meeting-prep');
    cards.forEach((card, i) => {
      card.classList.add('animate-in');
      card.style.animationDelay = `${0.2 + i * 0.08}s`;
    });

    // Animate empty states
    const emptyState = container.querySelector('.empty-state');
    if (emptyState) {
      emptyState.classList.add('animate-in-scale');
    }
  }
};

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
  App.init();
});
